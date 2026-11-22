use std::collections::HashSet;

use crate::model::{CardStatus, Component, Project, TaskStatus};

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
    let ids: HashSet<&str> = project
        .components
        .iter()
        .map(|component| component.id.as_str())
        .collect();

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
    }

    for conn in &project.config.connections {
        if !ids.contains(conn.from.as_str()) || !ids.contains(conn.to.as_str()) {
            findings.push(AuditFinding {
                severity: FindingSeverity::Error,
                kind: FindingKind::DanglingConnection,
                component_id: None,
                message: format!(
                    "Connection '{} -> {}' references missing component",
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
}
