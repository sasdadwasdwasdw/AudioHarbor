use crate::{
    error::{AppError, Result},
    metadata,
    models::{AudioFormat, Job, Progress},
    paths,
    process::{capture, run_lines, Engines},
};
use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub enum Tick {
    Download(Progress),
    Convert(Option<f64>),
}
fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
}
pub fn parse_progress(line: &str) -> Option<Progress> {
    let v: Value = serde_json::from_str(line.strip_prefix("AH_PROGRESS:")?).ok()?;
    let downloaded = number(&v, "downloaded_bytes");
    let total = number(&v, "total_bytes").or_else(|| number(&v, "total_bytes_estimate"));
    Some(Progress {
        percent: total
            .filter(|t| *t > 0.0)
            .and_then(|t| downloaded.map(|d| (d / t * 100.0).clamp(0.0, 100.0))),
        downloaded: downloaded.map(|v| v as u64),
        total: total.map(|v| v as u64),
        speed: number(&v, "speed"),
        eta: number(&v, "eta"),
    })
}
pub fn capped_bitrate(request: u16, source: Option<f64>) -> Result<u16> {
    match source.filter(|v|v.is_finite()&&*v>0.0) {Some(v)=>Ok(request.min(v.floor().clamp(8.0,320.0) as u16)),None=>Err(AppError::new("quality","The source bitrate could not be measured. Choose FLAC or WAV to preserve the decoded source without a bitrate target."))}
}
fn mp3_bitrate(request: u16, source: Option<f64>) -> Result<u16> {
    let cap = capped_bitrate(request, source)?;
    [32,40,48,56,64,80,96,112,128,160,192,224,256,320].into_iter().rev().find(|v|*v<=cap).ok_or_else(||AppError::new("quality","This source bitrate is too low for MP3 at its original sample rate. Choose FLAC or WAV."))
}
pub async fn execute(
    engines: &Engines,
    job: &Job,
    cancel: CancellationToken,
    on_tick: impl Fn(Tick) + Send + Sync,
) -> Result<String> {
    engines.verify()?;
    let dir = paths::staging(&job.output_dir, &job.id)?;
    let info = tokio::select! {_=cancel.cancelled()=>return Err(AppError::new("cancelled","Download cancelled.")),result=metadata::selected(engines,&job.media.url)=>result?};
    let format_id = info
        .get("format_id")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::new("format", "No supported audio stream was found."))?;
    let mut args = engines.base_args();
    args.extend(
        [
            "--no-playlist",
            "--no-simulate",
            "--newline",
            "--progress",
            "--progress-delta",
            "0.3",
            "--progress-template",
            "download:AH_PROGRESS:%(progress)j",
            "--print",
            "after_move:AH_FILE:%(filepath)j",
            "--windows-filenames",
            "--no-overwrites",
            "--continue",
            "--format",
            format_id,
            "--output",
        ]
        .map(str::to_owned),
    );
    args.push(dir.join("source.%(ext)s").to_string_lossy().into_owned());
    args.extend(["--".into(), job.media.url.clone()]);
    let mut source_path = None;
    run_lines(
        &engines.yt,
        &args,
        cancel.clone(),
        Duration::from_secs(24 * 3600),
        |line| {
            if let Some(p) = parse_progress(line) {
                on_tick(Tick::Download(p));
            }
            if let Some(raw) = line.strip_prefix("AH_FILE:") {
                source_path = serde_json::from_str::<String>(raw).ok().map(PathBuf::from);
            }
        },
    )
    .await?;
    let source = paths::contained_file(
        &source_path
            .ok_or_else(|| AppError::new("engine", "The engine did not report its output file."))?,
        &dir,
    )?;
    on_tick(Tick::Convert(None));
    let probe_args = vec![
        "-v".into(),
        "error".into(),
        "-select_streams".into(),
        "a:0".into(),
        "-show_entries".into(),
        "stream=codec_name,bit_rate:format=bit_rate,duration".into(),
        "-of".into(),
        "json".into(),
        source.to_string_lossy().into_owned(),
    ];
    let probe: Value = tokio::select! {_=cancel.cancelled()=>return Err(AppError::new("cancelled","Download cancelled.")),r=capture(&engines.ffprobe,&probe_args,1024*1024)=>serde_json::from_str(&r?)?};
    let stream = probe
        .get("streams")
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .ok_or_else(|| AppError::new("format", "The selected media contains no audio stream."))?;
    let parse_num = |v: &Value, key: &str| {
        v.get(key).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
    };
    let source_rate = parse_num(stream, "bit_rate")
        .or_else(|| parse_num(&probe["format"], "bit_rate"))
        .map(|v| v / 1000.0)
        .or_else(|| number(&info, "abr"));
    let duration = parse_num(&probe["format"], "duration").or(job.media.duration);
    let converted = dir.join(format!("converted.{}", job.format.extension()));
    if converted.exists() {
        paths::contained_file(&converted, &dir)?;
        fs::remove_file(&converted)?;
    }
    let mut ff = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-n".into(),
        "-i".into(),
        source.to_string_lossy().into_owned(),
        "-map".into(),
        "0:a:0".into(),
        "-vn".into(),
        "-map_metadata".into(),
        "-1".into(),
    ];
    let codec = stream
        .get("codec_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    match job.format {
        AudioFormat::Mp3 => {
            ff.extend([
                "-c:a".into(),
                "libmp3lame".into(),
                "-b:a".into(),
                format!("{}k", mp3_bitrate(job.quality, source_rate)?),
            ]);
        }
        AudioFormat::M4a if ["aac", "alac"].contains(&codec) => {
            ff.extend(["-c:a".into(), "copy".into()])
        }
        AudioFormat::M4a => ff.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            format!("{}k", capped_bitrate(192, source_rate)?),
        ]),
        AudioFormat::Flac => ff.extend(["-c:a".into(), "flac".into()]),
        AudioFormat::Wav => ff.extend(["-c:a".into(), "pcm_s24le".into()]),
    }
    ff.extend([
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
        converted.to_string_lossy().into_owned(),
    ]);
    run_lines(
        &engines.ffmpeg,
        &ff,
        cancel.clone(),
        Duration::from_secs(24 * 3600),
        |line| {
            if let Some(micro) = line
                .strip_prefix("out_time_us=")
                .and_then(|s| s.parse::<f64>().ok())
            {
                on_tick(Tick::Convert(
                    duration
                        .filter(|d| *d > 0.0)
                        .map(|d| (micro / 1_000_000.0 / d * 100.0).clamp(0.0, 100.0)),
                ));
            }
        },
    )
    .await?;
    if cancel.is_cancelled() {
        return Err(AppError::new("cancelled", "Download cancelled."));
    }
    let converted = paths::contained_file(&converted, &dir)?;
    let root = paths::validate_directory(&job.output_dir)?;
    let destination = root.join(format!(
        "{} [{}].{}",
        paths::safe_name(&job.media.title),
        job.id,
        job.format.extension()
    ));
    // Exclusive creation is authoritative, even if another process creates a file concurrently.
    tokio::task::spawn_blocking(move || {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        let copy_result = (|| -> Result<()> {
            let mut input = fs::File::open(&converted)?;
            let mut buffer = vec![0u8; 1024 * 1024];
            loop {
                if cancel.is_cancelled() {
                    return Err(AppError::new(
                        "cancelled",
                        "Download cancelled. Partial data is retained for retry.",
                    ));
                }
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count])?;
            }
            output.sync_all()?;
            Ok(())
        })();
        drop(output);
        if let Err(error) = copy_result {
            let _ = fs::remove_file(&destination);
            return Err(error);
        }
        // Stage cleanup is limited to this canonical job directory; retained on failures for retry.
        if let Err(error) = fs::remove_dir_all(&dir) {
            log::warn!("Stage cleanup failed: {:?}", error.kind());
        }
        Ok(destination.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| AppError::new("io", "Could not finalize the output file."))?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structured_progress() {
        let p = parse_progress(
            "AH_PROGRESS:{\"downloaded_bytes\":50,\"total_bytes\":100,\"speed\":500,\"eta\":1}",
        )
        .unwrap();
        assert_eq!(p.percent, Some(50.0));
        assert_eq!(p.speed, Some(500.0));
        assert!(parse_progress("anything else").is_none());
    }
    #[test]
    fn no_upscale() {
        assert_eq!(capped_bitrate(320, Some(127.8)).unwrap(), 127);
        assert_eq!(capped_bitrate(192, Some(500.0)).unwrap(), 192);
        assert!(capped_bitrate(320, None).is_err());
        assert_eq!(mp3_bitrate(320, Some(127.8)).unwrap(), 112);
        assert!(mp3_bitrate(128, Some(16.0)).is_err());
    }
}
