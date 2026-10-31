use crate::{
    error::{AppError, Result},
    models::{Media, Metadata},
    paths::validate_url,
    process::{capture, Engines},
};
use serde_json::Value;

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
pub fn media(v: &Value, fallback: &str) -> Result<Media> {
    if v.get("is_live").and_then(Value::as_bool) == Some(true)
        || v.get("has_drm").and_then(Value::as_bool) == Some(true)
    {
        return Err(AppError::new(
            "restricted",
            "Live streams and DRM-protected content are not supported.",
        ));
    }
    let url = validate_url(
        &text(v, "webpage_url")
            .or_else(|| text(v, "url"))
            .unwrap_or_else(|| fallback.into()),
    )?;
    let thumbnail =
        text(v, "thumbnail").filter(|s| url::Url::parse(s).is_ok_and(|u| u.scheme() == "https"));
    Ok(Media {
        id: text(v, "id")
            .ok_or_else(|| AppError::new("metadata", "This entry has no media identifier."))?,
        title: text(v, "title").unwrap_or_else(|| "Untitled audio".into()),
        url,
        uploader: text(v, "channel").or_else(|| text(v, "uploader")),
        duration: v.get("duration").and_then(Value::as_f64),
        thumbnail,
        source: text(v, "extractor_key")
            .or_else(|| text(v, "extractor"))
            .unwrap_or_else(|| "Website".into()),
        abr: v.get("abr").and_then(Value::as_f64),
    })
}
pub fn parse(value: &Value, url: &str) -> Result<Metadata> {
    let is_playlist = value.get("entries").is_some();
    let items = if let Some(entries) = value.get("entries").and_then(Value::as_array) {
        entries
            .iter()
            .filter(|v| !v.is_null())
            .map(|v| media(v, url))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![media(value, url)?]
    };
    if items.is_empty() {
        return Err(AppError::new(
            "metadata",
            "No available entries were found.",
        ));
    }
    let total = value
        .get("playlist_count")
        .or_else(|| value.get("n_entries"))
        .and_then(Value::as_u64)
        .unwrap_or(items.len() as u64) as usize;
    let truncated = total > items.len() || (is_playlist && items.len() >= 101);
    Ok(Metadata {
        title: text(value, "title").unwrap_or_else(|| items[0].title.clone()),
        is_playlist,
        total,
        truncated,
        items: items.into_iter().take(100).collect(),
    })
}
pub async fn inspect(engines: &Engines, input: &str) -> Result<Metadata> {
    engines.verify()?;
    let url = validate_url(input)?;
    let mut args = engines.base_args();
    args.extend(
        [
            "--skip-download",
            "--dump-single-json",
            "--flat-playlist",
            "--playlist-end",
            "101",
            "--",
            "",
        ]
        .map(str::to_owned),
    );
    *args.last_mut().unwrap() = url.clone();
    parse(
        &serde_json::from_str::<Value>(&capture(&engines.yt, &args, 16 * 1024 * 1024).await?)?,
        &url,
    )
}
pub async fn selected(engines: &Engines, url: &str) -> Result<Value> {
    let mut args = engines.base_args();
    args.extend(
        [
            "--no-playlist",
            "--skip-download",
            "--dump-single-json",
            "--format",
            "bestaudio/best",
            "--",
            url,
        ]
        .map(str::to_owned),
    );
    let value: Value = serde_json::from_str(&capture(&engines.yt, &args, 16 * 1024 * 1024).await?)?;
    media(&value, url)?;
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_unicode() {
        let v = serde_json::json!({"id":"1","title":"เพลงไทย","webpage_url":"https://example.com/a","duration":42,"thumbnail":"https://example.com/a.jpg"});
        let result = parse(&v, "https://example.com").unwrap();
        assert_eq!(result.items[0].title, "เพลงไทย");
        assert!(!result.is_playlist);
    }
    #[test]
    fn playlist_and_drm() {
        let v = serde_json::json!({"title":"List","playlist_count":7,"entries":[{"id":"a","url":"https://example.com/a"},null]});
        let result = parse(&v, "https://example.com").unwrap();
        assert_eq!(result.items.len(), 1);
        assert!(result.truncated);
        assert!(media(&serde_json::json!({"has_drm":true}), "https://example.com").is_err());
    }
    #[test]
    fn malformed() {
        assert!(parse(&serde_json::json!({}), "https://example.com").is_err());
    }
}
