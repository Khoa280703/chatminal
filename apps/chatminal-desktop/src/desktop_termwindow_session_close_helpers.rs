impl TermWindow {
    pub(crate) fn close_chatminal_terminal_handle_or_session(
        &mut self,
        _session_id: &str,
        terminal_handle: TerminalUiKey,
    ) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let closed = crate::chatminal_runtime::desktop_close_session_terminal_handle_or_session(
            self.window_id as DesktopWindowId,
            crate::chatminal_runtime::SessionTerminalHandle::new(terminal_handle),
        );
        if closed {
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
            self.sync_active_chatminal_session_from_mux();
        }
        closed
    }

    fn create_chatminal_session(&mut self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        match self.chatminal_sidebar.create_session(
            self.terminal_size.cols.max(20),
            self.terminal_size.rows.max(5),
        ) {
            Ok(created) => {
                if let Some(window) = self.window.as_ref() {
                    window.invalidate();
                }
                self.switch_chatminal_session(&created.session_id);
            }
            Err(err) => {
                log::error!("failed to create sidebar session: {err}");
            }
        }
    }

}
