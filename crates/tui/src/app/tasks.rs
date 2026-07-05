use super::*;

impl Workbench {
    pub fn poll_tasks(&mut self) {
        self.animation_tick = self.animation_tick.wrapping_add(1);

        if let Some(rx) = self.scan_rx.take() {
            loop {
                match rx.try_recv() {
                    Ok(TaskEvent::ScanProgress(progress)) => {
                        self.status = self.scan_progress_status(&progress);
                        self.scan_progress = Some(progress);
                    }
                    Ok(TaskEvent::ScanFinished(Ok(mut report))) => {
                        self.scan_cancel = None;
                        self.registry.annotate_entries(&mut report.entries);
                        self.scan_summary = report.summary;
                        self.entries = report.entries;
                        self.plan = None;
                        self.scan_progress = None;
                        self.task_log.push(self.i18n.format(
                            "status_scan_log",
                            &[
                                ("entries", self.scan_summary.entries_seen.to_string()),
                                ("errors", self.scan_summary.errors.to_string()),
                            ],
                        ));
                        self.status = self.i18n.format(
                            "status_scan_finished",
                            &[
                                ("entries", self.scan_summary.entries_seen.to_string()),
                                (
                                    "candidates",
                                    self.entries
                                        .iter()
                                        .filter(|entry| !entry.rule_hits.is_empty())
                                        .count()
                                        .to_string(),
                                ),
                            ],
                        );
                        let activate_scan = self.view == View::Scan || self.review_after_scan;
                        self.review_after_scan = false;
                        let usage_after_scan = self.usage_after_scan;
                        self.usage_after_scan = false;
                        if !self.entries.is_empty() {
                            self.build_plan_for_view(activate_scan);
                        }
                        if usage_after_scan {
                            self.show_usage();
                        } else if self.view == View::Scan {
                            self.select_first();
                        }
                        break;
                    }
                    Ok(TaskEvent::ScanFinished(Err(err))) => {
                        self.scan_cancel = None;
                        self.status = if err.contains(SCAN_CANCELLED) {
                            self.i18n.t("status_scan_cancelled")
                        } else {
                            err
                        };
                        self.scan_progress = None;
                        self.review_after_scan = false;
                        self.usage_after_scan = false;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        self.scan_rx = Some(rx);
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.scan_cancel = None;
                        self.status = self.i18n.t("status_scan_disconnected");
                        self.scan_progress = None;
                        self.review_after_scan = false;
                        self.usage_after_scan = false;
                        break;
                    }
                }
            }
        }

        if let Some(rx) = self.insight_rx.take() {
            loop {
                match rx.try_recv() {
                    Ok(InsightEvent::Finished(Ok(insight))) => {
                        self.insight.state = InsightState::Ready(insight);
                    }
                    Ok(InsightEvent::Finished(Err(err))) => {
                        self.insight.state = InsightState::Error(err);
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        self.insight_rx = Some(rx);
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }
    }

    pub(crate) fn start_scan(&mut self, request: ScanRequest) {
        self.start_scan_for_view(request, View::Scan);
    }

    pub(crate) fn start_scan_for_view(&mut self, mut request: ScanRequest, view: View) {
        if self.scan_rx.is_some() {
            self.status = self.i18n.t("status_scan_already_running");
            return;
        }
        if request.paths.is_empty() && !request.include_global {
            request.paths = self.roots.clone();
        }
        let resolved = match resolve_scan_roots(&request, &self.config.scan.global_kinds) {
            Ok(resolved) => resolved,
            Err(error) => {
                self.status = if error.to_string().contains(NO_GLOBAL_SCAN_ROOTS) {
                    self.i18n.t("status_no_global_caches")
                } else {
                    error.to_string()
                };
                return;
            }
        };
        if resolved.roots.is_empty() {
            self.status = self.i18n.t("status_no_global_caches");
            return;
        }
        self.roots = resolved.roots;

        let roots = self.roots.clone();
        let options = ScanOptions {
            stay_on_filesystem: self.config.scan.stay_on_filesystem,
            ignore_dirs: self.config.scan.ignore_dirs.clone(),
            ignore_patterns: self.config.scan.ignore_patterns.clone(),
        };
        let effect = match spawn_scan(roots, options) {
            Ok(effect) => effect,
            Err(error) => {
                self.status = error.to_string();
                return;
            }
        };
        self.scan_rx = Some(effect.receiver);
        self.scan_cancel = Some(effect.cancellation);
        self.scan_progress = Some(ScanProgress {
            phase: ScanPhase::Scanning,
            entries_total: 0,
            entries_scanned: 0,
            bytes_scanned: 0,
            errors: 0,
            current_path: self.roots.first().cloned(),
        });
        self.entries.clear();
        self.scan_summary = ScanSummary::default();
        self.plan = None;
        self.view = view;
        self.list_state.select(None);
        self.status = self.i18n.format(
            "status_scanning",
            &[("roots", self.roots.len().to_string())],
        );
        self.task_log.push(self.i18n.t("status_scan_started"));
    }

    pub(crate) fn cancel_scan(&mut self) {
        if let Some(cancel) = &self.scan_cancel {
            cancel.store(true, Ordering::Relaxed);
            self.status = self.i18n.t("status_scan_cancelling");
        }
    }

    pub(crate) fn start_usage_scan(&mut self, request: ScanRequest) {
        if self.scan_rx.is_some() {
            self.status = self.i18n.t("status_scan_already_running");
            return;
        }
        self.usage_after_scan = true;
        self.start_scan_for_view(request, View::Usage);
    }

    pub(crate) fn scan_progress_status(&self, progress: &ScanProgress) -> String {
        let phase = self.scan_phase_label(progress.phase);
        if progress.phase == ScanPhase::Scanning && progress.entries_total == 0 {
            return self.i18n.format(
                "status_scan_progress_unbounded",
                &[
                    ("phase", phase),
                    ("scanned", progress.entries_scanned.to_string()),
                    ("size", format_bytes(progress.bytes_scanned)),
                ],
            );
        }
        self.i18n.format(
            "status_scan_progress",
            &[
                ("phase", phase),
                ("scanned", progress.entries_scanned.to_string()),
                ("total", progress.entries_total.to_string()),
                ("size", format_bytes(progress.bytes_scanned)),
            ],
        )
    }

    pub(crate) fn scan_phase_label(&self, phase: ScanPhase) -> String {
        let key = match phase {
            ScanPhase::Discovering => "scan_phase_discovering",
            ScanPhase::Scanning => "scan_phase_scanning",
            ScanPhase::Aggregating => "scan_phase_aggregating",
        };
        self.i18n.t(key)
    }

    pub(crate) fn explain_selected_item(&mut self) {
        let Some(plan) = &self.plan else {
            self.status = self.i18n.t("status_select_scan_only");
            return;
        };
        let Some(idx) = self.list_state.selected() else {
            return;
        };
        let Some(item) = plan.items.get(idx) else {
            return;
        };

        self.insight.target = Some(item.path.clone());
        self.insight.state = InsightState::Loading;

        let path = item.path.clone();
        let context = PathContext {
            size_bytes: item.size_bytes,
            parent_path: item.path.parent().map(PathBuf::from),
            rule_id: Some(item.rule_id.clone()),
            reason: Some(item.reason.clone()),
        };
        if let Err(error) = spawn_insight(
            self.config.agent.clone(),
            path,
            context,
            self.insight_tx.clone(),
        ) {
            self.insight.state = InsightState::Error(error.to_string());
        }
    }

    pub(crate) fn refresh_history(&mut self) {
        match load_history(&self.state_dir) {
            Ok((execution_manifests, restore_manifests)) => {
                self.execution_manifests = execution_manifests;
                self.restore_manifests = restore_manifests;
            }
            Err(error) => {
                self.execution_manifests.clear();
                self.restore_manifests.clear();
                self.status = error.to_string();
            }
        }
    }
}
