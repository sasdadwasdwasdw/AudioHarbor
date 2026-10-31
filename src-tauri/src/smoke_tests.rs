//! Opt-in real engine tests. They never run as part of offline unit tests.
use crate::{download, metadata, models::*, process::Engines};

#[tokio::test]
#[ignore = "Requires prepared Windows engines and an authorized public media URL in AUDIOHARBOR_TEST_URL"]
async fn real_metadata_download_and_conversion() {
    let url = std::env::var("AUDIOHARBOR_TEST_URL")
        .expect("Set AUDIOHARBOR_TEST_URL to media you may download");
    let engines = Engines::resolve().unwrap();
    let metadata = metadata::inspect(&engines, &url).await.unwrap();
    assert!(!metadata.is_playlist);
    let temp = tempfile::tempdir().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let output_dir = temp.path().join("เพลงไทย");
    std::fs::create_dir_all(&output_dir).unwrap();
    let mut media = metadata.items[0].clone();
    media.title = "เสียงทดสอบ".into();
    let job = Job {
        id: uuid::Uuid::new_v4().to_string(),
        media,
        status: Status::Downloading,
        format: AudioFormat::Mp3,
        quality: 192,
        output_dir: output_dir.to_string_lossy().into_owned(),
        output_file: None,
        created_at: now.clone(),
        updated_at: now,
        progress: Progress::default(),
        error: None,
    };
    let path = download::execute(
        &engines,
        &job,
        tokio_util::sync::CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert!(std::path::Path::new(&path).is_file());
    assert!(path.contains("เพลงไทย") && path.contains("เสียงทดสอบ"));
    let probe = crate::process::capture(
        &engines.ffprobe,
        &[
            "-v".into(),
            "error".into(),
            "-show_entries".into(),
            "stream=codec_name".into(),
            "-of".into(),
            "json".into(),
            path,
        ],
        1024 * 1024,
    )
    .await
    .unwrap();
    let probe: serde_json::Value = serde_json::from_str(&probe).unwrap();
    assert_eq!(probe["streams"][0]["codec_name"], "mp3");
}
