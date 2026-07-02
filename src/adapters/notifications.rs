//! Notification helpers for showing toast messages.

use gpui::*;
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::WindowExt;

/// Show a success notification.
pub fn show_success(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        (NotificationType::Success, message.to_string()),
        cx,
    );
}

/// Show an error notification.
pub fn show_error(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        (NotificationType::Error, message.to_string()),
        cx,
    );
}

/// Show a warning notification.
pub fn show_warning(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        (NotificationType::Warning, message.to_string()),
        cx,
    );
}

/// Show an info notification.
pub fn show_info(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        (NotificationType::Info, message.to_string()),
        cx,
    );
}

/// Show a success notification with auto-hide.
pub fn show_success_autohide(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        Notification::new()
            .message(message)
            .with_type(NotificationType::Success)
            .autohide(true),
        cx,
    );
}

/// Show an error notification without auto-hide.
pub fn show_error_persistent(window: &mut Window, cx: &mut App, message: &str) {
    window.push_notification(
        Notification::new()
            .message(message)
            .with_type(NotificationType::Error)
            .autohide(false),
        cx,
    );
}
