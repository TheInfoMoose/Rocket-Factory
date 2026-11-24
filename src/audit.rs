use std::collections::{HashMap, HashSet};

use crate::model::{CardStatus, Component, EdgeKind, GridPos, Project, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    UntrackedComponent,
    UnpushedFeatures,
    DanglingConnection,
    DuplicateComponentId,
    InvalidParent,
    SelfConnection,
    InvalidGrid,
    HierarchyDrift,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditFinding {
    pub severity: FindingSeverity,
    pub kind: FindingKind,
    pub component_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskProgress {
    pub complete: usize,
    pub total: usize,
}

pub fn compute_card_status(component: &Component) -> CardStatus {
    if component.external_tasks.is_empty() {
        return if component.features.is_empty() {
            CardStatus::Gap
        } else {
            CardStatus::Planned
        };
    }

    if component
        .external_tasks
        .iter()
        .any(|task| task.status == TaskStatus::Blocked)
    {
        return CardStatus::Blocked;
    }

    if component
        .external_tasks
        .iter()
        .all(|task| task.status == TaskStatus::Complete)
    {
        return CardStatus::Complete;
    }

    CardStatus::InProgress
}

pub fn task_progress(component: &Component) -> TaskProgress {
    let complete = component
        .external_tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Complete)
        .count();
    TaskProgress {
        complete,
        total: component.external_tasks.len(),
    }
}

pub fn audit_project(project: &Project) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    let mut ids: HashSet<&str> = HashSet::new();
    let mut reported_duplicates: HashSet<&str> = HashSet::new();
    let mut parent_by_id: HashMap<&str, &str> = HashMap::new();

    for component in &project.components {
        if !ids.insert(component.id.as_str()) && reported_duplicates.insert(component.id.as_str()) {
            findings.push(AuditFinding {
                severity: FindingSeverity::Error,
                kind: FindingKind::DuplicateComponentId,
                component_id: Some(component.id.clone()),
                message: format!("Duplicate component ID '{}'", component.id),
            });
        }
        if let Some(parent_id) = component.parent_id.as_deref() {
            parent_by_id
                .entry(component.id.as_str())
                .or_insert(parent_id);
        }
    }

    let grid = project.config.grid;
    let grid_valid = grid.cols > 0 && grid.rows > 0;
    if !grid_valid {
        findings.push(AuditFinding {
            severity: FindingSeverity::Error,
            kind: FindingKind::InvalidGrid,
            component_id: None,
            message: format!(
                "Grid dimensions {}x{} must be at least 1x1",
                grid.cols, grid.rows
            ),
        });
    }

    let mut occupied_positions: HashMap<GridPos, &str> = HashMap::new();
    let mut contains_edges: HashSet<(&str, &str)> = HashSet::new();
    for conn in &project.config.connections {
        if conn.kind == EdgeKind::Contains {
            contains_edges.insert((conn.from.as_str(), conn.to.as_str()));
        }
    }

    for component in &project.components {
        let unpushed_count = component
            .features
            .iter()
            .filter(|feature| !feature.pushed)
            .count();

        if component.external_tasks.is_empty() && component.features.is_empty() {
            findings.push(AuditFinding {
                severity: FindingSeverity::Error,
                kind: FindingKind::UntrackedComponent,
                component_id: Some(component.id.clone()),
                message: format!(
                    "Component '{}' has no features or linked tasks",
                    component.id
                ),
            });
        } else if component.external_tasks.is_empty() && unpushed_count > 0 {
            findings.push(AuditFinding {
                severity: FindingSeverity::Warning,
                kind: FindingKind::UnpushedFeatures,
                component_id: Some(component.id.clone()),
                message: format!(
                    "Component '{}' has unpushed local features and no linked tasks",
                    component.id
                ),
            });
        } else if unpushed_count > 0 {
            findings.push(AuditFinding {
                severity: FindingSeverity::Warning,
                kind: FindingKind::UnpushedFeatures,
                component_id: Some(component.id.clone()),
                message: format!(
                    "Component '{}' has {} unpushed local feature(s)",
                    component.id, unpushed_count
                ),
            });
        }

        if let Some(parent_id) = component.parent_id.as_deref() {
            if !ids.contains(parent_id) {
                findings.push(AuditFinding {
                    severity: FindingSeverity::Error,
                    kind: FindingKind::InvalidParent,
                    component_id: Some(component.id.clone()),
                    message: format!(
                        "Component '{}' references missing parent '{}'",
                        component.id, parent_id
                    ),
                });
            } else {
                let mut visited = HashSet::new();
                visited.insert(component.id.as_str());
                let mut current = parent_id;
                while visited.insert(current) {
                    match parent_by_id.get(current) {
                        Some(&next) => current = next,
                        None => break,
                    }
                }
                if current == component.id.as_str() {
                    findings.push(AuditFinding {
                        severity: FindingSeverity::Error,
                        kind: FindingKind::InvalidParent,
                        component_id: Some(component.id.clone()),
                        message: format!(
                            "Component '{}' is part of a parent_id cycle",
                            component.id
                        ),
                    });
                }

                if parent_id != component.id.as_str()
                    && !contains_edges.contains(&(parent_id, component.id.as_str()))
                {
                    findings.push(AuditFinding {
                        severity: FindingSeverity::Warning,
                        kind: FindingKind::HierarchyDrift,
                        component_id: Some(component.id.clone()),
                        message: format!(
                            "Component '{}' has parent_id '{}' without matching Contains connection",
                            component.id, parent_id
                        ),
                    });
                }
            }
        }

        if grid_valid
            && (component.position.col >= grid.cols || component.position.row >= grid.rows)
        {
            findings.push(AuditFinding {
                severity: FindingSeverity::Error,
                kind: FindingKind::InvalidGrid,
                component_id: Some(component.id.clone()),
                message: format!(
                    "Component '{}' position ({}, {}) is outside grid bounds {}x{}",
                    component.id,
                    component.position.col,
                    component.position.row,
                    grid.cols,
                    grid.rows
                ),
            });
        }

        if let Some(&existing_id) = occupied_positions.get(&component.position) {
            findings.push(AuditFinding {
                severity: FindingSeverity::Error,
                kind: FindingKind::InvalidGrid,
                component_id: Some(component.id.clone()),
                message: format!(
                    "Component '{}' overlaps '{}' at grid position ({}, {})",
                    component.id, existing_id, component.position.col, component.position.row
                ),
            });
        } else {
            occupied_positions.insert(component.position, component.id.as_str());
        }
    }

    for conn in &project.config.connections {
        if conn.from == conn.to {
            findings.push(AuditFinding {
                severity: FindingSeverity::Error,
                kind: FindingKind::SelfConnection,
                component_id: Some(conn.from.clone()),
                message: format!("Component '{}' cannot connect to itself", conn.from),
            });
        }

        let from_exists = ids.contains(conn.from.as_str());
        let to_exists = ids.contains(conn.to.as_str());
        if !from_exists || !to_exists {
            findings.push(AuditFinding {
                severity: FindingSeverity::Error,
                kind: FindingKind::DanglingConnection,
                component_id: None,
                message: format!(
                    "Connection '{} -> {}' references missing component",
                    conn.from, conn.to
                ),
            });
        } else if conn.kind == EdgeKind::Contains
            && parent_by_id.get(conn.to.as_str()).copied() != Some(conn.from.as_str())
        {
            findings.push(AuditFinding {
                severity: FindingSeverity::Warning,
                kind: FindingKind::HierarchyDrift,
                component_id: Some(conn.to.clone()),
                message: format!(
                    "Contains connection '{} -> {}' does not match child parent_id",
                    conn.from, conn.to
                ),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ComponentKind, Connection, EdgeKind, ExternalTaskRef, FeatureItem, GridConfig, GridPos,
        ProjectConfig,
    };

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

    #[test]
    fn card_status_transitions() {
        let empty = make_component("empty");
        assert_eq!(compute_card_status(&empty), CardStatus::Gap);

        let mut planned = make_component("planned");
        planned.features.push(FeatureItem {
            id: "feat-1".to_string(),
            title: "Initial feature".to_string(),
            pushed: false,
            external_id: None,
        });
        assert_eq!(compute_card_status(&planned), CardStatus::Planned);

        let mut in_progress = make_component("in_progress");
        in_progress.external_tasks = vec![
            ExternalTaskRef {
                external_id: "TASK-1".to_string(),
                title: "Pending task".to_string(),
                status: TaskStatus::Pending,
            },
            ExternalTaskRef {
                external_id: "TASK-2".to_string(),
                title: "Done task".to_string(),
                status: TaskStatus::Complete,
            },
        ];
        assert_eq!(compute_card_status(&in_progress), CardStatus::InProgress);

        let mut blocked = make_component("blocked");
        blocked.external_tasks = vec![
            ExternalTaskRef {
                external_id: "TASK-3".to_string(),
                title: "Done task".to_string(),
                status: TaskStatus::Complete,
            },
            ExternalTaskRef {
                external_id: "TASK-4".to_string(),
                title: "Blocked task".to_string(),
                status: TaskStatus::Blocked,
            },
        ];
        assert_eq!(compute_card_status(&blocked), CardStatus::Blocked);

        let mut complete = make_component("complete");
        complete.external_tasks = vec![
            ExternalTaskRef {
                external_id: "TASK-5".to_string(),
                title: "Done 1".to_string(),
                status: TaskStatus::Complete,
            },
            ExternalTaskRef {
                external_id: "TASK-6".to_string(),
                title: "Done 2".to_string(),
                status: TaskStatus::Complete,
            },
        ];
        assert_eq!(compute_card_status(&complete), CardStatus::Complete);
    }

    #[test]
    fn audit_flags_untracked_view_box_and_dangling_edge() {
        let mut data_stream = make_component("data_stream");
        data_stream.external_tasks.push(ExternalTaskRef {
            external_id: "TASK-10".to_string(),
            title: "Ingest events".to_string(),
            status: TaskStatus::Complete,
        });

        let view_box = make_component("view_box");

        let project = Project {
            config: ProjectConfig {
                name: "Test Project".to_string(),
                description: "Audit test".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: vec![Connection {
                    from: "data_stream".to_string(),
                    to: "ghost_node".to_string(),
                    kind: EdgeKind::DataFlow,
                    label: None,
                }],
            },
            components: vec![data_stream, view_box],
        };

        let findings = audit_project(&project);

        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::UntrackedComponent
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("view_box")
        }));

        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::DanglingConnection
                && finding.severity == FindingSeverity::Error
                && finding.component_id.is_none()
                && finding.message.contains("ghost_node")
        }));
    }

    #[test]
    fn task_progress_counts_completed_tasks() {
        let mut comp = make_component("worker");
        comp.external_tasks = vec![
            ExternalTaskRef {
                external_id: "TASK-1".to_string(),
                title: "First".to_string(),
                status: TaskStatus::Complete,
            },
            ExternalTaskRef {
                external_id: "TASK-2".to_string(),
                title: "Second".to_string(),
                status: TaskStatus::Complete,
            },
            ExternalTaskRef {
                external_id: "TASK-3".to_string(),
                title: "Third".to_string(),
                status: TaskStatus::InProgress,
            },
        ];

        assert_eq!(
            task_progress(&comp),
            TaskProgress {
                complete: 2,
                total: 3,
            }
        );
    }

    #[test]
    fn audit_detects_parent_cycle_and_grid_overlap() {
        let mut cycle_a = make_component("cycle_a");
        cycle_a.parent_id = Some("cycle_b".to_string());
        cycle_a.position = GridPos { col: 0, row: 0 };

        let mut cycle_b = make_component("cycle_b");
        cycle_b.parent_id = Some("cycle_a".to_string());
        cycle_b.position = GridPos { col: 1, row: 0 };

        let mut orphan = make_component("orphan");
        orphan.parent_id = Some("ghost_parent".to_string());
        orphan.position = GridPos { col: 0, row: 0 };

        let mut dup_one = make_component("dup_node");
        dup_one.position = GridPos { col: 0, row: 0 };

        let mut dup_two = make_component("dup_node");
        dup_two.position = GridPos { col: 0, row: 0 };

        let mut drift_target = make_component("drift_target");
        drift_target.position = GridPos { col: 2, row: 0 };

        let project = Project {
            config: ProjectConfig {
                name: "Structural Audit".to_string(),
                description: "Detects structural defects".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig { cols: 4, rows: 4 },
                connections: vec![
                    Connection {
                        from: "cycle_a".to_string(),
                        to: "cycle_a".to_string(),
                        kind: EdgeKind::DependsOn,
                        label: None,
                    },
                    Connection {
                        from: "dup_node".to_string(),
                        to: "drift_target".to_string(),
                        kind: EdgeKind::Contains,
                        label: None,
                    },
                ],
            },
            components: vec![cycle_a, cycle_b, orphan, dup_one, dup_two, drift_target],
        };

        let findings = audit_project(&project);

        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::DuplicateComponentId
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("dup_node")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::InvalidParent
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("cycle_a")
                && finding.message.contains("cycle")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::InvalidParent
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("orphan")
                && finding.message.contains("ghost_parent")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::SelfConnection
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("cycle_a")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::InvalidGrid
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("dup_node")
                && finding.message.contains("overlaps")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::HierarchyDrift
                && finding.severity == FindingSeverity::Warning
                && finding.component_id.as_deref() == Some("drift_target")
        }));
    }

    #[test]
    fn audit_flags_sub_component_out_of_bounds_and_cross_parent_overlap() {
        let mut root_a = make_component("root_a");
        root_a.position = GridPos { col: 0, row: 0 };

        let mut root_b = make_component("root_b");
        root_b.position = GridPos { col: 1, row: 0 };

        let mut oob_child = make_component("oob_child");
        oob_child.parent_id = Some("root_a".to_string());
        oob_child.position = GridPos { col: 5, row: 0 };

        let mut overlap_child = make_component("overlap_child");
        overlap_child.parent_id = Some("root_b".to_string());
        overlap_child.position = GridPos { col: 0, row: 0 };

        let project = Project {
            config: ProjectConfig {
                name: "Global Grid Audit".to_string(),
                description: "Checks global bounds and overlaps".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig { cols: 4, rows: 4 },
                connections: vec![
                    Connection {
                        from: "root_a".to_string(),
                        to: "oob_child".to_string(),
                        kind: EdgeKind::Contains,
                        label: None,
                    },
                    Connection {
                        from: "root_b".to_string(),
                        to: "overlap_child".to_string(),
                        kind: EdgeKind::Contains,
                        label: None,
                    },
                ],
            },
            components: vec![root_a, root_b, oob_child, overlap_child],
        };

        let findings = audit_project(&project);

        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::InvalidGrid
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("oob_child")
                && finding.message.contains("outside grid bounds")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::InvalidGrid
                && finding.severity == FindingSeverity::Error
                && finding.component_id.as_deref() == Some("overlap_child")
                && finding.message.contains("overlaps")
        }));
    }

    #[test]
    fn audit_flags_unpushed_features_and_dangling_from_connection() {
        let mut planned_only = make_component("planned_only");
        planned_only.position = GridPos { col: 0, row: 0 };
        planned_only.features.push(FeatureItem {
            id: "feat-1".to_string(),
            title: "Draft local feature".to_string(),
            pushed: false,
            external_id: None,
        });

        let mut mixed_node = make_component("mixed_node");
        mixed_node.position = GridPos { col: 1, row: 0 };
        mixed_node.external_tasks.push(ExternalTaskRef {
            external_id: "TASK-20".to_string(),
            title: "Tracked task".to_string(),
            status: TaskStatus::InProgress,
        });
        mixed_node.features.push(FeatureItem {
            id: "feat-2".to_string(),
            title: "Pending local feature".to_string(),
            pushed: false,
            external_id: None,
        });

        let project = Project {
            config: ProjectConfig {
                name: "Unpushed & Dangling Source".to_string(),
                description: "Audit branch coverage".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig { cols: 4, rows: 4 },
                connections: vec![Connection {
                    from: "ghost_source".to_string(),
                    to: "planned_only".to_string(),
                    kind: EdgeKind::DependsOn,
                    label: None,
                }],
            },
            components: vec![planned_only, mixed_node],
        };

        let findings = audit_project(&project);

        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::UnpushedFeatures
                && finding.severity == FindingSeverity::Warning
                && finding.component_id.as_deref() == Some("planned_only")
                && finding.message.contains("no linked tasks")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::UnpushedFeatures
                && finding.severity == FindingSeverity::Warning
                && finding.component_id.as_deref() == Some("mixed_node")
                && finding.message.contains("1 unpushed local feature(s)")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == FindingKind::DanglingConnection
                && finding.severity == FindingSeverity::Error
                && finding.component_id.is_none()
                && finding.message.contains("ghost_source")
        }));
    }
}
