use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{RbError, RbResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePalette {
    pub background: RgbColor,
    pub surface: RgbColor,
    pub foreground: RgbColor,
    pub muted: RgbColor,
    pub accent: RgbColor,
    pub border_default: RgbColor,
    pub border_focused: RgbColor,
    pub status_gap: RgbColor,
    pub status_planned: RgbColor,
    pub status_in_progress: RgbColor,
    pub status_blocked: RgbColor,
    pub status_complete: RgbColor,
    pub md_heading: RgbColor,
    pub md_code: RgbColor,
    pub alert_warn: RgbColor,
    pub alert_error: RgbColor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub palette: ThemePalette,
}

pub const DEFAULT_CRT_AMBER_RON: &str = r#"(
    name: "crt_amber",
    palette: (
        background: (r: 10, g: 10, b: 8),
        surface: (r: 20, g: 18, b: 14),
        foreground: (r: 255, g: 176, b: 0),
        muted: (r: 153, g: 107, b: 20),
        accent: (r: 255, g: 196, b: 61),
        border_default: (r: 120, g: 82, b: 16),
        border_focused: (r: 255, g: 209, b: 102),
        status_gap: (r: 140, g: 100, b: 36),
        status_planned: (r: 214, g: 158, b: 46),
        status_in_progress: (r: 255, g: 196, b: 61),
        status_blocked: (r: 255, g: 82, b: 82),
        status_complete: (r: 64, g: 224, b: 112),
        md_heading: (r: 255, g: 218, b: 121),
        md_code: (r: 94, g: 212, b: 212),
        alert_warn: (r: 255, g: 159, b: 28),
        alert_error: (r: 255, g: 82, b: 82),
    ),
)"#;

pub const MAX_THEME_FILE_BYTES: u64 = 256 * 1024;

pub fn default_theme() -> RbResult<Theme> {
    ron::from_str(DEFAULT_CRT_AMBER_RON).map_err(|source| RbError::RonParse {
        path: PathBuf::from("<embedded:crt_amber>"),
        source,
    })
}

pub fn load_theme_file(path: &Path) -> RbResult<Theme> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|source| RbError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_THEME_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| RbError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > MAX_THEME_FILE_BYTES {
        return Err(RbError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("theme file exceeds maximum size limit of {MAX_THEME_FILE_BYTES} bytes"),
            ),
        });
    }
    let content = String::from_utf8(bytes).map_err(|err| RbError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, err),
    })?;
    ron::from_str(&content).map_err(|source| RbError::RonParse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn validate_theme_name(name: &str) -> RbResult<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains('\0')
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(RbError::Validation(format!("invalid theme name: '{name}'")));
    }
    Ok(())
}

pub fn resolve_theme(theme_name: &str, search_dirs: &[PathBuf]) -> RbResult<Theme> {
    validate_theme_name(theme_name)?;

    let filename = format!("{theme_name}.ron");
    for dir in search_dirs {
        let Ok(dir_meta) = std::fs::symlink_metadata(dir) else {
            continue;
        };
        if dir_meta.file_type().is_symlink() {
            return Err(RbError::PathEscape(dir.to_path_buf()));
        }
        if !dir_meta.is_dir() {
            continue;
        }
        let candidate = dir.join(&filename);
        let canon_candidate = crate::storage::ensure_contained(dir, &candidate)?;
        if !canon_candidate.is_file() {
            continue;
        }
        return load_theme_file(&canon_candidate);
    }
    if theme_name == "crt_amber" {
        return default_theme();
    }
    Err(RbError::NotFound(format!("theme '{theme_name}'")))
}

pub fn list_available_themes(search_dirs: &[PathBuf]) -> Vec<String> {
    let mut themes = BTreeSet::new();
    themes.insert("crt_amber".to_string());

    for dir in search_dirs {
        let Ok(dir_meta) = std::fs::symlink_metadata(dir) else {
            continue;
        };
        if dir_meta.file_type().is_symlink() || !dir_meta.is_dir() {
            continue;
        }
        let Ok(canon_dir) = std::fs::canonicalize(dir) else {
            continue;
        };
        let Ok(entries) = std::fs::read_dir(&canon_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("ron") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if validate_theme_name(stem).is_err() {
                continue;
            }
            let Ok(canon_path) = crate::storage::ensure_contained(&canon_dir, &path) else {
                continue;
            };
            if !canon_path.is_file() {
                continue;
            }
            themes.insert(stem.to_string());
        }
    }

    themes.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_crt_amber_theme_parses_with_green_complete_border() {
        let t = default_theme().unwrap();
        assert_eq!(t.name, "crt_amber");
        assert_eq!(t.palette.background, RgbColor { r: 10, g: 10, b: 8 });
        assert_eq!(
            t.palette.foreground,
            RgbColor {
                r: 255,
                g: 176,
                b: 0
            }
        );
        assert!(t.palette.status_complete.g > t.palette.status_complete.r);
    }

    #[test]
    fn resolve_theme_loads_override_file() {
        let dir = tempfile::tempdir().unwrap();
        let ocean_ron = DEFAULT_CRT_AMBER_RON.replace("\"crt_amber\"", "\"ocean\"");
        std::fs::write(dir.path().join("ocean.ron"), ocean_ron).unwrap();

        let theme = resolve_theme("ocean", &[dir.path().to_path_buf()]).unwrap();
        assert_eq!(theme.name, "ocean");

        assert!(resolve_theme("nonexistent", &[dir.path().to_path_buf()]).is_err());
    }

    #[test]
    fn resolve_theme_rejects_traversal_and_invalid_names() {
        let dir = tempfile::tempdir().unwrap();
        let search_dirs = vec![dir.path().to_path_buf()];

        for bad_name in ["../secret", "a/b", "a\\b", "", "foo.bar", "bad name"] {
            let err = resolve_theme(bad_name, &search_dirs).unwrap_err();
            assert!(
                matches!(err, RbError::Validation(_)),
                "expected Validation for '{bad_name}', got {err:?}"
            );
            assert!(validate_theme_name(bad_name).is_err());
        }

        assert!(validate_theme_name("crt_amber").is_ok());
        assert!(validate_theme_name("Neon-Grid_01").is_ok());
    }

    #[test]
    fn resolve_theme_rejects_invalid_theme_name_with_validation() {
        let dir = tempfile::tempdir().unwrap();
        let search_dirs = vec![dir.path().to_path_buf()];

        let err_traversal = resolve_theme("../evil", &search_dirs).unwrap_err();
        assert!(
            matches!(err_traversal, RbError::Validation(_)),
            "expected Validation for '../evil', got {err_traversal:?}"
        );

        let err_empty = resolve_theme("", &search_dirs).unwrap_err();
        assert!(
            matches!(err_empty, RbError::Validation(_)),
            "expected Validation for '', got {err_empty:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn resolve_theme_rejects_symlink_escape() {
        let search_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();

        let outside_file = outside_dir.path().join("secret.ron");
        let secret_ron = DEFAULT_CRT_AMBER_RON.replace("\"crt_amber\"", "\"secret\"");
        std::fs::write(&outside_file, secret_ron).unwrap();

        let link_path = search_dir.path().join("escape.ron");
        std::os::unix::fs::symlink(&outside_file, &link_path).unwrap();

        let err = resolve_theme("escape", &[search_dir.path().to_path_buf()]).unwrap_err();
        assert!(matches!(err, RbError::PathEscape(_)));

        let available = list_available_themes(&[search_dir.path().to_path_buf()]);
        assert_eq!(available, vec!["crt_amber".to_string()]);
    }

    #[test]
    fn list_available_themes_discovers_deduplicates_and_sorts() {
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let missing_dir = dir_a.path().join("does_not_exist");

        assert_eq!(list_available_themes(&[]), vec!["crt_amber".to_string()]);

        std::fs::write(dir_a.path().join("solarized.ron"), DEFAULT_CRT_AMBER_RON).unwrap();
        std::fs::write(dir_a.path().join("crt_amber.ron"), DEFAULT_CRT_AMBER_RON).unwrap();
        std::fs::write(dir_a.path().join("readme.txt"), "not a theme").unwrap();
        std::fs::write(dir_a.path().join("bad.name.ron"), DEFAULT_CRT_AMBER_RON).unwrap();
        std::fs::write(dir_a.path().join("bad space.ron"), DEFAULT_CRT_AMBER_RON).unwrap();
        std::fs::create_dir(dir_a.path().join("subdir.ron")).unwrap();

        std::fs::write(dir_b.path().join("amber_night.ron"), DEFAULT_CRT_AMBER_RON).unwrap();
        std::fs::write(dir_b.path().join("solarized.ron"), DEFAULT_CRT_AMBER_RON).unwrap();

        let themes = list_available_themes(&[
            missing_dir,
            dir_a.path().to_path_buf(),
            dir_b.path().to_path_buf(),
        ]);
        assert_eq!(
            themes,
            vec![
                "amber_night".to_string(),
                "crt_amber".to_string(),
                "solarized".to_string(),
            ]
        );
    }

    #[test]
    fn theme_load_rejects_oversized_ron_and_symlinked_search_dir() {
        let dir = tempfile::tempdir().unwrap();
        let oversized = dir.path().join("huge.ron");
        let payload = vec![b' '; (MAX_THEME_FILE_BYTES as usize) + 1];
        std::fs::write(&oversized, payload).unwrap();

        let err = load_theme_file(&oversized).unwrap_err();
        match err {
            RbError::Io { source, .. } => {
                assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);
            }
            other => panic!("expected RbError::Io(InvalidData), got {other:?}"),
        }

        #[cfg(unix)]
        {
            let target_dir = tempfile::tempdir().unwrap();
            let ocean_ron = DEFAULT_CRT_AMBER_RON.replace("\"crt_amber\"", "\"ocean\"");
            std::fs::write(target_dir.path().join("ocean.ron"), ocean_ron).unwrap();

            let link_dir = dir.path().join("themes_link");
            std::os::unix::fs::symlink(target_dir.path(), &link_dir).unwrap();

            let err = resolve_theme("ocean", std::slice::from_ref(&link_dir)).unwrap_err();
            assert!(matches!(err, RbError::PathEscape(_)));

            let available = list_available_themes(&[link_dir]);
            assert_eq!(available, vec!["crt_amber".to_string()]);
        }
    }
}
