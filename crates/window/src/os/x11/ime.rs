use xcb::x::Window;

pub struct ImeClient {
    inner: ImeClientInner,
}

impl ImeClient {
    pub fn new(conn: &xcb::Connection, screen_num: i32) -> Self {
        Self {
            inner: ImeClientInner::new(conn, screen_num),
        }
    }

    pub fn set_commit_string_cb<F>(&mut self, callback: F)
    where
        F: for<'a> FnMut(Window, &'a str) + 'static,
    {
        self.inner.set_commit_string_cb(callback);
    }

    pub fn set_preedit_draw_cb<F>(&mut self, callback: F)
    where
        F: for<'a> FnMut(Window, &'a str) + 'static,
    {
        self.inner.set_preedit_draw_cb(callback);
    }

    pub fn set_preedit_done_cb<F>(&mut self, callback: F)
    where
        F: FnMut(Window) + 'static,
    {
        self.inner.set_preedit_done_cb(callback);
    }

    pub fn set_forward_event_cb<F>(&mut self, callback: F)
    where
        F: for<'a> FnMut(Window, &'a xcb::Event) + 'static,
    {
        self.inner.set_forward_event_cb(callback);
    }

    pub fn process_event(&mut self, event: &xcb::Event) -> bool {
        self.inner.process_event(event)
    }

    pub fn update_pos(&mut self, window_id: Window, x: i16, y: i16) {
        self.inner.update_pos(window_id, x, y);
    }
}

struct ImeClientInner;

impl ImeClientInner {
    fn new(_conn: &xcb::Connection, _screen_num: i32) -> Self {
        log::warn!(
            "X11 IME support is disabled in Chatminal build/runtime; continuing without IME"
        );
        Self
    }

    fn set_commit_string_cb<F>(&mut self, _callback: F)
    where
        F: for<'a> FnMut(Window, &'a str) + 'static,
    {
    }

    fn set_preedit_draw_cb<F>(&mut self, _callback: F)
    where
        F: for<'a> FnMut(Window, &'a str) + 'static,
    {
    }

    fn set_preedit_done_cb<F>(&mut self, _callback: F)
    where
        F: FnMut(Window) + 'static,
    {
    }

    fn set_forward_event_cb<F>(&mut self, _callback: F)
    where
        F: for<'a> FnMut(Window, &'a xcb::Event) + 'static,
    {
    }

    fn process_event(&mut self, _event: &xcb::Event) -> bool {
        false
    }

    fn update_pos(&mut self, _window_id: Window, _x: i16, _y: i16) {}
}
