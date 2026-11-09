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
    let image_path = parent.join(&sheet.image);
    image_within_directory(&image_path, parent).map_err(|error| match error {
        StoreError::UnsafeImagePath(_) => StoreError::UnsafeImagePath(sheet.image.clone()),
        other => other,
    })?;
    Ok(Loaded { sheet, image_path })
}
/// Reject existing image symlinks that escape the given directory. Directory
/// symlinks are allowed when both paths resolve within the same directory.
/// Missing images remain the image reader's responsibility.
pub fn image_within_directory(image: &Path, dir: &Path) -> Result<(), StoreError> {
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    match image.canonicalize() {
        Ok(actual) if !actual.starts_with(dir.canonicalize()?) => Err(StoreError::UnsafeImagePath(
            image.to_string_lossy().into_owned(),
        )),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
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
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
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
            Ok(file) => break (temp, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    };
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        // Close before rename also supports platforms disallowing open-file moves.
        drop(file);
        fs::rename(&temp, path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rusty-nails-store-{}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
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
        let dir = Temp::new();
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
    fn canonical_roundtrip_and_load() {
        let dir = Temp::new();
        let path = super::super::example::write_example(&dir.0).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.image_path, dir.0.join("example.png"));
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
        let dir = Temp::new();
        let outside = Temp::new();
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
        image_within_directory(&image, &dir.0).unwrap();
        load(&sidecar).unwrap();
        let alias = outside.0.join("directory");
        symlink(&dir.0, &alias).unwrap();
        image_within_directory(&alias.join("example.png"), &alias).unwrap();
        load(&alias.join("example.sprites.json")).unwrap();

        fs::remove_file(&image).unwrap();
        image_within_directory(&image, &dir.0).unwrap();
        load(&sidecar).unwrap();
        symlink("missing.png", &image).unwrap();
        image_within_directory(&image, &dir.0).unwrap();
        load(&sidecar).unwrap();
    }
    #[test]
    fn image_directory_check_uses_current_directory_for_empty_parent() {
        // Do not change the process cwd: other tests can run concurrently.
        let relative = Path::new("Cargo.toml");
        assert_eq!(relative.parent(), Some(Path::new("")));
        assert!(relative.is_file());
        image_within_directory(relative, relative.parent().unwrap()).unwrap();
        image_within_directory(Path::new("missing-image.png"), Path::new("")).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn existing_image_symlink_cannot_escape_sidecar_directory() {
        let dir = Temp::new();
        let outside = Temp::new();
        fs::write(outside.0.join("out.png"), b"image").unwrap();
        std::os::unix::fs::symlink(outside.0.join("out.png"), dir.0.join("example.png")).unwrap();
        let path = dir.0.join("example.sprites.json");
        save(&path, &super::super::example().1).unwrap();
        assert!(matches!(load(&path), Err(StoreError::UnsafeImagePath(_))));
    }
}
