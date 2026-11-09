use super::format::SpriteSheet;
use std::{
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(Vec<String>),
    UnsafeImagePath(String),
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Keep user-supplied paths and JSON diagnostics on one status/smoke line.
        let message = match self {
            Self::Io(e) => format!("sprite file IO: {e}"),
            Self::Json(e) => format!("sprites.json: {e}"),
            Self::Invalid(errors) => errors.join("; "),
            Self::UnsafeImagePath(path) => format!("unsafe image path: {path:?}"),
        };
        write!(f, "{}", message.replace(['\n', '\r'], " "))
    }
}
impl std::error::Error for StoreError {}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
pub struct Loaded {
    pub sheet: SpriteSheet,
    pub image_path: PathBuf,
}

pub fn sidecar_for(sheet: &Path) -> PathBuf {
    sheet.with_extension("sprites.json")
}

pub fn image_path_is_safe(rel: &str) -> bool {
    // components() alone normalizes away '.' and empty components on Unix.
    !rel.is_empty()
        && !rel.contains(['\\', '\0', ':'])
        && rel
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(rel)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
pub fn parse(text: &str) -> Result<SpriteSheet, StoreError> {
    let sheet: SpriteSheet = serde_json::from_str(text)?;
    if !image_path_is_safe(&sheet.image) {
        return Err(StoreError::UnsafeImagePath(sheet.image));
    }
    sheet.validate().map_err(StoreError::Invalid)?;
    Ok(sheet)
}
pub fn load(sidecar: &Path) -> Result<Loaded, StoreError> {
    let sheet = parse(&fs::read_to_string(sidecar)?)?;
    let parent = sidecar
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let given = parent.join(&sheet.image);
    let image_path = match image_within_directory(&given, parent) {
        Ok(checked) => checked,
        // Keep missing-image diagnosis with the image reader, as before.
        Err(StoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => given,
        Err(StoreError::UnsafeImagePath(_)) => {
            return Err(StoreError::UnsafeImagePath(sheet.image));
        }
        Err(error) => return Err(error),
    };
    Ok(Loaded { sheet, image_path })
}
/// Reject existing image symlinks that escape the given directory. Directory
/// symlinks are allowed when both paths resolve within the same directory.
/// Return exactly the canonical image path that was checked. Callers must use
/// this path rather than resolve the input again. Missing images return IO errors.
pub fn image_within_directory(image: &Path, dir: &Path) -> Result<PathBuf, StoreError> {
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let actual = image.canonicalize()?;
    if !actual.starts_with(dir.canonicalize()?) {
        return Err(StoreError::UnsafeImagePath(
            image.to_string_lossy().into_owned(),
        ));
    }
    Ok(actual)
}
pub fn to_json(sheet: &SpriteSheet) -> String {
    let mut normalized = sheet.clone();
    normalized.normalize();
    let mut json = serde_json::to_string_pretty(&normalized)
        .expect("SpriteSheet contains JSON serializable fields");
    json.push('\n');
    json
}
pub fn save(sidecar: &Path, sheet: &SpriteSheet) -> Result<(), StoreError> {
    sheet.validate().map_err(StoreError::Invalid)?;
    write_atomic(sidecar, to_json(sheet).as_bytes())
}

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
struct Staged(PathBuf);
impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn stage_bytes(path: &Path, bytes: &[u8]) -> Result<Staged, StoreError> {
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination needs a file name",
        )
    })?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // create_new prevents clobbering a stale temporary file or following a
    // symlink planted at its name. A counter separates writes in this process.
    let (temp, mut file) = loop {
        let mut filename = name.to_os_string();
        filename.push(format!(
            ".tmp-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let temp = parent.join(filename);
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break (Staged(temp), file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    };
    file.write_all(bytes)?;
    file.sync_all()?;
    // Close before publication also supports platforms disallowing open-file moves.
    drop(file);
    Ok(temp)
}
/// Create or replace the destination with complete bytes through one `rename`.
///
/// On success, the bytes were written to a new same-directory temporary file,
/// synced with `sync_all`, closed, and published with `rename`. On local Linux
/// filesystems readers see the old or the new file, never a partial one, and
/// handles already open keep reading the old contents; other platforms and
/// network mounts are not verified. A symlink at the destination is replaced,
/// not followed, and the new file does not keep the old file's permissions.
/// On error, the destination is normally unchanged and the temporary file is
/// removed on a best-effort basis; a network filesystem may report a failed
/// `rename` that took effect.
/// The parent directory is not synced, so after a crash the destination may
/// still show its previous state and a temporary file may remain: crash
/// durability of the published name is not guaranteed on any platform.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let temp = stage_bytes(path, bytes)?;
    fs::rename(&temp.0, path)?;
    Ok(())
}
/// Publish complete, synced bytes without replacing any existing destination,
/// including a dangling symlink. Competing publishers cannot both succeed.
///
/// On success, the bytes were written to a new same-directory temporary file,
/// synced with `sync_all`, closed, and published with `hard_link`; the
/// temporary name is then removed on a best-effort basis.
/// On error, no destination is normally created and the temporary file is
/// removed on a best-effort basis; a network filesystem may report a failed
/// `hard_link` that took effect.
/// The parent directory is not synced, so after a crash the published name may
/// be missing and a temporary file may remain: crash durability of the
/// published name is not guaranteed on any platform.
pub fn publish_new(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let temp = stage_bytes(path, bytes)?;
    fs::hard_link(&temp.0, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn lexical_paths_are_portable_and_strict() {
        for bad in [
            "/abs/x.png",
            "../x.png",
            "./x.png",
            "a\\b.png",
            "",
            "a//b.png",
            "a/./b.png",
            "a/../b.png",
            "a/",
            "C:/x.png",
            "x\0.png",
        ] {
            assert!(!image_path_is_safe(bad), "{bad:?}");
            let mut sheet = super::super::example().1;
            sheet.image = bad.into();
            assert!(matches!(
                parse(&to_json(&sheet)),
                Err(StoreError::UnsafeImagePath(_))
            ));
        }
        for good in ["cave-01-wall.png", "sub/x.png"] {
            assert!(image_path_is_safe(good));
        }
        assert_eq!(
            sidecar_for(Path::new("a/cave-01-wall.png")),
            Path::new("a/cave-01-wall.sprites.json")
        );
    }
    #[test]
    fn atomic_replacement_preserves_old_inode_and_cleans_failures() {
        let dir = crate::test_support::TempDir::new("store");
        let path = dir.0.join("sheet.json");
        fs::write(&path, b"old complete content").unwrap();
        let mut old_handle = fs::File::open(&path).unwrap();
        write_atomic(&path, b"new complete content").unwrap();
        let mut old = String::new();
        old_handle.read_to_string(&mut old).unwrap();
        assert_eq!(
            old, "old complete content",
            "direct truncate/write violates atomic replacement"
        );
        assert_eq!(fs::read(&path).unwrap(), b"new complete content");
        let target_dir = dir.0.join("cannot-replace-directory");
        fs::create_dir(&target_dir).unwrap();
        assert!(matches!(
            write_atomic(&target_dir, b"no"),
            Err(StoreError::Io(_))
        ));
        let missing = dir.0.join("missing/sidecar.json");
        assert!(matches!(
            write_atomic(&missing, b"no"),
            Err(StoreError::Io(_))
        ));
        assert!(!dir.0.join("missing").exists());
        assert!(
            fs::read_dir(&dir.0).unwrap().all(|e| !e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp-"))
        );
    }
    #[test]
    fn publication_is_exclusive_and_cleans_success_and_failure() {
        let dir = crate::test_support::TempDir::new("store");
        let path = dir.0.join("image.png");
        publish_new(&path, b"complete PNG").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"complete PNG");
        assert!(matches!(
            publish_new(&path, b"replacement"),
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(&path).unwrap(), b"complete PNG");
        let directory = dir.0.join("directory.png");
        fs::create_dir(&directory).unwrap();
        assert!(matches!(
            publish_new(&directory, b"replacement"),
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists
        ));
        assert!(directory.is_dir());
        assert!(publish_new(&dir.0.join("missing/image.png"), b"no parent").is_err());
        assert!(!dir.0.join("missing").exists());
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }
    #[cfg(unix)]
    #[test]
    fn publication_refuses_live_and_dangling_symlink_names() {
        let dir = crate::test_support::TempDir::new("store");
        let outside = crate::test_support::TempDir::new("store");
        let target = outside.0.join("target.png");
        fs::write(&target, b"original").unwrap();
        for (name, target) in [
            ("live.png", target.clone()),
            ("dangling.png", outside.0.join("missing.png")),
        ] {
            let link = dir.0.join(name);
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(matches!(
                publish_new(&link, b"replacement"),
                Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists
            ));
            assert_eq!(fs::read_link(&link).unwrap(), target);
        }
        assert_eq!(fs::read(&target).unwrap(), b"original");
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 1);
    }
    #[test]
    fn concurrent_publication_has_one_winner_and_keeps_its_complete_bytes() {
        let dir = crate::test_support::TempDir::new("store");
        let path = dir.0.join("image.png");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let writers: Vec<_> = (0..2)
            .map(|i| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let bytes = vec![i; 16_384];
                    barrier.wait();
                    (bytes.clone(), publish_new(&path, &bytes))
                })
            })
            .collect();
        let results: Vec<_> = writers.into_iter().map(|w| w.join().unwrap()).collect();
        let winners: Vec<_> = results
            .iter()
            .filter(|(_, result)| result.is_ok())
            .collect();
        assert_eq!(winners.len(), 1);
        assert_eq!(fs::read(&path).unwrap(), winners[0].0);
        assert!(results.iter().any(|(_, result)| matches!(
            result,
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists
        )));
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }
    #[test]
    fn canonical_roundtrip_and_load() {
        let dir = crate::test_support::TempDir::new("store");
        let path = super::super::example::write_example(&dir.0).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(
            loaded.image_path,
            dir.0.join("example.png").canonicalize().unwrap()
        );
        assert_eq!(loaded.sheet, super::super::example().1);
        let json = to_json(&loaded.sheet);
        save(&path, &loaded.sheet).unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), json);
        assert_eq!(parse(&json).unwrap(), loaded.sheet);
        assert!(json.ends_with('\n') && !json.ends_with("\n\n"));
    }
    #[cfg(unix)]
    #[test]
    fn image_directory_check_matches_loader_for_symlinks_and_missing_files() {
        use std::os::unix::fs::symlink;
        let dir = crate::test_support::TempDir::new("store");
        let outside = crate::test_support::TempDir::new("store");
        let image = dir.0.join("example.png");
        let target = outside.0.join("out.png");
        fs::write(&target, b"outside image").unwrap();
        symlink(&target, &image).unwrap();
        let sidecar = dir.0.join("example.sprites.json");
        save(&sidecar, &super::super::example().1).unwrap();
        let before = fs::read(&sidecar).unwrap();
        assert!(matches!(
            image_within_directory(&image, &dir.0),
            Err(StoreError::UnsafeImagePath(path)) if path == image.to_string_lossy()
        ));
        assert!(matches!(
            load(&sidecar),
            Err(StoreError::UnsafeImagePath(_))
        ));
        assert_eq!(fs::read(&target).unwrap(), b"outside image");
        assert_eq!(fs::read(&sidecar).unwrap(), before);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 1);

        fs::remove_file(&image).unwrap();
        fs::write(dir.0.join("local.png"), b"local image").unwrap();
        symlink("local.png", &image).unwrap();
        let checked = dir.0.join("local.png").canonicalize().unwrap();
        assert_eq!(image_within_directory(&image, &dir.0).unwrap(), checked);
        assert_eq!(load(&sidecar).unwrap().image_path, checked);
        let alias = outside.0.join("directory");
        symlink(&dir.0, &alias).unwrap();
        assert_eq!(
            image_within_directory(&alias.join("example.png"), &alias).unwrap(),
            checked
        );
        assert_eq!(
            load(&alias.join("example.sprites.json"))
                .unwrap()
                .image_path,
            checked
        );

        fs::remove_file(&image).unwrap();
        assert!(
            matches!(image_within_directory(&image, &dir.0), Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound)
        );
        load(&sidecar).unwrap();
        symlink("missing.png", &image).unwrap();
        assert!(
            matches!(image_within_directory(&image, &dir.0), Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound)
        );
        load(&sidecar).unwrap();
    }
    #[test]
    fn image_directory_check_uses_current_directory_for_empty_parent() {
        // Do not change the process cwd: other tests can run concurrently.
        let relative = Path::new("Cargo.toml");
        assert_eq!(relative.parent(), Some(Path::new("")));
        assert!(relative.is_file());
        assert_eq!(
            image_within_directory(relative, relative.parent().unwrap()).unwrap(),
            relative.canonicalize().unwrap()
        );
        assert!(
            matches!(image_within_directory(Path::new("missing-image.png"), Path::new("")), Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound)
        );
    }
    #[cfg(unix)]
    #[test]
    fn checked_image_paths_keep_the_resolved_target_after_alias_replacement() {
        use std::os::unix::fs::symlink;
        let dir = crate::test_support::TempDir::new("store");
        let outside = crate::test_support::TempDir::new("store");
        let local = dir.0.join("local.png");
        let foreign = outside.0.join("foreign.png");
        fs::write(&local, b"checked local image").unwrap();
        fs::write(&foreign, b"foreign image").unwrap();
        let alias = dir.0.join("example.png");
        symlink(&local, &alias).unwrap();
        let sidecar = dir.0.join("example.sprites.json");
        save(&sidecar, &super::super::example().1).unwrap();
        let checked = image_within_directory(&alias, &dir.0).unwrap();
        let loaded = load(&sidecar).unwrap();
        fs::remove_file(&alias).unwrap();
        symlink(&foreign, &alias).unwrap();
        assert_eq!(checked, local.canonicalize().unwrap());
        assert_eq!(loaded.image_path, checked);
        assert_eq!(fs::read(&checked).unwrap(), b"checked local image");
        assert_eq!(
            fs::read(&loaded.image_path).unwrap(),
            b"checked local image"
        );
        assert!(matches!(
            load(&sidecar),
            Err(StoreError::UnsafeImagePath(_))
        ));
    }
    #[cfg(unix)]
    #[test]
    fn existing_image_symlink_cannot_escape_sidecar_directory() {
        let dir = crate::test_support::TempDir::new("store");
        let outside = crate::test_support::TempDir::new("store");
        fs::write(outside.0.join("out.png"), b"image").unwrap();
        std::os::unix::fs::symlink(outside.0.join("out.png"), dir.0.join("example.png")).unwrap();
        let path = dir.0.join("example.sprites.json");
        save(&path, &super::super::example().1).unwrap();
        assert!(matches!(load(&path), Err(StoreError::UnsafeImagePath(_))));
    }
}
