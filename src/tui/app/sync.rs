use std::collections::HashMap;
use std::path::Path;

use crate::adapter::{
    sync_pull_project, sync_pull_project_in_memory, sync_push_project, sync_push_project_in_memory,
    PullSyncReport, PushSyncReport,
};
use crate::error::RbResult;
use crate::model::{Component, ExternalTaskRef, FeatureItem, Project};

use super::state::{App, SyncRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureSyncUpdate {
    pub id: String,
    pub pushed: bool,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ComponentSyncUpdate {
    pub component_id: String,
    pub external_tasks: Option<Vec<ExternalTaskRef>>,
    pub feature_updates: Vec<FeatureSyncUpdate>,
}

#[derive(Debug, Clone)]
pub(crate) enum SyncOutcome {
    Success {
        updates: Vec<ComponentSyncUpdate>,
        status_message: String,
    },
    Error {
        updates: Vec<ComponentSyncUpdate>,
        status_message: String,
    },
}

fn format_pull_summary(report: &PullSyncReport) -> String {
    format!(
        "Synced {} adapter(s): {} component(s) updated, {} task(s) synced, {} feature(s) linked",
        report.adapters_count,
        report.components_updated,
        report.tasks_synced,
        report.features_linked
    )
}

fn format_push_summary(comp_id: &str, report: &PushSyncReport) -> String {
    let mut summary_msg = format!(
        "Pushed {comp_id}: {} component(s), {} feature(s) across {} adapter(s)",
        report.components_pushed, report.features_pushed, report.adapters_count
    );
    if let Some(details) = report.format_responses() {
        summary_msg.push_str(&format!(" [{details}]"));
    }
    summary_msg
}

type ComponentSyncSnapshot = (
    Vec<ExternalTaskRef>,
    HashMap<String, (bool, Option<String>)>,
);

fn diff_component_updates(
    before: &HashMap<String, ComponentSyncSnapshot>,
    components: Vec<Component>,
) -> Vec<ComponentSyncUpdate> {
    components
        .into_iter()
        .filter_map(|comp| {
            let (external_tasks, feature_updates) = match before.get(&comp.id) {
                Some((prev_tasks, prev_features)) => {
                    let external_tasks =
                        (*prev_tasks != comp.external_tasks).then_some(comp.external_tasks);
                    let feature_updates: Vec<FeatureSyncUpdate> = comp
                        .features
                        .into_iter()
                        .filter_map(|feat| {
                            let changed = match prev_features.get(&feat.id) {
                                Some((prev_pushed, prev_ext_id)) => {
                                    *prev_pushed != feat.pushed || *prev_ext_id != feat.external_id
                                }
                                None => true,
                            };
                            changed.then_some(FeatureSyncUpdate {
                                id: feat.id,
                                pushed: feat.pushed,
                                external_id: feat.external_id,
                            })
                        })
                        .collect();
                    (external_tasks, feature_updates)
                }
                None => {
                    let feature_updates: Vec<FeatureSyncUpdate> = comp
                        .features
                        .into_iter()
                        .map(|feat| FeatureSyncUpdate {
                            id: feat.id,
                            pushed: feat.pushed,
                            external_id: feat.external_id,
                        })
                        .collect();
                    (Some(comp.external_tasks), feature_updates)
                }
            };

            (external_tasks.is_some() || !feature_updates.is_empty()).then_some(
                ComponentSyncUpdate {
                    component_id: comp.id,
                    external_tasks,
                    feature_updates,
                },
            )
        })
        .collect()
}

pub(crate) fn run_sync_job(
    mut project: Project,
    root: &Path,
    adapters_path: &Path,
    request: SyncRequest,
    target_comp_id: Option<&str>,
    is_auto: bool,
) -> SyncOutcome {
    let before: HashMap<String, ComponentSyncSnapshot> = project
        .components
        .iter()
        .map(|c| {
            let feat_map = c
                .features
                .iter()
                .map(|f| (f.id.clone(), (f.pushed, f.external_id.clone())))
                .collect();
            (c.id.clone(), (c.external_tasks.clone(), feat_map))
        })
        .collect();

    let result = match request {
        SyncRequest::Pull => sync_pull_project_in_memory(&mut project, root, adapters_path, None)
            .map(|report| format_pull_summary(&report)),
        SyncRequest::PushSelected => {
            let Some(comp_id) = target_comp_id else {
                return SyncOutcome::Error {
                    updates: Vec::new(),
                    status_message: "No component selected to push".to_string(),
                };
            };
            sync_push_project_in_memory(
                &mut project,
                root,
                adapters_path,
                &[comp_id.to_string()],
                None,
            )
            .map(|report| format_push_summary(comp_id, &report))
        }
    };

    let updates = diff_component_updates(&before, project.components);

    match result {
        Ok(status_message) => SyncOutcome::Success {
            updates,
            status_message,
        },
        Err(err) => {
            let prefix = if is_auto {
                "Auto-sync failed"
            } else {
                "Adapter sync failed"
            };
            SyncOutcome::Error {
                updates,
                status_message: format!("{prefix}: {err}"),
            }
        }
    }
}

pub(crate) fn apply_sync_outcome(app: &mut App, outcome: SyncOutcome) {
    let (updates, status_message, is_error) = match outcome {
        SyncOutcome::Success {
            updates,
            status_message,
        } => (updates, status_message, false),
        SyncOutcome::Error {
            updates,
            status_message,
        } => (updates, status_message, true),
    };

    let mut updated_indices: Vec<usize> = Vec::new();
    let mut backups: Vec<(usize, Vec<ExternalTaskRef>, Vec<FeatureItem>)> = Vec::new();

    for update in updates {
        if let Some(idx) = app
            .project
            .components
            .iter()
            .position(|comp| comp.id == update.component_id)
        {
            let comp = &mut app.project.components[idx];
            let prev_tasks = comp.external_tasks.clone();
            let prev_features = comp.features.clone();
            let mut changed = false;

            if let Some(tasks) = update.external_tasks {
                comp.external_tasks = tasks;
                changed = true;
            }
            for feat_update in update.feature_updates {
                if let Some(live_feat) = comp.features.iter_mut().find(|f| f.id == feat_update.id) {
                    live_feat.pushed = feat_update.pushed;
                    live_feat.external_id = feat_update.external_id;
                    changed = true;
                }
            }
            if changed {
                backups.push((idx, prev_tasks, prev_features));
                updated_indices.push(idx);
            }
        }
    }

    let mut save_error: Option<String> = None;
    for (pos, &idx) in updated_indices.iter().enumerate() {
        if let Err(err) = crate::storage::save_component(&app.root, &app.project.components[idx]) {
            for (restore_idx, prev_tasks, prev_features) in backups.into_iter().skip(pos).rev() {
                app.project.components[restore_idx].external_tasks = prev_tasks;
                app.project.components[restore_idx].features = prev_features;
            }
            save_error = Some(format!("Failed to save synced component: {err}"));
            break;
        }
    }

    app.refresh_audit();
    app.clamp_selection();
    app.sync_in_flight = false;
    app.status_message = match (is_error, save_error) {
        (true, Some(err_msg)) => format!("{status_message}; additionally {err_msg}"),
        (false, Some(err_msg)) => err_msg,
        (_, None) => status_message,
    };
}

impl App {
    pub fn sync_pull(&mut self, adapters_path: &Path) -> RbResult<String> {
        if self.project.config.active_adapters.is_empty() {
            let msg = "No active adapters configured".to_string();
            self.status_message = msg.clone();
            return Ok(msg);
        }

        let report = sync_pull_project(&mut self.project, &self.root, adapters_path, None)?;
        self.refresh_audit();
        self.clamp_selection();

        let summary_msg = format_pull_summary(&report);
        self.status_message = summary_msg.clone();
        Ok(summary_msg)
    }

    pub fn sync_push_selected(&mut self, adapters_path: &Path) -> RbResult<String> {
        let Some(comp_id) = self.selected_component().map(|c| c.id.clone()) else {
            let msg = "No component selected to push".to_string();
            self.status_message = msg.clone();
            return Ok(msg);
        };

        if self.project.config.active_adapters.is_empty() {
            let msg = "No active adapters configured".to_string();
            self.status_message = msg.clone();
            return Ok(msg);
        }

        let report = sync_push_project(
            &mut self.project,
            &self.root,
            adapters_path,
            std::slice::from_ref(&comp_id),
            None,
        )?;
        self.refresh_audit();
        self.clamp_selection();

        let summary_msg = format_push_summary(&comp_id, &report);
        self.status_message = summary_msg.clone();
        Ok(summary_msg)
    }
}
