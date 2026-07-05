use super::*;

impl Workbench {
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        self.ime_guard_phase = !self.ime_guard_phase;

        if self.help_open {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?' | 'h' | 'H')) {
                self.help_open = false;
            }
            return;
        }

        if self.confirmation_pending() {
            match key.code {
                KeyCode::Left | KeyCode::Char('h' | 'H' | 'y' | 'Y') => {
                    self.confirm_choice = ConfirmChoice::Yes;
                }
                KeyCode::Right | KeyCode::Char('l' | 'L' | 'n' | 'N') => {
                    self.confirm_choice = ConfirmChoice::No;
                }
                KeyCode::Enter | KeyCode::Char(' ') => self.submit_confirmation(),
                KeyCode::Esc => self.cancel_confirmation(),
                _ => {}
            }
            return;
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Command => self.handle_command_key(key),
        }
    }

    pub(crate) fn handle_normal_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            self.clear_pending();
            return;
        }

        if self.pending_key == Some('g') {
            if let KeyCode::Char('g') = key.code {
                let count = self.take_count_or(1);
                self.select_line(count);
            }
            self.clear_pending();
            return;
        }

        if let KeyCode::Char(d) = key.code
            && d.is_ascii_digit()
        {
            self.count_buffer.push(d);
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                self.clear_pending();
            }
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('H') => {
                if self.is_scan_running() {
                    self.cancel_scan();
                } else {
                    self.go_home();
                }
                self.clear_pending();
            }
            KeyCode::Char('x') | KeyCode::Char('X') if self.is_scan_running() => {
                self.cancel_scan();
                self.clear_pending();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.take_count();
                self.select_next_n(count);
                self.clear_pending();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.take_count();
                self.select_previous_n(count);
                self.clear_pending();
            }
            KeyCode::Char('g') => {
                self.pending_key = Some('g');
            }
            KeyCode::Char('G') => {
                if self.count_buffer.is_empty() {
                    self.select_last();
                } else {
                    let count = self.take_count();
                    self.select_line(count);
                }
                self.clear_pending();
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.toggle_selected();
                self.clear_pending();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.review();
                self.clear_pending();
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.explain_selected_item();
                self.clear_pending();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.start_scan(ScanRequest::default());
                self.clear_pending();
            }
            KeyCode::Char('u') | KeyCode::Char('U')
                if !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.start_usage_scan(ScanRequest::default());
                self.clear_pending();
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                let executor = TrashExecutor;
                self.clean_with_executor(CleanupIntent::UserRequest, &executor);
                self.clear_pending();
            }
            KeyCode::Char('/') => {
                self.open_command('/');
                self.clear_pending();
            }
            KeyCode::Char('?') => {
                self.help_open = true;
                self.clear_pending();
            }
            KeyCode::Char('a') | KeyCode::Char('%') => {
                self.toggle_all_scan_selection();
                self.clear_pending();
            }
            KeyCode::PageDown => {
                self.page_down();
                self.clear_pending();
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_down();
                self.clear_pending();
            }
            KeyCode::PageUp => {
                self.page_up();
                self.clear_pending();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_up();
                self.clear_pending();
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_forward();
                self.clear_pending();
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_back();
                self.clear_pending();
            }
            _ => {
                self.clear_pending();
            }
        }
    }

    pub(crate) fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.close_command(),
            KeyCode::Enter => {
                self.submit_input();
            }
            KeyCode::Backspace => {
                if self.input.len() <= 1 {
                    self.close_command();
                } else {
                    self.input.pop();
                    self.palette_open = self.input.starts_with('/');
                    self.clamp_palette_selection();
                }
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.delete_word_back();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let prefix = self.input.chars().next().unwrap_or('/');
                self.input.clear();
                self.input.push(prefix);
                self.palette_open = prefix == '/';
                self.clamp_palette_selection();
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.push(ch);
                self.palette_open = self.input.starts_with('/');
                self.clamp_palette_selection();
            }
            KeyCode::Up => {
                if self.palette_open {
                    self.palette_previous();
                } else {
                    self.select_previous();
                }
            }
            KeyCode::Down => {
                if self.palette_open {
                    self.palette_next();
                } else {
                    self.select_next();
                }
            }
            KeyCode::Tab | KeyCode::Char('n')
                if self.palette_open
                    && (key.code == KeyCode::Tab
                        || key.modifiers.contains(KeyModifiers::CONTROL)) =>
            {
                self.palette_next();
            }
            KeyCode::BackTab | KeyCode::Char('p')
                if self.palette_open
                    && (key.code == KeyCode::BackTab
                        || key.modifiers.contains(KeyModifiers::CONTROL)) =>
            {
                self.palette_previous();
            }
            _ => {}
        }
    }

    pub(crate) fn open_command(&mut self, prefix: char) {
        self.mode = Mode::Command;
        self.input = String::from(prefix);
        self.palette_open = prefix == '/';
        self.palette_state.select(Some(0));
    }

    pub(crate) fn delete_word_back(&mut self) {
        let prefix_len = 1;
        if self.input.len() <= prefix_len {
            return;
        }
        let rest = &self.input[prefix_len..];
        let mut i = rest.len();
        while i > 0 && rest.as_bytes()[i - 1].is_ascii_whitespace() {
            i -= 1;
        }
        while i > 0 && !rest.as_bytes()[i - 1].is_ascii_whitespace() {
            i -= 1;
        }
        self.input.truncate(prefix_len + i);
        self.palette_open = self.input.starts_with('/');
        self.clamp_palette_selection();
    }

    pub(crate) fn close_command(&mut self) {
        self.input.clear();
        self.mode = Mode::Normal;
        self.palette_open = false;
        self.palette_state.select(None);
        self.clear_pending();
    }

    pub(crate) fn go_home(&mut self) {
        self.view = View::Home;
        self.list_state.select(None);
        self.status = self.i18n.t("status_home");
    }

    pub fn submit_input(&mut self) {
        let input = std::mem::take(&mut self.input);
        self.mode = Mode::Normal;
        self.palette_open = false;
        self.clear_pending();

        if input.trim().is_empty() {
            self.palette_state.select(None);
            return;
        }

        if input.starts_with('/') {
            self.submit_slash_input(&input);
            self.palette_state.select(None);
            return;
        }

        self.palette_state.select(None);
        match self.agent.interpret(&input) {
            Ok(response) => {
                self.status = if response
                    .actions
                    .iter()
                    .any(|action| matches!(action, ActionRequest::Scan(_)))
                {
                    self.i18n.t("status_plain_language_scan_review")
                } else {
                    self.i18n.t("status_plain_language_help")
                };
                for action in response.actions {
                    self.dispatch(action);
                }
            }
            Err(err) => {
                self.status = err.to_string();
            }
        }
    }

    pub(crate) fn submit_slash_input(&mut self, input: &str) {
        if let Ok(action) = parse_slash_command(input) {
            self.status = self.i18n.format(
                "status_queued",
                &[("command", command_name_for_status(&action).to_string())],
            );
            self.dispatch(action);
            return;
        }

        let filtered = self.filtered_palette_commands_for(input);
        if let Some(cmd) = filtered
            .get(self.palette_state.selected().unwrap_or(0))
            .or(filtered.first())
            && let Ok(action) = parse_slash_command(&palette_command_invocation(cmd.name))
        {
            self.status = self.i18n.format(
                "status_queued",
                &[("command", command_name_for_status(&action).to_string())],
            );
            self.dispatch(action);
            return;
        }

        self.status = self.i18n.t("status_help");
    }
}
