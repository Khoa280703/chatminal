use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Length};

use crate::message::Message;
use crate::session::SessionId;
use crate::ui::theme::SESSION_ITEM_HEIGHT;

pub fn sidebar_view<'a>(
    sessions: &[(SessionId, String)],
    active_id: Option<SessionId>,
    sidebar_width: f32,
) -> Element<'a, Message> {
    let mut list = column![];

    for (idx, (id, name)) in sessions.iter().enumerate() {
        let prefix = if Some(*id) == active_id { "●" } else { "○" };
        let label = format!("{prefix} {}. {name}", idx + 1);

        let item = button(text(label))
            .width(Length::Fill)
            .height(Length::Fixed(SESSION_ITEM_HEIGHT))
            .on_press(Message::SelectSession(*id));

        list = list.push(item);
    }

    let list = scrollable(list.spacing(4));

    let footer = column![
        button(text("+ New Session")).on_press(Message::NewSession),
        text("Alt+N New • Alt+W Close").size(12),
    ]
    .spacing(8)
    .padding([8, 0]);

    container(column![list.height(Length::Fill), footer].spacing(8))
        .width(Length::Fixed(sidebar_width))
        .height(Length::Fill)
        .padding(8)
        .into()
}
