//! Writing an exported replay clip to disk (V1.6, ADR-0012). The Replay
//! screen records the canvas in the WebView and hands the encoded video
//! here as the raw IPC body — `x-clip-name` carries the file name the
//! frontend built (`src/replay/clip.ts`), and the answer is the absolute
//! path the file landed on, for the toast to name.

use std::path::{Path, PathBuf};

use tauri::ipc::{InvokeBody, Request};
use tauri::Manager;

use crate::perf::timed;

/// The one folder clips go to — inside the user's Videos directory, so a
/// saved clip is findable without a dialog (ADR-0012).
const CLIP_DIR: &str = "ClutchFactor";

/// Header carrying the file name; the body carries the video itself.
const NAME_HEADER: &str = "x-clip-name";

/// A 10-second 1024² recording runs to a few MB. Anything near this ceiling
/// is a bug on the way in, not a clip, and shouldn't reach the disk.
const MAX_CLIP_BYTES: usize = 200 * 1024 * 1024;

/// The name the app will actually write. Characters outside
/// `[A-Za-z0-9._-]` are dropped; a name carrying a path is refused outright
/// rather than flattened (stripping the separators would save somewhere the
/// caller never asked for), and the extension must be one this app records.
pub fn sanitize_clip_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("that clip arrived without a file name".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("a clip name can't carry a path".into());
    }
    let name: String = trimmed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    let (stem, ext) = name
        .rsplit_once('.')
        .ok_or_else(|| "a clip name needs an .mp4 or .webm extension".to_string())?;
    if !matches!(ext.to_ascii_lowercase().as_str(), "mp4" | "webm") {
        return Err(format!("this saves mp4 and webm clips, not {ext}"));
    }
    if stem.is_empty() || stem.chars().all(|c| c == '.') {
        return Err("that clip name is only an extension".into());
    }
    Ok(name)
}

/// `dir/name`, or the first free `dir/name-2`, `dir/name-3`, … — exporting
/// the same moment twice adds a file, it never overwrites one. `exists` is
/// injected so the counting is testable without touching a filesystem.
pub fn unique_path(dir: &Path, name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let candidate = dir.join(name);
    if !exists(&candidate) {
        return candidate;
    }
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    let mut n = 2u32;
    loop {
        let suffixed = if ext.is_empty() {
            format!("{stem}-{n}")
        } else {
            format!("{stem}-{n}.{ext}")
        };
        let candidate = dir.join(suffixed);
        if !exists(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Writes one recorded clip and answers with its absolute path. Video bytes
/// arrive as `InvokeBody::Raw`, so nothing is base64'd on the way across.
#[tauri::command]
pub async fn save_clip(app: tauri::AppHandle, request: Request<'_>) -> Result<String, String> {
    let InvokeBody::Raw(body) = request.body() else {
        return Err("that clip didn't arrive as video data".into());
    };
    if body.len() > MAX_CLIP_BYTES {
        return Err(format!(
            "that clip is {} MB — too large to save, the ceiling is {} MB",
            body.len() / (1024 * 1024),
            MAX_CLIP_BYTES / (1024 * 1024)
        ));
    }
    let name = sanitize_clip_name(
        request
            .headers()
            .get(NAME_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default(),
    )?;
    let dir = app
        .path()
        .video_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| format!("couldn't find a folder to save clips in: {e}"))?
        .join(CLIP_DIR);

    // One copy of the bytes, because the write has to outlive the request.
    let bytes = body.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        timed("save_clip", || {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("couldn't create {}: {e}", dir.display()))?;
            let path = unique_path(&dir, &name, |p| p.exists());
            std::fs::write(&path, &bytes).map_err(|e| format!("couldn't write the clip: {e}"))?;
            Ok(path.to_string_lossy().into_owned())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn accepts_a_well_formed_mp4_name() {
        assert_eq!(
            sanitize_clip_name("mirage-r12-0m34s.mp4").unwrap(),
            "mirage-r12-0m34s.mp4"
        );
    }

    #[test]
    fn accepts_a_webm_name() {
        assert_eq!(
            sanitize_clip_name("dust2-r3-1m03s.webm").unwrap(),
            "dust2-r3-1m03s.webm"
        );
    }

    #[test]
    fn rejects_an_empty_name() {
        assert!(sanitize_clip_name("   ").is_err());
    }

    #[test]
    fn rejects_path_separators_rather_than_stripping_them() {
        assert!(sanitize_clip_name("../secrets/clip.mp4").is_err());
        assert!(sanitize_clip_name("..\\secrets\\clip.mp4").is_err());
    }

    #[test]
    fn drops_characters_outside_the_allowed_alphabet() {
        assert_eq!(
            sanitize_clip_name("mirage r1*.mp4").unwrap(),
            "mirager1.mp4"
        );
    }

    #[test]
    fn rejects_an_extension_that_is_not_mp4_or_webm() {
        assert!(sanitize_clip_name("clip.mov").is_err());
        assert!(sanitize_clip_name("clip.exe").is_err());
    }

    #[test]
    fn rejects_a_name_with_no_extension() {
        assert!(sanitize_clip_name("clip").is_err());
    }

    #[test]
    fn rejects_a_name_that_is_only_an_extension() {
        assert!(sanitize_clip_name(".mp4").is_err());
        assert!(sanitize_clip_name("..mp4").is_err());
    }

    #[test]
    fn uses_the_plain_name_when_nothing_is_there() {
        let dir = Path::new("/clips");
        assert_eq!(
            unique_path(dir, "mirage-r12-0m34s.mp4", |_| false),
            PathBuf::from("/clips/mirage-r12-0m34s.mp4")
        );
    }

    #[test]
    fn suffixes_before_the_extension_when_the_name_is_taken() {
        let dir = Path::new("/clips");
        let taken = |p: &Path| p == Path::new("/clips/mirage-r12-0m34s.mp4");
        assert_eq!(
            unique_path(dir, "mirage-r12-0m34s.mp4", taken),
            PathBuf::from("/clips/mirage-r12-0m34s-2.mp4")
        );
    }

    #[test]
    fn keeps_counting_while_the_suffixed_names_are_taken() {
        let dir = Path::new("/clips");
        let taken = |p: &Path| {
            p == Path::new("/clips/clip.webm")
                || p == Path::new("/clips/clip-2.webm")
                || p == Path::new("/clips/clip-3.webm")
        };
        assert_eq!(
            unique_path(dir, "clip.webm", taken),
            PathBuf::from("/clips/clip-4.webm")
        );
    }
}
