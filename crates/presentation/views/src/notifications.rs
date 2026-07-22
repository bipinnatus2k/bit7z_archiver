use gpui::*;
use gpui_component::WindowExt;
use gpui_component::notification::{Notification, NotificationType};

pub fn show_success(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification((NotificationType::Success, message.to_string()), cx);
}

pub fn show_error(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification((NotificationType::Error, message.to_string()), cx);
}

pub fn show_warning(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification((NotificationType::Warning, message.to_string()), cx);
}

pub fn show_info(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification((NotificationType::Info, message.to_string()), cx);
}

pub fn show_success_autohide(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        Notification::new()
            .message(message)
            .with_type(NotificationType::Success)
            .autohide(true),
        cx,
    );
}

pub fn show_error_persistent(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        Notification::new()
            .message(message)
            .with_type(NotificationType::Error)
            .autohide(false),
        cx,
    );
}
