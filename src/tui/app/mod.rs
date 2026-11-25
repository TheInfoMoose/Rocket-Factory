mod actions;
mod grid;
mod state;
mod sync;

pub(crate) use state::SyncRequest;
pub use state::{App, FocusColumn, InputMode, ListFilter};
pub(crate) use sync::{apply_sync_outcome, run_sync_job, SyncOutcome};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::audit::audit_project;
    #[cfg(unix)]
    use crate::audit::compute_card_status;
    #[cfg(unix)]
    use crate::model::CardStatus;
    use crate::model::{
        Component, ComponentKind, Connection, EdgeKind, ExternalTaskRef, FeatureItem, GridConfig,
        GridPos, Project, ProjectConfig, TaskStatus,
    };
    #[cfg(unix)]
    use crate::storage::save_component;
    use crate::storage::{save_project_config, COMPONENTS_DIR, ROCKET_DIR};
    use crate::theme::default_theme;
    use crate::tui::modal::ModalState;

    fn make_component(id: &str) -> Component {
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
            markdown_body: String::new(),
        }
    }

    fn make_app(components: Vec<Component>) -> App {
        let project = Project {
            config: ProjectConfig {
                name: "Test Project".to_string(),
                description: "App state test".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: Vec::new(),
            },
            components,
        };
        let theme = default_theme().expect("default theme");
        App::new(PathBuf::from("/tmp/test-project"), project, theme)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn app_cycles_focus_and_quits_on_q() {
        let mut app = make_app(vec![make_component("comp_a"), make_component("comp_b")]);
        assert_eq!(app.focus, FocusColumn::List);

        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, FocusColumn::Canvas);

        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, FocusColumn::Detail);

        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, FocusColumn::Canvas);

        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn app_gaps_filter_narrows_visible_components() {
        let mut tracked = make_component("tracked");
        tracked.external_tasks.push(ExternalTaskRef {
            external_id: "TASK-1".to_string(),
            title: "Completed task".to_string(),
            status: TaskStatus::Complete,
        });
        let view_box = make_component("view_box");

        let mut app = make_app(vec![tracked, view_box]);
        assert_eq!(app.gap_count(), 1);
        assert_eq!(app.visible_component_indices(), vec![0, 1]);

        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(app.visible_component_indices(), vec![1]);
        assert_eq!(app.selected_component().unwrap().id, "view_box");
    }

    #[test]
    fn app_detail_focus_scrolls_instead_of_changing_selection() {
        let mut app = make_app(vec![make_component("comp_a"), make_component("comp_b")]);
        app.focus = FocusColumn::Detail;

        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('k')));

        assert_eq!(app.detail_scroll, 1);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn app_canvas_directional_selection_moves_to_nearest_in_direction() {
        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 2, row: 0 };
        let mut c = make_component("gamma");
        c.position = GridPos { col: 1, row: 0 };
        let mut d = make_component("delta");
        d.position = GridPos { col: 1, row: 2 };

        let mut app = make_app(vec![a, b, c, d]);
        app.focus = FocusColumn::Canvas;
        app.detail_scroll = 3;

        // Right from (0,0): nearest is gamma at (1,0), not beta at (2,0)
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.selected_component().unwrap().id, "gamma");
        assert_eq!(app.detail_scroll, 0);

        // Down from (1,0): delta at (1,2)
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected_component().unwrap().id, "delta");

        // Down again: no component below, selection unchanged
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.selected_component().unwrap().id, "delta");

        // Left from (1,2): alpha at (0,0)
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.selected_component().unwrap().id, "alpha");

        // Up from (0,0): none above, selection unchanged
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.selected_component().unwrap().id, "alpha");
    }

    #[test]
    fn app_move_card_moves_clamps_blocks_collisions_and_persists() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Move Test", "Move card persistence").expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Toggle MoveCard on
        app.handle_key(key(KeyCode::Char('m')));
        assert_eq!(app.mode, InputMode::MoveCard);

        // Attempt to move alpha right into beta's occupied cell (1, 0)
        app.handle_key(key(KeyCode::Right));
        assert_eq!(
            app.selected_component().unwrap().position,
            GridPos { col: 0, row: 0 }
        );
        assert!(app.status_message.contains("occupied"));

        // Clamp at (0, 0) when moving left/up
        app.handle_key(key(KeyCode::Left));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(
            app.selected_component().unwrap().position,
            GridPos { col: 0, row: 0 }
        );

        // Move alpha down into unoccupied (0, 1)
        app.handle_key(key(KeyCode::Down));
        assert_eq!(
            app.selected_component().unwrap().position,
            GridPos { col: 0, row: 1 }
        );

        // Verify persisted on disk
        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("alpha").unwrap().position,
            GridPos { col: 0, row: 1 }
        );

        // Toggle MoveCard off with 'm'
        app.handle_key(key(KeyCode::Char('m')));
        assert_eq!(app.mode, InputMode::Navigate);
    }

    #[test]
    fn app_connect_mode_adds_and_removes_edge_and_rejects_self_connection() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Connect Test", "Edge toggle persistence")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        app.focus = FocusColumn::Canvas;

        // Self-connection is rejected
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.project.config.connections.is_empty());
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());

        // Esc cancels Connect mode and clears connect_source_id
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());

        // Add edge alpha -> beta
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.selected_component().unwrap().id, "beta");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());
        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].from, "alpha");
        assert_eq!(app.project.config.connections[0].to, "beta");
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DependsOn);

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(reloaded.config.connections.len(), 1);

        // Toggle same edge alpha -> beta removes it
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.selected_component().unwrap().id, "alpha");
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));

        assert!(app.project.config.connections.is_empty());
        let reloaded_after = load_project(dir.path()).expect("reload project");
        assert!(reloaded_after.config.connections.is_empty());
    }

    fn type_keys(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
    }

    #[test]
    fn app_new_component_modal_scaffolds_persists_selects_and_rejects_duplicate() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Modal Test", "New component modal test")
            .expect("init workspace");

        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        render_app(&mut app, 140, 40);

        // Open New Component modal with 'n'
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());

        // Type ID, tab to Name, type Name, tab to Archetype, cycle right to Interface, submit
        type_keys(&mut app, "telemetry_bus");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Telemetry Bus");
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));

        // Modal closes back to Navigate, component is added, scaffolded, selected, and persisted
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.components.len(), 2);
        let selected = app.selected_component().expect("new component selected");
        assert_eq!(selected.id, "telemetry_bus");
        assert_eq!(selected.name, "Telemetry Bus");
        assert_eq!(selected.kind, ComponentKind::Interface);
        assert_eq!(selected.parent_id, None);
        assert_eq!(selected.position, GridPos { col: 2, row: 0 });
        assert!(selected.features.is_empty());

        let reloaded = load_project(dir.path()).expect("reload project");
        assert!(reloaded.component("telemetry_bus").is_some());

        // Duplicate ID is rejected cleanly and keeps modal open with error
        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "telemetry_bus");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Duplicate");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        assert_eq!(app.project.components.len(), 2);
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("already exists"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }

        // Esc cancels back to Navigate
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
    }

    #[test]
    fn app_edit_component_and_connection_modals_update_and_persist() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Edit Modal Test", "Edit component and edge")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // 'E' with no outgoing connection sets a helpful status message
        app.handle_key(key(KeyCode::Char('E')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert!(app.status_message.contains("No outgoing connection"));

        // Edit selected component ('alpha') with 'e' and add a feature
        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Tab)); // archetype
        app.handle_key(key(KeyCode::Tab)); // description
        type_keys(&mut app, "Core alpha service");
        app.handle_key(key(KeyCode::Tab)); // path
        type_keys(&mut app, "src/alpha.rs");
        app.handle_key(key(KeyCode::Tab)); // tags
        type_keys(&mut app, "core, api");
        app.handle_key(key(KeyCode::Tab)); // add feature
        type_keys(&mut app, "Expose health endpoint");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        let alpha = app.project.component("alpha").expect("alpha exists");
        assert_eq!(alpha.description, "Core alpha service");
        assert_eq!(alpha.relative_path.as_deref(), Some("src/alpha.rs"));
        assert_eq!(alpha.tags, vec!["core", "api"]);
        assert_eq!(alpha.features.len(), 1);
        assert_eq!(alpha.features[0].title, "Expose health endpoint");

        // Add an outgoing connection and edit it with 'E'
        app.project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        app.handle_key(key(KeyCode::Char('E')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Right)); // DependsOn -> DataFlow
        app.handle_key(key(KeyCode::Tab)); // label
        type_keys(&mut app, "events");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DataFlow);
        assert_eq!(
            app.project.config.connections[0].label.as_deref(),
            Some("events")
        );

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(reloaded.component("alpha").unwrap().features.len(), 1);
        assert_eq!(reloaded.config.connections[0].kind, EdgeKind::DataFlow);
        assert_eq!(
            reloaded.config.connections[0].label.as_deref(),
            Some("events")
        );
    }

    #[test]
    fn app_v_key_queues_editor_and_reloads_markdown_from_disk() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Editor Test", "Markdown reload test").expect("init workspace");

        let mut comp = make_component("alpha");
        comp.markdown_body = "# Alpha\nInitial spec\n".to_string();
        save_component(dir.path(), &comp).expect("save alpha");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        assert!(app.pending_editor_component.is_none());

        let md_path = dir
            .path()
            .join(ROCKET_DIR)
            .join(COMPONENTS_DIR)
            .join("alpha.md");
        let updated_md = "# Alpha\nUpdated spec from external editor\n";
        std::fs::write(&md_path, updated_md).expect("write updated markdown");

        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.pending_editor_component.as_deref(), Some("alpha"));

        let comp_id = app
            .pending_editor_component
            .take()
            .expect("pending editor component id");
        app.reload_component_markdown(&comp_id)
            .expect("reload component markdown");

        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .markdown_body,
            updated_md
        );
        assert!(app.status_message.contains("Reloaded"));
    }

    #[test]
    fn app_mutations_rollback_in_memory_state_on_disk_save_failure() {
        use crate::storage::{init_workspace, load_project, save_component, PROJECT_FILE};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Rollback Test", "Transactional save failure")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        a.description = "Original alpha".to_string();
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: Some("initial".to_string()),
        });
        save_project_config(dir.path(), &project.config).expect("save initial connections");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Break component saves by replacing .rocket/components with a regular file
        let components_path = dir.path().join(ROCKET_DIR).join(COMPONENTS_DIR);
        std::fs::remove_dir_all(&components_path).expect("remove components dir");
        std::fs::write(&components_path, "not a directory").expect("create blocking file");

        // 1. move_selected_card rolls back position on save failure
        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(
            app.project.component("alpha").unwrap().position,
            GridPos { col: 0, row: 0 }
        );
        assert!(app.status_message.contains("Failed to save"));
        app.handle_key(key(KeyCode::Char('m'))); // exit MoveCard

        // 2. UpdateComponent rolls back fields and features on save failure
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Tab)); // archetype
        app.handle_key(key(KeyCode::Tab)); // description
        type_keys(&mut app, " mutated");
        app.handle_key(key(KeyCode::Tab)); // path
        app.handle_key(key(KeyCode::Tab)); // tags
        app.handle_key(key(KeyCode::Tab)); // add feature
        type_keys(&mut app, "Unpersisted feature");
        app.handle_key(key(KeyCode::Enter));

        let alpha = app.project.component("alpha").unwrap();
        assert_eq!(alpha.description, "Original alpha");
        assert!(alpha.features.is_empty());
        assert!(app.status_message.contains("Failed to save"));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::EditComponent(ref form)) = app.modal {
            assert!(form.error.is_some());
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save"));
        } else {
            panic!("expected EditComponent modal to remain open on save failure");
        }
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());

        // Break project.toml saves by replacing .rocket/project.toml with a directory
        let project_file = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        std::fs::remove_file(&project_file).expect("remove project.toml");
        std::fs::create_dir(&project_file).expect("replace project.toml with dir");

        // 3. confirm_connect rolls back connection removal on save failure
        app.focus = FocusColumn::Canvas;
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].from, "alpha");
        assert_eq!(app.project.config.connections[0].to, "beta");
        assert!(app.status_message.contains("Failed to save connections"));

        // 4. UpdateConnection rolls back kind/label on save failure and keeps modal open
        app.handle_key(key(KeyCode::Left)); // back to alpha
        app.handle_key(key(KeyCode::Char('E')));
        app.handle_key(key(KeyCode::Right)); // DependsOn -> DataFlow
        app.handle_key(key(KeyCode::Tab)); // label
        type_keys(&mut app, "_mut");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DependsOn);
        assert_eq!(
            app.project.config.connections[0].label.as_deref(),
            Some("initial")
        );
        assert!(app.status_message.contains("Failed to save connections"));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::EditConnection(ref form)) = app.modal {
            assert!(form.error.is_some());
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save connections"));
        } else {
            panic!("expected EditConnection modal to remain open on save failure");
        }
    }

    #[test]
    fn app_create_component_rejects_when_grid_is_full() {
        let mut comp = make_component("only_card");
        comp.position = GridPos { col: 0, row: 0 };
        let mut app = make_app(vec![comp]);
        app.project.config.grid = GridConfig { cols: 1, rows: 1 };

        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "second_card");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Second Card");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        assert_eq!(app.project.components.len(), 1);
        assert_eq!(app.project.components[0].id, "only_card");
        assert_eq!(
            app.project.components[0].position,
            GridPos { col: 0, row: 0 }
        );
        assert!(app.status_message.contains("full"));
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form.error.as_deref().unwrap_or("").contains("full"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }
    }

    #[test]
    fn app_v_key_clears_connect_mode_before_queueing_editor() {
        let mut app = make_app(vec![make_component("alpha"), make_component("beta")]);
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));

        // Action key 'v' is ignored while in Connect mode
        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        assert!(app.pending_editor_component.is_none());

        // Returning to Navigate via Esc allows 'v' to queue $EDITOR
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());

        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());
        assert_eq!(app.pending_editor_component.as_deref(), Some("alpha"));
    }

    #[test]
    fn app_sync_keys_queue_expected_requests() {
        let mut app = make_app(vec![make_component("alpha"), make_component("beta")]);
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);

        // Sync key 'r' is ignored while in Connect mode
        app.handle_key(key(KeyCode::Char('r')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.pending_sync, None);

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());

        app.handle_key(key(KeyCode::Char('r')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());
        assert_eq!(app.pending_sync, Some(SyncRequest::Pull));
        assert!(app.status_message.contains("Syncing"));

        app.pending_sync = None;
        app.handle_key(key(KeyCode::Char('S')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.pending_sync, Some(SyncRequest::Pull));

        app.pending_sync = None;
        app.handle_key(key(KeyCode::Char('p')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.pending_sync, Some(SyncRequest::PushSelected));
        assert!(app.status_message.contains("Pushing"));

        let mut empty_app = make_app(Vec::new());
        empty_app.handle_key(key(KeyCode::Char('p')));
        assert_eq!(empty_app.pending_sync, None);
        assert_eq!(empty_app.status_message, "No component selected to push");
    }

    #[cfg(unix)]
    #[test]
    fn sync_pull_updates_external_tasks_flips_status_and_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut project = crate::storage::init_workspace(dir.path(), "Sync Pull", "Live pull test")
            .expect("init workspace");
        project.config.active_adapters = vec!["mock_tracker".to_string()];
        crate::storage::save_project_config(dir.path(), &project.config)
            .expect("save project config");

        let mut comp = make_component("auth_svc");
        comp.features.push(FeatureItem {
            id: "auth_svc_f1".to_string(),
            title: "Login endpoint".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save initial component");
        project.components.push(comp);

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.pull_status_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"components\":[{\"component_id\":\"auth_svc\",\"tasks\":[{\"external_id\":\"TASK-101\",\"title\":\"Login endpoint\",\"status\":\"complete\"}]}]}"']
"#;
        std::fs::write(&adapters_path, adapters_toml).expect("write adapters.toml");

        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        assert_eq!(
            compute_card_status(app.project.component("auth_svc").expect("comp")),
            CardStatus::Planned
        );

        let summary = app.sync_pull(&adapters_path).expect("sync_pull succeeds");
        assert!(summary.contains("1 component(s) updated"));
        assert!(summary.contains("1 task(s) synced"));
        assert!(summary.contains("1 feature(s) linked"));
        assert_eq!(app.status_message, summary);

        let updated = app.project.component("auth_svc").expect("updated comp");
        assert_eq!(updated.external_tasks.len(), 1);
        assert_eq!(updated.external_tasks[0].external_id, "TASK-101");
        assert_eq!(updated.external_tasks[0].status, TaskStatus::Complete);
        assert_eq!(compute_card_status(updated), CardStatus::Complete);
        assert!(updated.features[0].pushed);
        assert_eq!(updated.features[0].external_id.as_deref(), Some("TASK-101"));

        let reloaded = crate::storage::load_project(dir.path()).expect("reload project");
        let disk_comp = reloaded.component("auth_svc").expect("disk comp");
        assert_eq!(disk_comp.external_tasks.len(), 1);
        assert_eq!(disk_comp.external_tasks[0].external_id, "TASK-101");
        assert_eq!(compute_card_status(disk_comp), CardStatus::Complete);
    }

    #[cfg(unix)]
    #[test]
    fn sync_push_selected_marks_features_pushed_and_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut project = crate::storage::init_workspace(dir.path(), "Sync Push", "Live push test")
            .expect("init workspace");
        project.config.active_adapters = vec!["mock_tracker".to_string()];
        crate::storage::save_project_config(dir.path(), &project.config)
            .expect("save project config");

        let mut comp = make_component("billing_api");
        comp.features.push(FeatureItem {
            id: "billing_api_f1".to_string(),
            title: "Invoice webhook".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save initial component");
        project.components.push(comp);

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.push_component_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"external_id\":\"COMP-1\"}"']

[adapters.push_feature_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"external_id\":\"FEAT-42\"}"']
"#;
        std::fs::write(&adapters_path, adapters_toml).expect("write adapters.toml");

        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        let summary = app
            .sync_push_selected(&adapters_path)
            .expect("sync_push_selected succeeds");
        assert!(summary.contains("1 component(s)"));
        assert!(summary.contains("1 feature(s)"));
        assert_eq!(app.status_message, summary);

        let updated = app.project.component("billing_api").expect("updated comp");
        assert!(updated.features[0].pushed);
        assert_eq!(updated.features[0].external_id.as_deref(), Some("FEAT-42"));

        let reloaded = crate::storage::load_project(dir.path()).expect("reload project");
        let disk_comp = reloaded.component("billing_api").expect("disk comp");
        assert!(disk_comp.features[0].pushed);
        assert_eq!(
            disk_comp.features[0].external_id.as_deref(),
            Some("FEAT-42")
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_push_selected_pushes_unpushed_features_to_all_active_adapters() {
        use crate::adapter::{AdapterConfig, AdaptersFile, CommandSpec, ExportFormat};

        let dir = tempfile::tempdir().expect("tempdir");
        let log_a = dir.path().join("tracker_a.log");
        let log_b = dir.path().join("tracker_b.log");

        let mut project =
            crate::storage::init_workspace(dir.path(), "Multi Push", "Multi-adapter TUI push test")
                .expect("init workspace");
        project.config.active_adapters = vec!["tracker_a".to_string(), "tracker_b".to_string()];
        crate::storage::save_project_config(dir.path(), &project.config)
            .expect("save project config");

        let mut comp = make_component("billing_api");
        comp.features.push(FeatureItem {
            id: "billing_api_f1".to_string(),
            title: "Invoice webhook".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save initial component");
        project.components.push(comp);

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_file = AdaptersFile {
            adapters: vec![
                AdapterConfig {
                    name: "tracker_a".to_string(),
                    export_format: ExportFormat::Json,
                    push_component_cmd: None,
                    push_feature_cmd: Some(CommandSpec {
                        program: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!(
                                "cat >> '{}' && printf '{{\"external_id\":\"A-42\"}}'",
                                log_a.display()
                            ),
                        ],
                    }),
                    pull_status_cmd: None,
                },
                AdapterConfig {
                    name: "tracker_b".to_string(),
                    export_format: ExportFormat::Json,
                    push_component_cmd: None,
                    push_feature_cmd: Some(CommandSpec {
                        program: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!(
                                "cat >> '{}' && printf '{{\"external_id\":\"B-99\"}}'",
                                log_b.display()
                            ),
                        ],
                    }),
                    pull_status_cmd: None,
                },
            ],
        };
        std::fs::write(
            &adapters_path,
            toml::to_string_pretty(&adapters_file).expect("serialize adapters.toml"),
        )
        .expect("write adapters.toml");

        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        let summary = app
            .sync_push_selected(&adapters_path)
            .expect("sync_push_selected succeeds");
        assert!(summary.contains("2 feature(s) across 2 adapter(s)"));
        assert_eq!(app.status_message, summary);

        let payload_a = std::fs::read_to_string(&log_a).expect("read tracker_a log");
        let payload_b = std::fs::read_to_string(&log_b).expect("read tracker_b log");
        assert!(payload_a.contains("billing_api_f1"));
        assert!(payload_b.contains("billing_api_f1"));

        let updated = app.project.component("billing_api").expect("updated comp");
        assert!(updated.features[0].pushed);
        assert_eq!(updated.features[0].external_id.as_deref(), Some("A-42"));

        let reloaded = crate::storage::load_project(dir.path()).expect("reload project");
        let disk_comp = reloaded.component("billing_api").expect("disk comp");
        assert!(disk_comp.features[0].pushed);
        assert_eq!(disk_comp.features[0].external_id.as_deref(), Some("A-42"));
    }

    #[test]
    fn app_theme_switcher_applies_persists_and_rolls_back_on_error() {
        use crate::storage::{init_workspace, load_project, PROJECT_FILE};
        use crate::theme::DEFAULT_CRT_AMBER_RON;

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Theme Test", "Theme switcher persistence")
            .expect("init workspace");

        let themes_dir = dir.path().join("custom_themes");
        std::fs::create_dir_all(&themes_dir).expect("create themes dir");
        let amber_alt_ron = DEFAULT_CRT_AMBER_RON.replace("\"crt_amber\"", "\"amber_alt\"");
        std::fs::write(themes_dir.join("amber_alt.ron"), amber_alt_ron)
            .expect("write amber_alt.ron");
        std::fs::write(themes_dir.join("broken_theme.ron"), "not valid ron (")
            .expect("write broken_theme.ron");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        app.theme_dirs = vec![themes_dir];

        // Open '?' help overlay and close it with '?'
        app.handle_key(key(KeyCode::Char('?')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.modal, Some(ModalState::Help));
        app.handle_key(key(KeyCode::Char('?')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());

        // Open 't' theme switcher: sorted list is ["amber_alt", "broken_theme", "crt_amber"]
        // Pre-selected index should be 2 ("crt_amber")
        app.handle_key(key(KeyCode::Char('t')));
        assert_eq!(app.mode, InputMode::Modal);
        let Some(ModalState::ThemeSwitcher(ref form)) = app.modal else {
            panic!("expected ThemeSwitcher modal");
        };
        assert_eq!(form.selected_theme(), Some("crt_amber"));

        // Move Up to "broken_theme" and press Enter -> stays open with error
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.theme.name, "crt_amber");
        assert_eq!(app.project.config.theme, "crt_amber");
        let Some(ModalState::ThemeSwitcher(ref form)) = app.modal else {
            panic!("expected ThemeSwitcher modal to remain open on parse error");
        };
        assert!(form.error.is_some());

        // Move Up to "amber_alt" and press Enter -> applies and persists
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.theme.name, "amber_alt");
        assert_eq!(app.project.config.theme, "amber_alt");
        assert_eq!(app.status_message, "Theme switched to amber_alt");

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(reloaded.config.theme, "amber_alt");

        // Break project.toml to verify rollback on save failure
        let project_file = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        std::fs::remove_file(&project_file).expect("remove project.toml");
        std::fs::create_dir(&project_file).expect("replace project.toml with dir");

        app.handle_key(key(KeyCode::Char('t')));
        // Currently on "amber_alt" (index 0); move Up wraps to "crt_amber" (index 2)
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.theme.name, "amber_alt");
        assert_eq!(app.project.config.theme, "amber_alt");
        assert!(app.status_message.contains("Failed to save theme"));
    }

    #[test]
    fn app_delete_component_and_connection_cleanup_and_cancel() {
        use crate::audit::FindingKind;
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Delete Test", "Delete component and connection")
            .expect("init workspace");

        let mut alpha = make_component("alpha");
        alpha.position = GridPos { col: 0, row: 0 };
        let mut beta = make_component("beta");
        beta.position = GridPos { col: 1, row: 0 };
        let mut gamma = make_component("gamma");
        gamma.position = GridPos { col: 2, row: 0 };
        save_component(dir.path(), &alpha).expect("save alpha");
        save_component(dir.path(), &beta).expect("save beta");
        save_component(dir.path(), &gamma).expect("save gamma");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections = vec![
            Connection {
                from: "alpha".to_string(),
                to: "beta".to_string(),
                kind: EdgeKind::DependsOn,
                label: None,
            },
            Connection {
                from: "gamma".to_string(),
                to: "beta".to_string(),
                kind: EdgeKind::DataFlow,
                label: Some("feed".to_string()),
            },
            Connection {
                from: "beta".to_string(),
                to: "alpha".to_string(),
                kind: EdgeKind::Invokes,
                label: None,
            },
        ];
        save_project_config(dir.path(), &project.config).expect("save config");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Selected is alpha (index 0). 'D' presents all incident edges (outgoing alpha -> beta first, then incoming beta -> alpha); cancel with Esc.
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(
            app.modal,
            Some(ModalState::confirm_delete_connections(vec![
                Connection {
                    from: "alpha".to_string(),
                    to: "beta".to_string(),
                    kind: EdgeKind::DependsOn,
                    label: None,
                },
                Connection {
                    from: "beta".to_string(),
                    to: "alpha".to_string(),
                    kind: EdgeKind::Invokes,
                    label: None,
                },
            ]))
        );
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections.len(), 3);

        // Confirm deleting outgoing alpha -> beta with 'y'
        app.handle_key(key(KeyCode::Char('D')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections.len(), 2);
        assert!(!app
            .project
            .config
            .connections
            .iter()
            .any(|c| c.from == "alpha" && c.to == "beta"));

        // Now alpha has no outgoing connection; 'D' falls back to incoming beta -> alpha
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(
            app.modal,
            Some(ModalState::confirm_delete_connections(vec![Connection {
                from: "beta".to_string(),
                to: "alpha".to_string(),
                kind: EdgeKind::Invokes,
                label: None,
            }]))
        );
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].from, "gamma");
        assert_eq!(app.project.config.connections[0].to, "beta");

        // Alpha now has no connections -> 'D' reports status and stays in Navigate
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(
            app.status_message,
            "No connection on selected component to delete"
        );

        // Select beta (index 1), press 'd' -> shows 1 affected connection (gamma -> beta), cancel with 'n'
        app.selected_index = 1;
        app.detail_scroll = 4;
        app.pending_editor_component = Some("beta".to_string());
        app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(
            app.modal,
            Some(ModalState::confirm_delete_component("beta", "beta", 1))
        );
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.project.component("beta").is_some());

        // Delete beta with 'd' + 'y' -> removes component and incident edge gamma -> beta cleanly
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert!(app.project.component("beta").is_none());
        assert!(app.project.config.connections.is_empty());
        assert!(app.pending_editor_component.is_none());
        assert_eq!(app.detail_scroll, 0);
        assert_eq!(app.status_message, "Deleted component beta");
        assert!(!app
            .audit_findings
            .iter()
            .any(|f| f.kind == FindingKind::DanglingConnection));

        let reloaded = load_project(dir.path()).expect("reload project");
        assert!(reloaded.component("beta").is_none());
        assert!(reloaded.config.connections.is_empty());
        assert!(!audit_project(&reloaded)
            .iter()
            .any(|f| f.kind == FindingKind::DanglingConnection));
    }

    #[test]
    fn app_delete_rolls_back_on_disk_failure_and_keeps_modal_open() {
        use crate::storage::{init_workspace, load_project, save_component, PROJECT_FILE};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(
            dir.path(),
            "Delete Rollback",
            "Transactional delete failure",
        )
        .expect("init workspace");

        let alpha = make_component("alpha");
        let beta = make_component("beta");
        save_component(dir.path(), &alpha).expect("save alpha");
        save_component(dir.path(), &beta).expect("save beta");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        save_project_config(dir.path(), &project.config).expect("save config");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // 1. Delete alpha when its file on disk is already missing -> delete_component fails,
        //    connections are restored on disk and in memory, modal stays open with form.error.
        let alpha_toml = dir
            .path()
            .join(ROCKET_DIR)
            .join(COMPONENTS_DIR)
            .join("alpha.toml");
        std::fs::remove_file(&alpha_toml).expect("remove alpha.toml");

        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.project.component("alpha").is_some());
        assert_eq!(app.project.config.connections.len(), 1);
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open on delete_component error");
        }
        let reloaded = load_project(dir.path()).expect("reload after restore");
        assert_eq!(reloaded.config.connections.len(), 1);

        // Close modal
        app.handle_key(key(KeyCode::Esc));

        // 2. Break project.toml by replacing it with a directory -> DeleteConnection fails and rolls back
        let project_file = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        std::fs::remove_file(&project_file).expect("remove project.toml");
        std::fs::create_dir(&project_file).expect("replace project.toml with dir");

        app.handle_key(key(KeyCode::Char('D')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.project.config.connections.len(), 1);
        assert!(app.status_message.contains("Failed to save connections"));
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open on save failure");
        }

        // 3. DeleteComponent also fails cleanly when saving updated connections fails
        app.handle_key(key(KeyCode::Esc));
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.project.component("alpha").is_some());
        assert_eq!(app.project.config.connections.len(), 1);
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open on config save failure");
        }
    }

    #[test]
    fn app_noop_delete_in_connect_mode_preserves_connect_source() {
        // 1. In InputMode::Connect with connect_source_id = Some("alpha"), pressing 'D' or 'd'
        //    is ignored and keeps Connect mode and connect_source_id untouched.
        let mut app = make_app(vec![make_component("alpha"), make_component("beta")]);
        app.mode = InputMode::Connect;
        app.connect_source_id = Some("alpha".to_string());

        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        assert!(app.modal.is_none());

        app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        assert!(app.modal.is_none());

        // 2. With an empty component list in InputMode::Connect and connect_source_id = Some("alpha"),
        //    pressing 'd' keeps Connect mode and connect_source_id untouched.
        let mut empty_app = make_app(vec![]);
        empty_app.mode = InputMode::Connect;
        empty_app.connect_source_id = Some("alpha".to_string());

        empty_app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(empty_app.mode, InputMode::Connect);
        assert_eq!(empty_app.connect_source_id.as_deref(), Some("alpha"));
        assert!(empty_app.modal.is_none());

        // 3. Returning to Navigate via Esc allows 'd' to open ConfirmDelete modal.
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());

        app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.connect_source_id.is_none());
        assert!(matches!(app.modal, Some(ModalState::ConfirmDelete(_))));
    }

    #[test]
    fn sub_component_expand_collapse_hides_and_shows_children() {
        use crate::storage::init_workspace;

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Sub Expand Test", "Sub-component expand test")
            .expect("init workspace");

        let mut parent = make_component("parent_svc");
        parent.position = GridPos { col: 0, row: 0 };

        let mut child = make_component("child_mod");
        child.parent_id = Some("parent_svc".to_string());
        child.position = GridPos { col: 1, row: 0 };

        let mut sibling = make_component("sibling_svc");
        sibling.position = GridPos { col: 2, row: 0 };

        let mut orphan = make_component("orphan_mod");
        orphan.parent_id = Some("missing_parent".to_string());
        orphan.position = GridPos { col: 3, row: 0 };

        let mut app = make_app(vec![parent, child, sibling, orphan]);
        app.root = dir.path().to_path_buf();

        assert!(app.has_sub_components("parent_svc"));
        assert!(!app.has_sub_components("sibling_svc"));
        assert!(!app.is_expanded("parent_svc"));

        // Initially collapsed: child_mod is hidden, dangling orphan_mod is treated as top-level
        let visible_ids = |a: &App| -> Vec<String> {
            a.visible_component_indices()
                .into_iter()
                .map(|i| a.project.components[i].id.clone())
                .collect()
        };
        assert_eq!(
            visible_ids(&app),
            vec!["parent_svc", "sibling_svc", "orphan_mod"]
        );

        // Expand parent_svc via Space -> child_mod appears immediately after parent_svc
        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("parent_svc"));
        assert!(app.status_message.contains("Expanded"));
        assert_eq!(
            visible_ids(&app),
            vec!["parent_svc", "child_mod", "sibling_svc", "orphan_mod"]
        );

        // Collapse parent_svc via Enter -> child_mod hidden again
        app.handle_key(key(KeyCode::Enter));
        assert!(!app.is_expanded("parent_svc"));
        assert!(app.status_message.contains("Collapsed"));
        assert_eq!(
            visible_ids(&app),
            vec!["parent_svc", "sibling_svc", "orphan_mod"]
        );

        // Create a new sub-component via 'a' -> auto-expands parent and places near parent
        app.handle_key(key(KeyCode::Char('a')));
        assert_eq!(app.mode, InputMode::Modal);
        type_keys(&mut app, "child_two");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Child Two");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.is_expanded("parent_svc"));
        let child_two = app
            .project
            .component("child_two")
            .expect("child_two created");
        assert_eq!(child_two.parent_id.as_deref(), Some("parent_svc"));
        assert_eq!(
            visible_ids(&app),
            vec![
                "parent_svc",
                "child_mod",
                "child_two",
                "sibling_svc",
                "orphan_mod"
            ]
        );
    }

    #[test]
    fn expand_shifts_colliding_card_without_overwriting() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Collision Test", "Expand collision shift")
            .expect("init workspace");

        let mut parent = make_component("parent_svc");
        parent.position = GridPos { col: 0, row: 0 };

        let mut child = make_component("child_mod");
        child.parent_id = Some("parent_svc".to_string());
        child.position = GridPos { col: 1, row: 0 };

        let mut blocker = make_component("blocker_svc");
        blocker.position = GridPos { col: 1, row: 0 };

        save_component(dir.path(), &parent).expect("save parent");
        save_component(dir.path(), &child).expect("save child");
        save_component(dir.path(), &blocker).expect("save blocker");

        let mut project = load_project(dir.path()).expect("load project");
        // Tight 2x1 grid forces grid expansion when shifting blocker right
        project.config.grid = GridConfig { cols: 2, rows: 1 };
        save_project_config(dir.path(), &project.config).expect("save tight grid");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Select parent_svc (index 0 in visible list: [blocker_svc, parent_svc] sorted by id)
        let parent_vis_idx = app
            .visible_component_indices()
            .into_iter()
            .position(|i| app.project.components[i].id == "parent_svc")
            .expect("parent visible");
        app.selected_index = parent_vis_idx;

        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("parent_svc"));

        let child_pos = app.project.component("child_mod").unwrap().position;
        let blocker_pos = app.project.component("blocker_svc").unwrap().position;
        let parent_pos = app.project.component("parent_svc").unwrap().position;

        assert_eq!(child_pos, GridPos { col: 1, row: 0 });
        assert_ne!(blocker_pos, child_pos);
        assert_ne!(blocker_pos, parent_pos);
        assert_eq!(blocker_pos, GridPos { col: 2, row: 0 });
        assert!(app.project.config.grid.cols >= 3);

        // Persisted shifted card and expanded grid on disk
        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("blocker_svc").unwrap().position,
            GridPos { col: 2, row: 0 }
        );
        assert!(reloaded.config.grid.cols >= 3);
    }

    #[test]
    fn edit_component_changes_archetype_without_clobbering_and_adds_sub_component() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Edit Sub Test", "Archetype and sub-component")
            .expect("init workspace");

        let mut comp = make_component("core_engine");
        comp.kind = ComponentKind::Concept;
        comp.description = "Existing description".to_string();
        comp.markdown_body = "# Core Engine\nCustom architecture notes.\n".to_string();
        comp.position = GridPos { col: 0, row: 0 };
        comp.features.push(FeatureItem {
            id: "core_engine_f1".to_string(),
            title: "Existing feature".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save core_engine");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Open Edit Component modal
        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.mode, InputMode::Modal);

        // Tab to Archetype (field 1), move right twice: Concept -> Interface -> Logic
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Right));

        // Tab through Description (2), Path (3), Tags (4), Add Feature (5) to Add Sub-Component (6)
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Token Parser");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.status_message.contains("Token Parser"));

        let updated = app
            .project
            .component("core_engine")
            .expect("core_engine exists");
        assert_eq!(updated.kind, ComponentKind::Logic);
        assert_eq!(updated.description, "Existing description");
        assert_eq!(updated.features.len(), 1);
        assert_eq!(updated.features[0].title, "Existing feature");
        assert_eq!(
            updated.markdown_body,
            "# Core Engine\nCustom architecture notes.\n"
        );

        let sub = app
            .project
            .component("token_parser")
            .expect("sub-component created with snake_case id");
        assert_eq!(sub.name, "Token Parser");
        assert_eq!(sub.kind, ComponentKind::Concept);
        assert_eq!(sub.parent_id.as_deref(), Some("core_engine"));
        assert_eq!(sub.position, GridPos { col: 1, row: 0 });

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("core_engine").unwrap().kind,
            ComponentKind::Logic
        );
        assert_eq!(
            reloaded
                .component("token_parser")
                .unwrap()
                .parent_id
                .as_deref(),
            Some("core_engine")
        );
    }

    #[test]
    fn edit_connection_contains_ingests_and_detaches_sub_component() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Contains Test", "EdgeKind::Contains ingestion")
            .expect("init workspace");

        let mut alpha = make_component("alpha");
        alpha.position = GridPos { col: 0, row: 0 };
        let mut beta = make_component("beta");
        beta.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &alpha).expect("save alpha");
        save_component(dir.path(), &beta).expect("save beta");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        project.config.connections.push(Connection {
            from: "beta".to_string(),
            to: "alpha".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        save_project_config(dir.path(), &project.config).expect("save config");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Edit alpha -> beta to EdgeKind::Contains (index 0 -> 4 via 4 Right presses)
        app.handle_key(key(KeyCode::Char('E')));
        assert_eq!(app.mode, InputMode::Modal);
        for _ in 0..4 {
            app.handle_key(key(KeyCode::Right));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::Contains);
        assert_eq!(
            app.project.component("beta").unwrap().parent_id.as_deref(),
            Some("alpha")
        );

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("beta").unwrap().parent_id.as_deref(),
            Some("alpha")
        );

        // Expand alpha, select beta, and try to set beta -> alpha to Contains -> rejected as cycle
        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("alpha"));
        app.selected_index = 1; // beta
        assert_eq!(app.selected_component().unwrap().id, "beta");

        app.handle_key(key(KeyCode::Char('E')));
        for _ in 0..4 {
            app.handle_key(key(KeyCode::Right));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("descendant"));
        assert_eq!(app.project.component("alpha").unwrap().parent_id, None);
        app.handle_key(key(KeyCode::Esc));

        // Select alpha again and change alpha -> beta from Contains to Custom -> detaches beta
        app.selected_index = 0;
        app.handle_key(key(KeyCode::Char('E')));
        app.handle_key(key(KeyCode::Right)); // Contains -> Custom
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::Custom);
        assert_eq!(app.project.component("beta").unwrap().parent_id, None);

        let reloaded_after = load_project(dir.path()).expect("reload after detach");
        assert_eq!(reloaded_after.component("beta").unwrap().parent_id, None);
    }

    #[test]
    fn delete_parent_detaches_sub_components() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Delete Parent", "Detach children on delete")
            .expect("init workspace");

        let mut parent = make_component("parent_box");
        parent.position = GridPos { col: 0, row: 0 };
        let mut child = make_component("child_box");
        child.parent_id = Some("parent_box".to_string());
        child.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &parent).expect("save parent");
        save_component(dir.path(), &child).expect("save child");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Expand parent_box then delete it
        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("parent_box"));

        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.project.component("parent_box").is_none());
        assert!(!app.is_expanded("parent_box"));
        let detached = app
            .project
            .component("child_box")
            .expect("child_box still exists");
        assert_eq!(detached.parent_id, None);
        assert_eq!(app.visible_component_indices().len(), 1);

        let reloaded = load_project(dir.path()).expect("reload project");
        assert!(reloaded.component("parent_box").is_none());
        assert_eq!(reloaded.component("child_box").unwrap().parent_id, None);
    }

    #[test]
    fn connect_prompt_states_dependency_direction() {
        use crate::storage::init_workspace;

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Connect Prompt", "Connect prompt test")
            .expect("init workspace");

        let mut alpha = make_component("alpha");
        alpha.name = "Alpha Service".to_string();
        alpha.position = GridPos { col: 0, row: 0 };
        let mut beta = make_component("beta");
        beta.name = "Beta Store".to_string();
        beta.position = GridPos { col: 1, row: 0 };

        let mut app = make_app(vec![alpha, beta]);
        app.root = dir.path().to_path_buf();
        app.focus = FocusColumn::Canvas;

        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(
            app.status_message,
            "Connect: select dependency for Alpha Service (Alpha Service depends on <target>), Enter to confirm"
        );

        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.status_message, "Connected: alpha depends on beta");
    }

    #[test]
    fn zoom_keys_cycle_between_compact_normal_and_expanded() {
        use crate::tui::canvas::ZoomLevel;

        let mut app = make_app(vec![make_component("alpha")]);
        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);

        app.handle_key(key(KeyCode::Char('+')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Normal);
        assert_eq!(app.status_message, "Zoom: Normal");

        app.handle_key(key(KeyCode::Char('=')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Expanded);
        assert_eq!(app.status_message, "Zoom: Expanded");

        // Clamps at Expanded
        app.handle_key(key(KeyCode::Char('+')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Expanded);
        assert_eq!(app.status_message, "Zoom: Expanded");

        app.handle_key(key(KeyCode::Char('-')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Normal);
        assert_eq!(app.status_message, "Zoom: Normal");

        app.handle_key(key(KeyCode::Char('-')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);
        assert_eq!(app.status_message, "Zoom: Compact");

        // Clamps at Compact
        app.handle_key(key(KeyCode::Char('-')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);
        assert_eq!(app.status_message, "Zoom: Compact");
    }

    #[test]
    fn first_unoccupied_grid_pos_starts_top_center() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let empty_app = make_app(Vec::new());
        let first_pos = empty_app
            .first_unoccupied_grid_pos()
            .expect("empty grid has free cell");
        assert_eq!(first_pos.row, 0);
        assert_eq!(first_pos.col, 3);
        assert_ne!(first_pos, GridPos { col: 0, row: 0 });

        // Full 1x1 grid returns None when effective capacity does not expand it.
        let mut full_app = make_app(vec![make_component("only")]);
        full_app.project.config.grid = GridConfig { cols: 1, rows: 1 };
        assert_eq!(full_app.first_unoccupied_grid_pos(), None);

        // Creating a top-level component in a floored cell outside saved grid persists grid expansion.
        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Grid Floor", "Persist expanded grid").expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        render_app(&mut app, 140, 40);

        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "beta");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Beta");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        let beta = app.project.component("beta").expect("beta created");
        assert_eq!(beta.position, GridPos { col: 2, row: 0 });
        assert_eq!(app.project.config.grid, GridConfig { cols: 3, rows: 1 });

        let reloaded = load_project(dir.path()).expect("reload after creation");
        assert_eq!(reloaded.config.grid, GridConfig { cols: 3, rows: 1 });
    }

    fn render_app(app: &mut App, width: u16, height: u16) {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("create test terminal");
        terminal
            .draw(|frame| crate::tui::ui::render(frame, app))
            .expect("render frame");
    }

    fn set_dir_readonly(path: &std::path::Path, readonly: bool) {
        let mut perms = std::fs::metadata(path)
            .expect("read metadata")
            .permissions();
        perms.set_readonly(readonly);
        std::fs::set_permissions(path, perms).expect("set permissions");
    }

    #[test]
    fn create_top_level_rolls_back_component_when_config_save_fails() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Rollback Create", "Rollback top-level create")
            .expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        render_app(&mut app, 140, 40);

        let rocket_dir = dir.path().join(ROCKET_DIR);
        let project_toml = rocket_dir.join(crate::storage::PROJECT_FILE);
        set_dir_readonly(&project_toml, true);
        set_dir_readonly(&rocket_dir, true);

        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "beta");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Beta");
        app.handle_key(key(KeyCode::Enter));

        set_dir_readonly(&rocket_dir, false);
        set_dir_readonly(&project_toml, false);

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save grid config"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }
        assert!(app.project.component("beta").is_none());
        assert_eq!(app.project.config.grid, GridConfig { cols: 1, rows: 1 });

        let reloaded = load_project(dir.path()).expect("reload after rollback");
        assert!(reloaded.component("beta").is_none());
        assert_eq!(reloaded.config.grid, GridConfig { cols: 1, rows: 1 });
    }

    #[test]
    fn create_sub_component_rolls_back_component_when_config_save_fails() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Rollback Sub", "Rollback sub-component create")
            .expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        let rocket_dir = dir.path().join(ROCKET_DIR);
        let project_toml = rocket_dir.join(crate::storage::PROJECT_FILE);
        set_dir_readonly(&project_toml, true);
        set_dir_readonly(&rocket_dir, true);

        app.handle_key(key(KeyCode::Char('a')));
        type_keys(&mut app, "alpha_sub");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Alpha Sub");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save grid config"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }
        assert!(app.project.component("alpha_sub").is_none());

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());

        set_dir_readonly(&rocket_dir, false);
        set_dir_readonly(&project_toml, false);

        let reloaded = load_project(dir.path()).expect("reload after rollback");
        assert!(reloaded.component("alpha_sub").is_none());
        assert_eq!(reloaded.config.grid, GridConfig { cols: 1, rows: 1 });
    }

    #[test]
    fn move_card_rolls_back_component_when_config_save_fails() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Rollback Move", "Rollback card move").expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        render_app(&mut app, 140, 40);

        let rocket_dir = dir.path().join(ROCKET_DIR);
        let project_toml = rocket_dir.join(crate::storage::PROJECT_FILE);
        set_dir_readonly(&project_toml, true);
        set_dir_readonly(&rocket_dir, true);

        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Right));

        set_dir_readonly(&rocket_dir, false);
        set_dir_readonly(&project_toml, false);

        assert!(app.status_message.contains("Failed to save grid config"));
        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .position,
            GridPos { col: 0, row: 0 }
        );
        assert_eq!(app.project.config.grid, GridConfig { cols: 1, rows: 1 });

        let reloaded = load_project(dir.path()).expect("reload after rollback");
        assert_eq!(
            reloaded.component("alpha").expect("alpha on disk").position,
            GridPos { col: 0, row: 0 }
        );
        assert_eq!(reloaded.config.grid, GridConfig { cols: 1, rows: 1 });
    }

    #[test]
    fn move_out_of_grid_card_forward_does_not_jump_backwards() {
        let mut far_right = make_component("alpha");
        far_right.position = GridPos { col: 20, row: 0 };
        let mut app = make_app(vec![far_right]);
        app.project.config.grid = GridConfig { cols: 8, rows: 8 };

        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Right));
        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .position,
            GridPos { col: 20, row: 0 }
        );

        // Also verify Down on an out-of-grid row does not jump backwards.
        app.project.components[0].position = GridPos { col: 0, row: 20 };
        app.handle_key(key(KeyCode::Down));
        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .position,
            GridPos { col: 0, row: 20 }
        );
    }

    #[test]
    fn mutation_errors_when_rocket_dir_missing() {
        let dir = tempfile::tempdir().expect("tempdir without .rocket");
        let mut alpha = make_component("alpha");
        alpha.position = GridPos { col: 0, row: 0 };
        alpha.description = "Initial alpha".to_string();

        let mut app = make_app(vec![alpha]);
        app.root = dir.path().to_path_buf();
        let initial_project = app.project.clone();

        // 1. Create component fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "beta");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Beta");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project, initial_project);
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("Failed to save"));
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected NewComponent modal to remain open");
        }
        app.handle_key(key(KeyCode::Esc));

        // 2. Edit component fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Tab)); // archetype
        app.handle_key(key(KeyCode::Tab)); // description
        type_keys(&mut app, " edited");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project, initial_project);
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("Failed to save"));
        if let Some(ModalState::EditComponent(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected EditComponent modal to remain open");
        }
        app.handle_key(key(KeyCode::Esc));

        // 3. Move card fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.project, initial_project);
        assert!(app.status_message.contains("Failed to save"));
        app.handle_key(key(KeyCode::Char('m')));

        // 4. Delete component fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.project, initial_project);
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("Failed to delete"));
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open");
        }
    }

    #[test]
    fn update_component_clamps_selection_under_gaps_only() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Gaps Clamp", "GapsOnly update clamp test")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        app.filter = ListFilter::GapsOnly;
        app.selected_index = 1;
        assert_eq!(app.selected_component().unwrap().id, "beta");

        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Tab)); // archetype
        app.handle_key(key(KeyCode::Tab)); // description
        app.handle_key(key(KeyCode::Tab)); // path
        app.handle_key(key(KeyCode::Tab)); // tags
        app.handle_key(key(KeyCode::Tab)); // add feature
        type_keys(&mut app, "Initial feature");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.selected_index, 0);
        assert!(app.selected_component().is_some());
        assert_eq!(app.selected_component().unwrap().id, "alpha");
    }

    #[test]
    fn visible_component_indices_includes_disconnected_parent_cycle() {
        let mut alpha = make_component("alpha");
        alpha.parent_id = Some("beta".to_string());
        let mut beta = make_component("beta");
        beta.parent_id = Some("alpha".to_string());

        let app = make_app(vec![alpha, beta]);
        let visible = app.visible_component_indices();
        assert_eq!(visible, vec![0, 1]);
    }

    #[test]
    fn find_proximity_grid_pos_terminates_at_u16_max() {
        let mut edge = make_component("edge");
        edge.position = GridPos {
            col: u16::MAX,
            row: 0,
        };
        let mut app = make_app(vec![edge]);
        app.project.config.grid = GridConfig {
            cols: u16::MAX,
            rows: 1,
        };

        let pos = app.find_proximity_grid_pos(GridPos {
            col: u16::MAX,
            row: 0,
        });
        assert!(pos.is_some());
        assert_ne!(
            pos,
            Some(GridPos {
                col: u16::MAX,
                row: 0,
            })
        );
    }

    #[test]
    fn sub_component_creation_adds_contains_edge_without_hierarchy_drift() {
        use crate::audit::FindingKind;
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Hierarchy Sync", "Contains edge sync test")
            .expect("init workspace");

        let mut parent = make_component("core_svc");
        parent.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &parent).expect("save parent");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // 1. Create sub-component via 'a' (ModalAction::CreateComponent { parent_id: Some(...) })
        app.handle_key(key(KeyCode::Char('a')));
        type_keys(&mut app, "sub_alpha");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Sub Alpha");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.project.config.connections.iter().any(|c| {
            c.from == "core_svc" && c.to == "sub_alpha" && c.kind == EdgeKind::Contains
        }));
        assert!(
            !app.audit_findings
                .iter()
                .any(|f| f.kind == FindingKind::HierarchyDrift),
            "expected zero HierarchyDrift findings after 'a' sub-component creation, got: {:?}",
            app.audit_findings
        );

        // 2. Select core_svc again and add sub-component via 'e' (ModalAction::UpdateComponent)
        app.selected_index = 0;
        assert_eq!(app.selected_component().expect("core_svc").id, "core_svc");
        app.handle_key(key(KeyCode::Char('e')));
        for _ in 0..6 {
            app.handle_key(key(KeyCode::Tab));
        }
        type_keys(&mut app, "Sub Beta");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        let sub_beta = app
            .project
            .components
            .iter()
            .find(|c| c.name == "Sub Beta")
            .expect("sub_beta created");
        let sub_beta_id = sub_beta.id.clone();
        assert!(app.project.config.connections.iter().any(|c| {
            c.from == "core_svc" && c.to == sub_beta_id && c.kind == EdgeKind::Contains
        }));
        assert!(
            !app.audit_findings
                .iter()
                .any(|f| f.kind == FindingKind::HierarchyDrift),
            "expected zero HierarchyDrift findings after UpdateComponent sub-component creation, got: {:?}",
            app.audit_findings
        );

        // Verify persisted on disk
        let reloaded = load_project(dir.path()).expect("reload project");
        assert!(reloaded.config.connections.iter().any(|c| {
            c.from == "core_svc" && c.to == "sub_alpha" && c.kind == EdgeKind::Contains
        }));
        assert!(reloaded.config.connections.iter().any(|c| {
            c.from == "core_svc" && c.to == sub_beta_id && c.kind == EdgeKind::Contains
        }));
        let disk_findings = audit_project(&reloaded);
        assert!(
            !disk_findings
                .iter()
                .any(|f| f.kind == FindingKind::HierarchyDrift),
            "expected zero HierarchyDrift on disk, got: {:?}",
            disk_findings
        );
    }

    #[test]
    fn update_component_rolls_back_on_partial_disk_failure() {
        use crate::storage::{init_workspace, load_project, save_component, PROJECT_FILE};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Update Rollback", "Partial disk failure test")
            .expect("init workspace");

        let mut parent = make_component("alpha");
        parent.position = GridPos { col: 0, row: 0 };
        parent.description = "Original description".to_string();
        save_component(dir.path(), &parent).expect("save alpha");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        let initial_project = app.project.clone();

        // Force project.toml save failure by replacing project.toml with a directory
        let project_toml = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        let saved_toml_bytes = std::fs::read(&project_toml).expect("read original project.toml");
        std::fs::remove_file(&project_toml).expect("remove project.toml");
        std::fs::create_dir(&project_toml).expect("replace project.toml with dir");

        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Tab)); // archetype
        app.handle_key(key(KeyCode::Tab)); // description
        type_keys(&mut app, " mutated");
        app.handle_key(key(KeyCode::Tab)); // path
        app.handle_key(key(KeyCode::Tab)); // tags
        app.handle_key(key(KeyCode::Tab)); // add feature
        app.handle_key(key(KeyCode::Tab)); // add sub-component
        type_keys(&mut app, "Worker");
        app.handle_key(key(KeyCode::Enter));

        // Modal stays open with error and in-memory project is unchanged
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::EditComponent(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected EditComponent modal to remain open");
        }
        assert_eq!(app.project, initial_project);

        // Restore project.toml so we can reload and inspect disk state
        std::fs::remove_dir(&project_toml).expect("remove blocking dir");
        std::fs::write(&project_toml, saved_toml_bytes).expect("restore project.toml");

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(reloaded.components.len(), 1);
        assert_eq!(
            reloaded
                .component("alpha")
                .expect("alpha on disk")
                .description,
            "Original description"
        );
        assert!(reloaded.component("alpha_worker").is_none());
    }

    #[test]
    fn confirm_connect_only_toggles_depends_on() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Connect Toggle", "DependsOn only toggle test")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DataFlow,
            label: Some("stream".to_string()),
        });
        save_project_config(dir.path(), &project.config).expect("save DataFlow edge");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        app.focus = FocusColumn::Canvas;

        // First 'c' connect alpha -> beta adds DependsOn while preserving DataFlow
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.project.config.connections.len(), 2);
        assert!(app
            .project
            .config
            .connections
            .iter()
            .any(|c| { c.from == "alpha" && c.to == "beta" && c.kind == EdgeKind::DataFlow }));
        assert!(app
            .project
            .config
            .connections
            .iter()
            .any(|c| { c.from == "alpha" && c.to == "beta" && c.kind == EdgeKind::DependsOn }));

        // Second 'c' connect alpha -> beta removes only DependsOn, keeping DataFlow
        app.handle_key(key(KeyCode::Left));
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DataFlow);
        assert_eq!(
            app.project.config.connections[0].label.as_deref(),
            Some("stream")
        );

        let reloaded = load_project(dir.path()).expect("reload after toggle");
        assert_eq!(reloaded.config.connections.len(), 1);
        assert_eq!(reloaded.config.connections[0].kind, EdgeKind::DataFlow);
    }

    #[test]
    fn delete_contains_connection_clears_child_parent_id() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Delete Contains", "Clear parent_id on delete")
            .expect("init workspace");

        let mut parent = make_component("parent_svc");
        parent.position = GridPos { col: 0, row: 0 };
        let mut child = make_component("child_mod");
        child.parent_id = Some("parent_svc".to_string());
        child.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &parent).expect("save parent");
        save_component(dir.path(), &child).expect("save child");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "parent_svc".to_string(),
            to: "child_mod".to_string(),
            kind: EdgeKind::Contains,
            label: None,
        });
        save_project_config(dir.path(), &project.config).expect("save Contains edge");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Select parent_svc (index 0) and delete its Contains connection via 'D' -> 'y'
        assert_eq!(app.selected_component().expect("parent").id, "parent_svc");
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Char('y')));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.project.config.connections.is_empty());
        assert_eq!(
            app.project.component("child_mod").expect("child").parent_id,
            None
        );

        let reloaded = load_project(dir.path()).expect("reload after delete");
        assert!(reloaded.config.connections.is_empty());
        assert_eq!(
            reloaded
                .component("child_mod")
                .expect("child on disk")
                .parent_id,
            None
        );
    }

    #[test]
    fn delete_component_rolls_back_detached_children_on_failure() {
        use crate::storage::{init_workspace, load_project, save_component, PROJECT_FILE};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Delete Rollback", "Rollback detached child")
            .expect("init workspace");

        let mut parent = make_component("parent_svc");
        parent.position = GridPos { col: 0, row: 0 };
        let mut child = make_component("child_mod");
        child.parent_id = Some("parent_svc".to_string());
        child.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &parent).expect("save parent");
        save_component(dir.path(), &child).expect("save child");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "parent_svc".to_string(),
            to: "child_mod".to_string(),
            kind: EdgeKind::Contains,
            label: None,
        });
        save_project_config(dir.path(), &project.config).expect("save Contains edge");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Force config save failure after child_mod has been detached on disk
        let project_toml = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        let saved_toml_bytes = std::fs::read(&project_toml).expect("read project.toml");
        std::fs::remove_file(&project_toml).expect("remove project.toml");
        std::fs::create_dir(&project_toml).expect("replace project.toml with dir");

        assert_eq!(app.selected_component().expect("parent").id, "parent_svc");
        app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Char('y')));

        // Modal stays open with error and in-memory state is unchanged
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open");
        }
        assert!(app.project.component("parent_svc").is_some());
        assert_eq!(
            app.project
                .component("child_mod")
                .expect("child in memory")
                .parent_id
                .as_deref(),
            Some("parent_svc")
        );

        // Restore project.toml and confirm child_mod was rolled back on disk
        std::fs::remove_dir(&project_toml).expect("remove blocking dir");
        std::fs::write(&project_toml, saved_toml_bytes).expect("restore project.toml");

        let reloaded = load_project(dir.path()).expect("reload after rollback");
        assert!(reloaded.component("parent_svc").is_some());
        assert_eq!(
            reloaded
                .component("child_mod")
                .expect("child on disk")
                .parent_id
                .as_deref(),
            Some("parent_svc")
        );
    }

    #[test]
    fn multi_edge_edit_and_delete_select_non_first_edge() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Multi Edge", "Cycle edit and delete edges")
            .expect("init workspace");

        let mut alpha = make_component("alpha");
        alpha.position = GridPos { col: 0, row: 0 };
        let mut beta = make_component("beta");
        beta.position = GridPos { col: 1, row: 0 };
        let mut gamma = make_component("gamma");
        gamma.position = GridPos { col: 2, row: 0 };
        save_component(dir.path(), &alpha).expect("save alpha");
        save_component(dir.path(), &beta).expect("save beta");
        save_component(dir.path(), &gamma).expect("save gamma");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections = vec![
            Connection {
                from: "alpha".to_string(),
                to: "beta".to_string(),
                kind: EdgeKind::DependsOn,
                label: Some("orig".to_string()),
            },
            Connection {
                from: "alpha".to_string(),
                to: "gamma".to_string(),
                kind: EdgeKind::DataFlow,
                label: None,
            },
        ];
        save_project_config(dir.path(), &project.config).expect("save config");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Selected is alpha (index 0). Press E, cycle to alpha -> gamma via ], set label, submit.
        assert_eq!(app.selected_component().expect("alpha").id, "alpha");
        app.handle_key(key(KeyCode::Char('E')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Char(']')));
        app.handle_key(key(KeyCode::Tab)); // move to Label field
        for ch in "stream".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections.len(), 2);
        assert_eq!(app.project.config.connections[0].to, "beta");
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DependsOn);
        assert_eq!(
            app.project.config.connections[0].label.as_deref(),
            Some("orig")
        );
        assert_eq!(app.project.config.connections[1].to, "gamma");
        assert_eq!(app.project.config.connections[1].kind, EdgeKind::DataFlow);
        assert_eq!(
            app.project.config.connections[1].label.as_deref(),
            Some("stream")
        );

        // Press D, cycle to alpha -> gamma via Down, confirm with y -> alpha -> gamma deleted, alpha -> beta remains.
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Char('y')));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].from, "alpha");
        assert_eq!(app.project.config.connections[0].to, "beta");
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DependsOn);
    }

    #[test]
    fn action_keys_are_ignored_in_move_card_and_connect_modes() {
        let mut app = make_app(vec![make_component("alpha"), make_component("beta")]);
        app.project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });

        let guarded_keys = ['n', 'e', 'E', 'd', 'D', 'v', 'r', 'p', 't', 'f', '?'];

        // MoveCard mode
        app.handle_key(key(KeyCode::Char('m')));
        assert_eq!(app.mode, InputMode::MoveCard);
        for ch in guarded_keys {
            app.handle_key(key(KeyCode::Char(ch)));
            assert_eq!(
                app.mode,
                InputMode::MoveCard,
                "key '{ch}' should not leave MoveCard mode"
            );
            assert!(
                app.modal.is_none(),
                "key '{ch}' should not open modal in MoveCard mode"
            );
        }
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);

        // Connect mode
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);
        for ch in guarded_keys {
            app.handle_key(key(KeyCode::Char(ch)));
            assert_eq!(
                app.mode,
                InputMode::Connect,
                "key '{ch}' should not leave Connect mode"
            );
            assert!(
                app.modal.is_none(),
                "key '{ch}' should not open modal in Connect mode"
            );
        }
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
    }
}
