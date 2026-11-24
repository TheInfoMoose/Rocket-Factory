use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{RbError, RbResult};
use crate::model::{Component, GridConfig, Project, ProjectConfig};

pub const ROCKET_DIR: &str = ".rocket";
pub const PROJECT_FILE: &str = "project.toml";
pub const COMPONENTS_DIR: &str = "components";

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn is_dir_sync_unsupported(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::InvalidInput
            | std::io::ErrorKind::PermissionDenied
            | std::io::ErrorKind::Unsupported
    ) || matches!(err.raw_os_error(), Some(9))
}

fn atomic_write(path: &Path, contents: &str) -> RbResult<()> {
    atomic_write_bytes(path, contents.as_bytes())
}

fn atomic_write_bytes(path: &Path, contents: &[u8]) -> RbResult<()> {
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

    let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = parent.join(format!("{file_name}.tmp.{}.{seq}", std::process::id()));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)
        .map_err(|source| RbError::Io {
            path: tmp_path.clone(),
            source,
        })?;

    let write_result = (|| -> std::io::Result<()> {
        file.write_all(contents)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();
    drop(file);

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

    match File::open(parent) {
        Ok(dir) => {
            if let Err(source) = dir.sync_all() {
                if !is_dir_sync_unsupported(&source) {
                    return Err(RbError::Io {
                        path: parent.to_path_buf(),
                        source,
                    });
                }
            }
        }
        Err(source) => {
            if !is_dir_sync_unsupported(&source) {
                return Err(RbError::Io {
                    path: parent.to_path_buf(),
                    source,
                });
            }
        }
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

    match fs::symlink_metadata(&resolved) {
        Ok(meta) => {
            let canon_target = match fs::canonicalize(&resolved) {
                Ok(target) => target,
                Err(_) if meta.file_type().is_symlink() => {
                    return Err(RbError::PathEscape(candidate.to_path_buf()));
                }
                Err(source) => {
                    return Err(RbError::Io {
                        path: resolved,
                        source,
                    });
                }
            };
            if !canon_target.starts_with(&canon_base) {
                return Err(RbError::PathEscape(candidate.to_path_buf()));
            }
            return Ok(canon_target);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(RbError::Io {
                path: resolved,
                source,
            });
        }
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
    config.grid.validate()?;
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

    let prev_toml = match fs::read(&toml_path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(RbError::Io {
                path: toml_path,
                source,
            });
        }
    };
    let _prev_md_exists = match fs::metadata(&md_path) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(source) => {
            return Err(RbError::Io {
                path: md_path,
                source,
            });
        }
    };

    let serialized = toml::to_string_pretty(component)?;
    atomic_write(&toml_path, &serialized)?;
    if let Err(err) = atomic_write(&md_path, &component.markdown_body) {
        match prev_toml {
            Some(bytes) => {
                let _ = atomic_write_bytes(&toml_path, &bytes);
            }
            None => {
                let _ = fs::remove_file(&toml_path);
            }
        }
        return Err(err);
    }
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

    match fs::symlink_metadata(&rocket_dir) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(RbError::NotFound(project_path.display().to_string()));
        }
        Err(source) => {
            return Err(RbError::Io {
                path: rocket_dir,
                source,
            });
        }
    }
    let safe_rocket_dir = ensure_contained(root, &rocket_dir)?;

    let safe_project_path =
        ensure_contained(&safe_rocket_dir, &safe_rocket_dir.join(PROJECT_FILE))?;
    let project_meta = match fs::metadata(&safe_project_path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(RbError::NotFound(project_path.display().to_string()));
        }
        Err(source) => {
            return Err(RbError::Io {
                path: safe_project_path,
                source,
            });
        }
    };
    if !project_meta.is_file() {
        return Err(RbError::Io {
            path: safe_project_path,
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "project.toml is not a regular file",
            ),
        });
    }

    let project_raw = fs::read_to_string(&safe_project_path).map_err(|source| RbError::Io {
        path: safe_project_path.clone(),
        source,
    })?;

    let config: ProjectConfig =
        toml::from_str(&project_raw).map_err(|source| RbError::TomlParse {
            path: safe_project_path,
            source,
        })?;
    config.grid.validate()?;

    let components_dir = safe_rocket_dir.join(COMPONENTS_DIR);
    match fs::symlink_metadata(&components_dir) {
        Ok(_) => {}
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
    }
    let safe_components_dir = ensure_contained(&safe_rocket_dir, &components_dir)?;
    let entries = fs::read_dir(&safe_components_dir).map_err(|source| RbError::Io {
        path: safe_components_dir.clone(),
        source,
    })?;

    let mut toml_files: Vec<(std::ffi::OsString, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RbError::Io {
            path: safe_components_dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            toml_files.push((entry.file_name(), path));
        }
    }
    toml_files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut components = Vec::with_capacity(toml_files.len());
    let mut seen_ids = HashSet::with_capacity(toml_files.len());
    for (_, path) in toml_files {
        let safe_path = ensure_contained(&safe_components_dir, &path)?;
        let meta = fs::metadata(&safe_path).map_err(|source| RbError::Io {
            path: safe_path.clone(),
            source,
        })?;
        if !meta.is_file() {
            continue;
        }

        let raw = fs::read_to_string(&safe_path).map_err(|source| RbError::Io {
            path: safe_path.clone(),
            source,
        })?;
        let mut component: Component =
            toml::from_str(&raw).map_err(|source| RbError::TomlParse {
                path: safe_path.clone(),
                source,
            })?;

        validate_component_id(&component.id)?;
        if !seen_ids.insert(component.id.clone()) {
            return Err(RbError::AlreadyExists(format!(
                "duplicate component id '{}'",
                component.id
            )));
        }
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if file_stem != component.id {
            return Err(RbError::Validation(format!(
                "component file '{}' has mismatched id '{}'",
                safe_path.display(),
                component.id
            )));
        }
        let md_path = ensure_contained(
            &safe_components_dir,
            &safe_components_dir.join(format!("{}.md", component.id)),
        )?;
        component.markdown_body = match fs::metadata(&md_path) {
            Ok(md_meta) => {
                if !md_meta.is_file() {
                    return Err(RbError::Io {
                        path: md_path,
                        source: std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "component markdown path is not a regular file",
                        ),
                    });
                }
                fs::read_to_string(&md_path).map_err(|source| RbError::Io {
                    path: md_path,
                    source,
                })?
            }
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

    #[cfg(unix)]
    #[test]
    fn storage_load_project_rejects_symlinked_project_and_component_toml() {
        use std::os::unix::fs::symlink;

        let outside = tempfile::tempdir().expect("create outside tempdir");
        let outside_project = outside.path().join("outside_project.toml");
        fs::write(&outside_project, "name = \"outside\"\ndescription = \"\"\n")
            .expect("write outside project");

        // 1. Symlinked .rocket/project.toml pointing outside workspace -> PathEscape
        let ws_proj = tempfile::tempdir().expect("create ws_proj");
        init_workspace(ws_proj.path(), "demo", "Demo").expect("init workspace");
        let proj_file = ws_proj.path().join(ROCKET_DIR).join(PROJECT_FILE);
        fs::remove_file(&proj_file).expect("remove original project.toml");
        symlink(&outside_project, &proj_file).expect("symlink project.toml");
        assert!(matches!(
            load_project(ws_proj.path()),
            Err(RbError::PathEscape(_))
        ));

        // 2. Symlinked .rocket/components/evil.toml pointing outside workspace -> PathEscape
        let ws_comp = tempfile::tempdir().expect("create ws_comp");
        init_workspace(ws_comp.path(), "demo", "Demo").expect("init workspace");
        let outside_comp = outside.path().join("secret.toml");
        let comp = make_component("evil", "");
        fs::write(&outside_comp, toml::to_string_pretty(&comp).unwrap())
            .expect("write outside component");
        let comp_dir = ws_comp.path().join(ROCKET_DIR).join(COMPONENTS_DIR);
        symlink(&outside_comp, comp_dir.join("evil.toml")).expect("symlink evil.toml");
        assert!(matches!(
            load_project(ws_comp.path()),
            Err(RbError::PathEscape(_))
        ));
        fs::remove_file(comp_dir.join("evil.toml")).expect("remove evil.toml");

        // 3. Dangling symlink .rocket/components/ghost.toml -> PathEscape
        symlink(
            outside.path().join("nonexistent.toml"),
            comp_dir.join("ghost.toml"),
        )
        .expect("symlink ghost.toml");
        assert!(matches!(
            load_project(ws_comp.path()),
            Err(RbError::PathEscape(_))
        ));
        fs::remove_file(comp_dir.join("ghost.toml")).expect("remove ghost.toml");

        // 4. In-bounds directory .rocket/components/dir.toml -> skipped cleanly
        fs::create_dir(comp_dir.join("dir.toml")).expect("create dir.toml");
        let loaded = load_project(ws_comp.path()).expect("load project skips dir.toml");
        assert!(loaded.components.is_empty());
    }

    #[test]
    fn load_project_rejects_duplicate_ids_and_stem_mismatch() {
        // 1. Stem mismatch: a.toml with id = "b" -> Validation error
        let dir_mismatch = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir_mismatch.path(), "demo", "Demo").expect("init workspace");
        let comp_dir = dir_mismatch.path().join(ROCKET_DIR).join(COMPONENTS_DIR);
        let comp_b = make_component("b", "");
        fs::write(
            comp_dir.join("a.toml"),
            toml::to_string_pretty(&comp_b).expect("serialize comp_b"),
        )
        .expect("write mismatched a.toml");
        assert!(matches!(
            load_project(dir_mismatch.path()),
            Err(RbError::Validation(_))
        ));

        // 2. Duplicate IDs across files: a.toml and b.toml both with id = "a" -> AlreadyExists error
        let dir_dup = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir_dup.path(), "demo", "Demo").expect("init workspace");
        let comp_dir_dup = dir_dup.path().join(ROCKET_DIR).join(COMPONENTS_DIR);
        let comp_a = make_component("a", "");
        let raw_a = toml::to_string_pretty(&comp_a).expect("serialize comp_a");
        fs::write(comp_dir_dup.join("a.toml"), &raw_a).expect("write a.toml");
        fs::write(comp_dir_dup.join("b.toml"), &raw_a).expect("write b.toml");
        assert!(matches!(
            load_project(dir_dup.path()),
            Err(RbError::AlreadyExists(_))
        ));
    }
}
