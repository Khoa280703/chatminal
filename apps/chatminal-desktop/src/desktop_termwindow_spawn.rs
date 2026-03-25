impl TermWindow {
    pub fn spawn_command(
        &self,
        spawn: &SpawnCommand,
        spawn_where: crate::spawn::SpawnWhere,
    ) {
        let size = if spawn_where == crate::spawn::SpawnWhere::NewWindow {
            self.config.initial_size(
                self.dimensions.dpi as u32,
                crate::cell_pixel_dims(&self.config, self.dimensions.dpi as f64).ok(),
            )
        } else {
            self.terminal_size
        };
        let term_config = Arc::new(TermConfig::with_config(self.config.clone()));

        crate::spawn::spawn_command_impl(
            spawn,
            spawn_where,
            size,
            Some(self.window_id as crate::scripting::guiwin::DesktopWindowId),
            term_config,
        )
    }

    pub fn spawn_runtime_entry(&mut self) {
        if self.chatminal_sidebar.is_enabled() {
            self.create_chatminal_session();
            return;
        }
        self.spawn_command(
            &SpawnCommand {
                ..Default::default()
            },
            crate::spawn::SpawnWhere::NewSession,
        );
    }
}
