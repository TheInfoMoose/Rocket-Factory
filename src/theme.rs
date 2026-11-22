use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{RfError, RfResult};

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

pub fn default_theme() -> RfResult<Theme> {
    ron::from_str(DEFAULT_CRT_AMBER_RON).map_err(|source| RfError::RonParse {
        path: PathBuf::from("<embedded:crt_amber>"),
        source,
    })
}

pub fn load_theme_file(path: &Path) -> RfResult<Theme> {
    let content = std::fs::read_to_string(path).map_err(|source| RfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    ron::from_str(&content).map_err(|source| RfError::RonParse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn validate_theme_name(name: &str) -> RfResult<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(RfError::PathEscape(PathBuf::from(name)));
    }
    Ok(())
}

pub fn resolve_theme(theme_name: &str, search_dirs: &[PathBuf]) -> RfResult<Theme> {
    validate_theme_name(theme_name)?;

    let filename = format!("{theme_name}.ron");
    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }
        let candidate = dir.join(&filename);
        if candidate.is_file() {
            let canon_dir = std::fs::canonicalize(dir).map_err(|source| RfError::Io {
                path: dir.to_path_buf(),
                source,
            })?;
            let canon_candidate =
                std::fs::canonicalize(&candidate).map_err(|source| RfError::Io {
                    path: candidate.clone(),
                    source,
                })?;
            if !canon_candidate.starts_with(&canon_dir) {
                return Err(RfError::PathEscape(candidate));
            }
            return load_theme_file(&canon_candidate);
        }
    }
    if theme_name == "crt_amber" {
        return default_theme();
    }
    Err(RfError::NotFound(format!("theme '{theme_name}'")))
}

pub fn list_available_themes(search_dirs: &[PathBuf]) -> Vec<String> {
    let mut themes = BTreeSet::new();
    themes.insert("crt_amber".to_string());

    for dir in search_dirs {
        let Ok(canon_dir) = std::fs::canonicalize(dir) else {
            continue;
        };
        if !canon_dir.is_dir() {
            continue;
        }
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
            let Ok(canon_path) = std::fs::canonicalize(&path) else {
                continue;
            };
            if !canon_path.is_file() || !canon_path.starts_with(&canon_dir) {
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
                matches!(err, RfError::PathEscape(_)),
                "expected PathEscape for '{bad_name}', got {err:?}"
            );
            assert!(validate_theme_name(bad_name).is_err());
        }

        assert!(validate_theme_name("crt_amber").is_ok());
        assert!(validate_theme_name("Neon-Grid_01").is_ok());
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
        assert!(matches!(err, RfError::PathEscape(_)));

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
}
