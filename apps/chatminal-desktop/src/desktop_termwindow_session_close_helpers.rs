impl TermWindow {
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
