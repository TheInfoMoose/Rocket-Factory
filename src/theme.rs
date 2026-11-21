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

pub fn resolve_theme(theme_name: &str, search_dirs: &[PathBuf]) -> RfResult<Theme> {
    let filename = format!("{theme_name}.ron");
    for dir in search_dirs {
        let candidate = dir.join(&filename);
        if candidate.is_file() {
            return load_theme_file(&candidate);
        }
    }
    if theme_name == "crt_amber" {
        return default_theme();
    }
    Err(RfError::NotFound(format!("theme '{theme_name}'")))
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
}
