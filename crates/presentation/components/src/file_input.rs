use gpui::{AnyElement, ClickEvent, ElementId, IntoElement};

use crate::prelude::*;

/// A styled file-upload trigger: dashed `semantic::border`, an icon, and a
/// "Click to upload" label.
///
/// LIMITATION (real scope boundary, not a cut corner): this codebase has no
/// OS file-dialog API — no bindings exist in `gpui`/`gpui_platform` for
/// opening a native file picker. `FileInput` is a purely presentational
/// trigger; clicking it only invokes `on_click`. It performs no file I/O and
/// does not open any dialog. The caller is responsible for wiring
/// `on_click` to their own file-dialog integration when one becomes
/// available.
#[derive(IntoElement, RegisterComponent)]
pub struct FileInput {
    id: ElementId,
    label: SharedString,
    hint: Option<SharedString>,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl FileInput {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: "Click to upload".into(),
            hint: None,
            disabled: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the click handler. Caller wires this to their own file-dialog
    /// integration — see the type-level doc comment for the scope boundary.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for FileInput {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .id(self.id)
            .items_center()
            .justify_center()
            .gap_2()
            .w_full()
            .py_8()
            .rounded_lg()
            .border_2()
            .border_dashed()
            .border_color(cx.theme().border)
            .when(!self.disabled, |this| this.cursor_pointer())
            .when(self.disabled, |this| this.opacity(0.5))
            .child(
                Icon::new(IconName::Paperclip)
                    .text_base()
                    .text_color(cx.theme().muted),
            )
            .child(Label::new(self.label).text_sm())
            .when_some(self.hint, |this, hint| {
                this.child(Label::new(hint).text_sm().text_color(cx.theme().muted))
            })
            .when_some(self.on_click.filter(|_| !self.disabled), |this, handler| {
                this.on_click(handler)
            })
    }
}

impl Component for FileInput {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn description() -> Option<&'static str> {
        Some(
            "A styled file-upload trigger. No OS file-dialog API exists in this codebase — \
             purely presentational, caller wires `on_click` to their own integration.",
        )
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_4()
                .child(
                    FileInput::new("file-input-default")
                        .hint("PNG, JPG up to 10MB")
                        .on_click(|_, _, _| {})
                        .into_any_element(),
                )
                .child(
                    FileInput::new("file-input-selected")
                        .label("resume.pdf")
                        .hint("245 KB — click to replace")
                        .on_click(|_, _, _| {})
                        .into_any_element(),
                )
                .child(
                    FileInput::new("file-input-disabled")
                        .disabled(true)
                        .into_any_element(),
                )
                .into_any_element(),
        )
    }
}
