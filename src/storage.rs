use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{RbError, RbResult};
use crate::model::{Component, GridConfig, Project, ProjectConfig};

pub const ROCKET_DIR: &str = ".rocket";
pub const PROJECT_FILE: &str = "project.toml";
pub const COMPONENTS_DIR: &str = "components";

pub fn validate_component_id(id: &str) -> RbResult<()> {
    if id.is_empty()
        || id.len() > 64
        || id.starts_with('-')
        || !id
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
    {
        return Err(RbError::InvalidComponentId(id.to_string()));
    }
    Ok(())
}

fn atomic_write(path: &Path, contents: &str) -> RbResult<()> {
    let parent = path.parent().ok_or_else(|| RbError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent"),
    })?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| RbError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path has no valid filename",
            ),
        })?;

    let tmp_path = parent.join(format!("{file_name}.tmp.{}", std::process::id()));

    let write_result = (|| -> std::io::Result<()> {
        let mut file = File::create(&tmp_path)?;
        file.write_all(contents.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(source) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(RbError::Io {
            path: tmp_path,
            source,
        });
    }

    if let Err(source) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(RbError::Io {
            path: path.to_path_buf(),
            source,
        });
    }

    Ok(())
}

fn ensure_contained(base: &Path, candidate: &Path) -> RbResult<PathBuf> {
    let canon_base = fs::canonicalize(base).map_err(|source| RbError::Io {
        path: base.to_path_buf(),
        source,
    })?;

    let parent = candidate
        .parent()
        .ok_or_else(|| RbError::PathEscape(candidate.to_path_buf()))?;
    let canon_parent = fs::canonicalize(parent).map_err(|source| RbError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    if !canon_parent.starts_with(&canon_base) {
        return Err(RbError::PathEscape(candidate.to_path_buf()));
    }

    let file_name = candidate
        .file_name()
        .ok_or_else(|| RbError::PathEscape(candidate.to_path_buf()))?;
    let resolved = canon_parent.join(file_name);

    if resolved.exists() {
        let canon_target = fs::canonicalize(&resolved).map_err(|source| RbError::Io {
            path: resolved,
            source,
        })?;
        if !canon_target.starts_with(&canon_base) {
            return Err(RbError::PathEscape(candidate.to_path_buf()));
        }
        return Ok(canon_target);
    }

    Ok(resolved)
}

pub fn init_workspace(root: &Path, name: &str, description: &str) -> RbResult<Project> {
    let rocket_dir = root.join(ROCKET_DIR);
    let project_path = rocket_dir.join(PROJECT_FILE);

    if project_path.exists() {
        return Err(RbError::AlreadyExists(project_path.display().to_string()));
    }

    let components_dir = rocket_dir.join(COMPONENTS_DIR);
    fs::create_dir_all(&components_dir).map_err(|source| RbError::Io {
        path: components_dir,
        source,
    })?;

    let config = ProjectConfig {
        name: name.to_string(),
        description: description.to_string(),
        theme: "crt_amber".to_string(),
        active_adapters: Vec::new(),
        grid: GridConfig::default(),
        connections: Vec::new(),
    };

    save_project_config(root, &config)?;

    Ok(Project {
        config,
        components: Vec::new(),
    })
}

pub fn save_project_config(root: &Path, config: &ProjectConfig) -> RbResult<()> {
    let rocket_dir = root.join(ROCKET_DIR);
    let project_path = rocket_dir.join(PROJECT_FILE);
    let safe_path = ensure_contained(&rocket_dir, &project_path)?;
    let serialized = toml::to_string_pretty(config)?;
    atomic_write(&safe_path, &serialized)
}

pub fn save_component(root: &Path, component: &Component) -> RbResult<()> {
    validate_component_id(&component.id)?;

    let components_dir = root.join(ROCKET_DIR).join(COMPONENTS_DIR);
    let toml_path = ensure_contained(
        &components_dir,
        &components_dir.join(format!("{}.toml", component.id)),
    )?;
    let md_path = ensure_contained(
        &components_dir,
        &components_dir.join(format!("{}.md", component.id)),
    )?;

    let serialized = toml::to_string_pretty(component)?;
    atomic_write(&toml_path, &serialized)?;
    atomic_write(&md_path, &component.markdown_body)?;
    Ok(())
}

pub fn delete_component(root: &Path, id: &str) -> RbResult<()> {
    validate_component_id(id)?;

    let components_dir = root.join(ROCKET_DIR).join(COMPONENTS_DIR);
    let toml_path = ensure_contained(&components_dir, &components_dir.join(format!("{id}.toml")))?;
    let md_path = ensure_contained(&components_dir, &components_dir.join(format!("{id}.md")))?;

    match fs::remove_file(&toml_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(RbError::NotFound(id.to_string()));
        }
        Err(source) => {
            return Err(RbError::Io {
                path: toml_path,
                source,
            });
        }
    }

    match fs::remove_file(&md_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(RbError::Io {
                path: md_path,
                source,
            });
        }
    }

    Ok(())
}

pub fn load_project(root: &Path) -> RbResult<Project> {
    let rocket_dir = root.join(ROCKET_DIR);
    let project_path = rocket_dir.join(PROJECT_FILE);

    let project_raw = match fs::read_to_string(&project_path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(RbError::NotFound(project_path.display().to_string()));
        }
        Err(source) => {
            return Err(RbError::Io {
                path: project_path,
                source,
            });
        }
    };

    let config: ProjectConfig =
        toml::from_str(&project_raw).map_err(|source| RbError::TomlParse {
            path: project_path,
            source,
        })?;

    let components_dir = rocket_dir.join(COMPONENTS_DIR);
    let entries = match fs::read_dir(&components_dir) {
        Ok(dir) => dir,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Project {
                config,
                components: Vec::new(),
            });
        }
        Err(source) => {
            return Err(RbError::Io {
                path: components_dir,
                source,
            });
        }
    };

    let mut toml_files: Vec<(std::ffi::OsString, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RbError::Io {
            path: components_dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            toml_files.push((entry.file_name(), path));
        }
    }
    toml_files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut components = Vec::with_capacity(toml_files.len());
    for (_, path) in toml_files {
        let raw = fs::read_to_string(&path).map_err(|source| RbError::Io {
            path: path.clone(),
            source,
        })?;
        let mut component: Component =
            toml::from_str(&raw).map_err(|source| RbError::TomlParse {
                path: path.clone(),
                source,
            })?;

        validate_component_id(&component.id)?;
        let md_path = ensure_contained(
            &components_dir,
            &components_dir.join(format!("{}.md", component.id)),
        )?;
        component.markdown_body = match fs::read_to_string(&md_path) {
            Ok(body) => body,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => {
                return Err(RbError::Io {
                    path: md_path,
                    source,
                });
            }
        };

        components.push(component);
    }

    Ok(Project { config, components })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ComponentKind, GridPos};

    fn make_component(id: &str, markdown_body: &str) -> Component {
        Component {
            id: id.to_string(),
            name: id.to_string(),
            kind: ComponentKind::Custom,
            parent_id: None,
            description: String::new(),
            relative_path: None,
            position: GridPos::default(),
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: markdown_body.to_string(),
        }
    }

    #[test]
    fn storage_init_save_load_roundtrip() {
        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "demo-app", "Demo").expect("init workspace");

        let comp = make_component("view_box", "# View Box\nDetails\n");
        save_component(dir.path(), &comp).expect("save component");

        let loaded = load_project(dir.path()).expect("load project");
        assert_eq!(loaded.config.name, "demo-app");
        assert_eq!(loaded.components.len(), 1);
        assert_eq!(loaded.components[0].id, "view_box");
        assert_eq!(loaded.components[0].markdown_body, "# View Box\nDetails\n");
    }

    #[test]
    fn storage_rejects_invalid_and_traversal_ids() {
        assert!(validate_component_id("../escape").is_err());
        assert!(validate_component_id("UpperCase").is_err());
        assert!(validate_component_id("").is_err());
        assert!(validate_component_id("valid_id-1").is_ok());
    }

    #[test]
    fn storage_delete_component_removes_both_files() {
        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "demo-app", "Demo").expect("init workspace");

        let comp = make_component("stream_1", "# Stream 1\n");
        save_component(dir.path(), &comp).expect("save component");

        delete_component(dir.path(), "stream_1").expect("delete component");
        assert!(load_project(dir.path()).unwrap().components.is_empty());
        assert!(matches!(
            delete_component(dir.path(), "stream_1"),
            Err(RbError::NotFound(_))
        ));
    }
}
