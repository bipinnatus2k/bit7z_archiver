use crate::title_bar::AppTitleBar;
use crate::{About, OpenSettings, ShowNotificationInfo, ToggleSearch};
use gpui::{
    div, AnyView, App, AppContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};
use gpui_component::notification::Notification;
use gpui_component::{v_flex, Root, WindowExt};
use bit7z_pres_dialogs::settings::SettingsDialog;

//只处理与窗体、App整体有关的action，比如弹通知，更新menu等等
pub struct StoryRoot {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) title_bar: Entity<AppTitleBar>,
    pub(crate) view: AnyView,
}

impl StoryRoot {
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        Self {
            focus_handle: cx.focus_handle(),
            title_bar,
            view: view.into(),
        }
    }

    fn on_action_notify_info(
        &mut self,
        notification: &ShowNotificationInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        struct Info;
        let note = Notification::new()
            .message(&notification.message.clone())
            .id::<Info>();
        window.push_notification(note, cx);
    }

    fn on_action_open_settings(
        &mut self,
        _:&OpenSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |_, cx| { 
            SettingsDialog::open(cx); 
        }).detach();
        
    }

    // fn on_action_about(&mut self, _: &About, window: &mut Window, cx: &mut Context<Self>) {
    //     if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
    //         cx.defer(move |cx| {
    //             window
    //                 .update(cx, |_, window, cx| {
    //                     window.defer(cx, |window, cx| {
    //                         bit7z_pres_dialogs::about::AboutDialog::open(cx);
    //                     });
    //                 })
    //                 .unwrap();
    //         });
    //     }
    // }

    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note = Notification::new()
            .message("You have toggled search.")
            .id::<Search>();
        window.push_notification(note, cx);
    }
}

impl Focusable for StoryRoot {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StoryRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .id("story-root")
            .on_action(cx.listener(Self::on_action_notify_info))
            .on_action(cx.listener(Self::on_action_toggle_search))
            .on_action(cx.listener(Self::on_action_open_settings))
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.title_bar.clone())
                    .child(
                        div()
                            .track_focus(&self.focus_handle)
                            .flex_1()
                            .overflow_hidden()
                            .child(self.view.clone()),
                    )
                    .children(sheet_layer)
                    .children(dialog_layer)
                    .children(notification_layer),
            )
    }
}
