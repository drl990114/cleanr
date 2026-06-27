use super::*;

impl Workbench {
    pub(crate) fn list_len(&self) -> usize {
        match self.view {
            View::Home => 0,
            View::Scan => {
                if let Some(plan) = &self.plan {
                    return plan.items.len();
                }
                self.entries
                    .iter()
                    .filter(|e| !e.rule_hits.is_empty())
                    .count()
            }
            View::Languages => self.i18n.packs().len(),
            View::Rules => self
                .registry
                .packs()
                .iter()
                .map(|pack| 1 + pack.definition.rules.len())
                .sum(),
            View::Plugins => self.registry.packs().len() + self.plugin_diagnostics().len(),
            View::Tasks => self.task_log.len(),
            View::Usage => usage_entries(self).len(),
            View::Restore => self.execution_manifests.len(),
        }
    }

    pub(crate) fn reset_list_selection(&mut self) {
        if self.list_len() > 0 {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    pub(crate) fn plugin_diagnostics(&self) -> Vec<&PluginDiagnostic> {
        let mut seen = BTreeSet::new();
        self.registry
            .diagnostics()
            .iter()
            .chain(self.i18n.diagnostics())
            .filter(|diagnostic| {
                seen.insert(format!(
                    "{}\0{}\0{}",
                    diagnostic.code,
                    diagnostic.message,
                    diagnostic
                        .path
                        .as_ref()
                        .map_or_else(String::new, |path| path.display().to_string())
                ))
            })
            .collect()
    }

    pub(crate) fn select_next(&mut self) {
        self.select_next_n(1);
    }

    pub(crate) fn select_previous(&mut self) {
        self.select_previous_n(1);
    }

    pub(crate) fn select_next_n(&mut self, n: usize) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let next = self
            .list_state
            .selected()
            .map_or(0usize, |i: usize| (i + n).min(len - 1));
        self.list_state.select(Some(next));
    }

    pub(crate) fn select_previous_n(&mut self, n: usize) {
        let prev = self
            .list_state
            .selected()
            .map_or(0usize, |i: usize| i.saturating_sub(n));
        self.list_state.select(Some(prev));
    }

    pub(crate) fn select_first(&mut self) {
        self.select_line(1);
    }

    pub(crate) fn select_last(&mut self) {
        let len = self.list_len();
        if len > 0 {
            self.list_state.select(Some(len - 1));
        }
    }

    pub(crate) fn select_line(&mut self, line: usize) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let idx = line.saturating_sub(1).min(len - 1);
        self.list_state.select(Some(idx));
    }

    pub(crate) fn page_down(&mut self) {
        let step = (self.viewport_height / 2).max(1) as usize;
        self.select_next_n(step);
    }

    pub(crate) fn page_up(&mut self) {
        let step = (self.viewport_height / 2).max(1) as usize;
        self.select_previous_n(step);
    }

    pub(crate) fn page_forward(&mut self) {
        let step = self.viewport_height.max(1) as usize;
        self.select_next_n(step);
    }

    pub(crate) fn page_back(&mut self) {
        let step = self.viewport_height.max(1) as usize;
        self.select_previous_n(step);
    }

    pub(crate) fn clear_pending(&mut self) {
        self.count_buffer.clear();
        self.pending_key = None;
    }

    pub(crate) fn take_count(&mut self) -> usize {
        self.take_count_or(1)
    }

    pub(crate) fn take_count_or(&mut self, default: usize) -> usize {
        if self.count_buffer.is_empty() {
            default
        } else {
            let count = self.count_buffer.parse().unwrap_or(default);
            self.count_buffer.clear();
            count.max(1)
        }
    }

    pub(crate) fn toggle_selected(&mut self) {
        match self.view {
            View::Scan => self.toggle_scan_selection(),
            View::Languages => self.switch_language(),
            View::Restore => self.request_restore_selected(),
            _ => self.status = self.i18n.t("status_select_scan_only"),
        }
    }

    pub(crate) fn toggle_scan_selection(&mut self) {
        if self.plan.is_none() && !self.entries.is_empty() {
            self.build_plan();
        }
        let Some(plan) = &mut self.plan else {
            self.status = self.i18n.t("status_no_scan_results");
            return;
        };
        let idx = self.list_state.selected().unwrap_or(0);
        let Some(item) = plan.items.get_mut(idx) else {
            return;
        };
        item.selected = !item.selected;
        if item.selected {
            plan.summary.selected_count += 1;
            plan.summary.selected_size_bytes += item.size_bytes;
        } else {
            plan.summary.selected_count -= 1;
            plan.summary.selected_size_bytes -= item.size_bytes;
        }
        let state = if item.selected {
            self.i18n.t("state_selected")
        } else {
            self.i18n.t("state_deselected")
        };
        let path = item.path.display().to_string();
        self.status = self.i18n.format(
            "status_item_toggled",
            &[("path", path), ("state", state.to_string())],
        );
    }

    pub(crate) fn toggle_all_scan_selection(&mut self) {
        if self.view != View::Scan {
            self.status = self.i18n.t("status_select_scan_only");
            return;
        }
        if self.plan.is_none() && !self.entries.is_empty() {
            self.build_plan();
        }
        let Some(plan) = &mut self.plan else {
            self.status = self.i18n.t("status_no_scan_results");
            return;
        };
        let all_selected = plan.items.iter().all(|item| item.selected);
        let target = !all_selected;
        plan.summary.selected_count = 0;
        plan.summary.selected_size_bytes = 0;
        for item in &mut plan.items {
            item.selected = target;
            if target {
                plan.summary.selected_count += 1;
                plan.summary.selected_size_bytes += item.size_bytes;
            }
        }
        self.status = if target {
            self.i18n.t("status_all_toggled_selected")
        } else {
            self.i18n.t("status_all_toggled_deselected")
        };
    }

    pub(crate) fn switch_language(&mut self) {
        let packs = self.i18n.packs();
        if packs.is_empty() {
            return;
        }
        let idx = self.list_state.selected().unwrap_or(0);
        let Some(pack) = packs.get(idx) else {
            return;
        };
        let new_locale = pack.locale.clone();
        self.i18n.set_locale(&new_locale);
        self.config.i18n.locale = Some(new_locale);
        if let Some(path) = default_config_path()
            && let Err(error) = save_config(&self.config, &path)
        {
            self.status = error.to_string();
            return;
        }
        self.status = self.i18n.format(
            "status_language_switched",
            &[("locale", self.i18n.locale().to_string())],
        );
    }

    // ------------------------------------------------------------------
    // Command palette
    // ------------------------------------------------------------------

    pub(crate) fn filtered_palette_commands(&self) -> Vec<cleanr_agent::CommandInfo> {
        self.filtered_palette_commands_for(&self.input)
    }

    pub(crate) fn has_scan_results(&self) -> bool {
        self.scan_rx.is_none() && !self.entries.is_empty()
    }

    pub(crate) fn filtered_palette_commands_for(
        &self,
        input: &str,
    ) -> Vec<cleanr_agent::CommandInfo> {
        filtered_palette_commands(self.has_scan_results(), input, &self.i18n)
    }

    pub(crate) fn clamp_palette_selection(&mut self) {
        let len = self.filtered_palette_commands().len();
        if len == 0 {
            self.palette_state.select(None);
        } else {
            let idx = self.palette_state.selected().unwrap_or(0).min(len - 1);
            self.palette_state.select(Some(idx));
        }
    }

    pub(crate) fn palette_next(&mut self) {
        let len = self.filtered_palette_commands().len();
        if len == 0 {
            return;
        }
        let next = self
            .palette_state
            .selected()
            .map_or(0usize, |i: usize| (i + 1) % len);
        self.palette_state.select(Some(next));
    }

    pub(crate) fn palette_previous(&mut self) {
        let len = self.filtered_palette_commands().len();
        if len == 0 {
            return;
        }
        let prev = self
            .palette_state
            .selected()
            .map_or(0usize, |i: usize| if i == 0 { len - 1 } else { i - 1 });
        self.palette_state.select(Some(prev));
    }
}
