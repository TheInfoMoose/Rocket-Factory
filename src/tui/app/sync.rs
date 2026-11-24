use std::path::Path;

use crate::adapter::{sync_pull_project, sync_push_project};
use crate::error::RbResult;

use super::state::App;

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

        let summary_msg = format!(
            "Synced {} adapter(s): {} component(s) updated, {} task(s) synced, {} feature(s) linked",
            report.adapters_count,
            report.components_updated,
            report.tasks_synced,
            report.features_linked
        );
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

        let mut summary_msg = format!(
            "Pushed {comp_id}: {} component(s), {} feature(s) across {} adapter(s)",
            report.components_pushed, report.features_pushed, report.adapters_count
        );
        if let Some(details) = report.format_responses() {
            summary_msg.push_str(&format!(" [{details}]"));
        }
        self.status_message = summary_msg.clone();
        Ok(summary_msg)
    }
}
