use crate::error::{AppError, Result};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

struct BoundedLines<R> {
    reader: BufReader<R>,
    pending: Vec<u8>,
}
impl<R: tokio::io::AsyncRead + Unpin> BoundedLines<R> {
    fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            pending: Vec::new(),
        }
    }
    async fn next_line(&mut self) -> std::io::Result<Option<String>> {
        loop {
            let available = self.reader.fill_buf().await?;
            if available.is_empty() {
                return if self.pending.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(
                        String::from_utf8_lossy(&std::mem::take(&mut self.pending)).into_owned(),
                    ))
                };
            }
            let count = available
                .iter()
                .position(|v| *v == b'\n')
                .map_or(available.len(), |i| i + 1);
            let complete = available[count - 1] == b'\n';
            if self.pending.len() + count > 64 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Engine output line exceeded the size limit.",
                ));
            }
            self.pending.extend_from_slice(&available[..count]);
            self.reader.consume(count);
            if complete {
                let bytes = std::mem::take(&mut self.pending);
                return Ok(Some(
                    String::from_utf8_lossy(&bytes)
                        .trim_end_matches(['\r', '\n'])
                        .into(),
                ));
            }
        }
    }
}
async fn read_capped(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    limit: usize,
) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(output);
        }
        if output.len() + count > limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Engine response exceeded the size limit.",
            ));
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

#[derive(Clone)]
pub struct Engines {
    pub yt: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub deno: PathBuf,
}
impl Engines {
    pub fn resolve() -> Result<Self> {
        let base = std::env::current_exe()?
            .parent()
            .ok_or_else(|| AppError::new("engine", "Cannot locate application directory."))?
            .to_path_buf();
        let resolve = |name: &str| {
            let packaged = base.join(format!("{name}.exe"));
            if packaged.is_file() || !cfg!(debug_assertions) {
                packaged
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("binaries")
                    .join(format!("{name}-x86_64-pc-windows-msvc.exe"))
            }
        };
        Ok(Self {
            yt: resolve("yt-dlp"),
            ffmpeg: resolve("ffmpeg"),
            ffprobe: resolve("ffprobe"),
            deno: resolve("deno"),
        })
    }
    pub fn verify(&self) -> Result<()> {
        for path in [&self.yt, &self.ffmpeg, &self.ffprobe, &self.deno] {
            if !path.is_file() {
                return Err(AppError::new("missing_engine",format!("Bundled engine {} is missing. Run npm run engines:prepare during development, or reinstall AudioHarbor.",path.file_name().unwrap_or_default().to_string_lossy())));
            }
        }
        Ok(())
    }
    pub fn base_args(&self) -> Vec<String> {
        vec![
            "--ignore-config".into(),
            "--no-cache-dir".into(),
            "--no-colors".into(),
            "--socket-timeout".into(),
            "25".into(),
            "--retries".into(),
            "3".into(),
            "--no-js-runtimes".into(),
            "--js-runtimes".into(),
            format!("deno:{}", self.deno.display()),
            "--ffmpeg-location".into(),
            self.ffmpeg.to_string_lossy().into_owned(),
        ]
    }
}
fn command(exe: &Path, args: &[String]) -> Command {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    cmd
}
// Kill-on-close contains yt-dlp's FFmpeg/Deno descendants, including on cancellation.
#[cfg(windows)]
struct ProcessTree(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
unsafe impl Send for ProcessTree {}
#[cfg(windows)]
impl ProcessTree {
    fn attach(child: &tokio::process::Child) -> Result<Self> {
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(
                    handle,
                    child
                        .raw_handle()
                        .ok_or_else(|| AppError::new("process", "Process handle unavailable."))?
                        as _,
                ) == 0
            {
                CloseHandle(handle);
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(Self(handle))
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(not(windows))]
struct ProcessTree;
#[cfg(not(windows))]
impl ProcessTree {
    fn attach(_child: &tokio::process::Child) -> Result<Self> {
        Ok(Self)
    }
}

pub async fn run_lines(
    exe: &Path,
    args: &[String],
    cancel: CancellationToken,
    limit: Duration,
    mut on_line: impl FnMut(&str) + Send,
) -> Result<()> {
    let mut child = command(exe, args)
        .spawn()
        .map_err(|e| AppError::new("process", format!("Could not launch bundled engine: {e}")))?;
    let tree = ProcessTree::attach(&child)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::new("process", "No engine output stream."))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::new("process", "No engine error stream."))?;
    let mut out = BoundedLines::new(stdout);
    let mut err = BoundedLines::new(stderr);
    let mut out_done = false;
    let mut err_done = false;
    let mut tail = std::collections::VecDeque::new();
    let timeout = tokio::time::sleep(limit);
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            _=cancel.cancelled()=>{drop(tree); let _=child.kill().await; let _=child.wait().await; return Err(AppError::new("cancelled","Download cancelled. Partial data is retained for retry."));},
            _=&mut timeout=>{drop(tree); let _=child.kill().await; let _=child.wait().await; return Err(AppError::new("timeout","The engine timed out. Check your connection and retry."));},
            line=out.next_line(), if !out_done=>match line? { Some(line)=>{if line.len()>16*1024*1024 {return Err(AppError::new("engine","Engine response exceeded the size limit."));} on_line(&line);},None=>out_done=true },
            line=err.next_line(), if !err_done=>match line? {Some(line)=>{ if tail.len()==6 {tail.pop_front();} tail.push_back(crate::error::redact(&line)); },None=>err_done=true},
        }
        if out_done && err_done {
            break;
        }
    }
    let status = tokio::select! { _=cancel.cancelled()=>{drop(tree);let _=child.kill().await;return Err(AppError::new("cancelled","Download cancelled."));}, _=&mut timeout=>{drop(tree);let _=child.kill().await;return Err(AppError::new("timeout","The engine timed out."));}, status=child.wait()=>status? };
    if !status.success() {
        return Err(AppError::engine(&format!(
            "Engine exited with {}. {}",
            status.code().map_or("a signal".into(), |v| v.to_string()),
            tail.into_iter().collect::<Vec<_>>().join(" ")
        )));
    }
    Ok(())
}
pub async fn capture(exe: &Path, args: &[String], limit: usize) -> Result<String> {
    let mut child = command(exe, args).spawn()?;
    let _tree = ProcessTree::attach(&child)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::new("process", "Missing stdout."))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::new("process", "Missing stderr."))?;
    let read = async {
        let (out, err) =
            tokio::try_join!(read_capped(stdout, limit), read_capped(stderr, 64 * 1024))?;
        let status = child.wait().await?;
        if !status.success() {
            return Err(AppError::engine(&String::from_utf8_lossy(&err)));
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    };
    match tokio::time::timeout(Duration::from_secs(90), read).await {
        Ok(result) => result,
        Err(_) => Err(AppError::new(
            "timeout",
            "Metadata request timed out. Check your connection and retry.",
        )),
    }
}
