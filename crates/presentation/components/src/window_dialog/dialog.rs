use gpui::{
    AnyElement, App, AppContext, AsyncApp, Bounds, FocusHandle, Focusable, IntoElement,
    ParentElement, Pixels, Render, SharedString, Size, Styled, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, px, size,
    prelude::FluentBuilder,
};
use gpui_component::{Root, TitleBar, v_flex};

// ---------------------------------------------------------------------------
// Re-export gpui-component's dialog sub-components for convenience.
// ---------------------------------------------------------------------------
pub use gpui_component::dialog::{
    DialogAction, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader,
    DialogTitle,
};

// ---------------------------------------------------------------------------
// Close action
// ---------------------------------------------------------------------------

/// Controls how the dialog window is closed.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum CloseAction {
    /// Call `window.remove_window()`.
    RemoveWindow,
    /// Call `window.close_dialog()` via gpui-component's dialog system.
    CloseDialog,
}

// ---------------------------------------------------------------------------
// Window options
// ---------------------------------------------------------------------------

/// Configuration for opening a dialog in an independent window.
#[derive(Clone)]
pub struct WindowDialogOptions {
    pub title: SharedString,
    pub width: Pixels,
    pub height: Option<Pixels>,
    pub min_width: Option<Pixels>,
    pub min_height: Option<Pixels>,
    pub kind: WindowKind,
    pub close_action: CloseAction,
    pub window_decorations: Option<WindowDecorations>,
    pub window_background: WindowBackgroundAppearance,
}

impl Default for WindowDialogOptions {
    fn default() -> Self {
        Self {
            title: SharedString::default(),
            width: px(480.),
            height: None,
            min_width: Some(px(320.)),
            min_height: Some(px(200.)),
            kind: WindowKind::Dialog,
            close_action: CloseAction::CloseDialog,
            window_decorations: Some(WindowDecorations::Client),
            window_background: WindowBackgroundAppearance::Opaque,
        }
    }
}

impl WindowDialogOptions {
    /// Compute the dialog window size.
    pub fn dialog_size(&self) -> Size<Pixels> {
        size(self.width, self.height.unwrap_or(self.width * 0.75))
    }

    pub fn build_window_options(self, bounds: WindowBounds) -> WindowOptions {
        let mut opts = WindowOptions {
            window_bounds: Some(bounds),
            titlebar: Some(TitleBar::title_bar_options()),
            window_background: self.window_background,
            window_decorations: self.window_decorations,
            kind: self.kind,
            focus: true,
            ..Default::default()
        };

        if let (Some(w), Some(h)) = (self.min_width, self.min_height) {
            opts.window_min_size = Some(Size { width: w, height: h });
        }

        opts
    }
}

// ---------------------------------------------------------------------------
// Entity that owns and renders the dialog layout inside its own window.
// ---------------------------------------------------------------------------

struct WindowDialogEntity {
    focus_handle: FocusHandle,
    header: Option<AnyElement>,
    footer: Option<AnyElement>,
    children: Vec<AnyElement>,
}

impl Focusable for WindowDialogEntity {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WindowDialogEntity {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let p = px(16.);

        v_flex()
            .size_full()
            .pt(p)
            .pb(p)
            .gap(p.max(px(8.)))
            .when_some(self.header.take(), |this, h| this.child(div().px(p).child(h)))
            .child(div().flex_1().overflow_hidden().px(p).children(std::mem::take(&mut self.children)))
            .when_some(self.footer.take(), |this, f| this.child(div().px(p).child(f)))
    }
}

// ---------------------------------------------------------------------------
// Builder handle passed to the user's closure.
// ---------------------------------------------------------------------------

/// A temporary handle provided to the `build` closure of
/// [`open_window_dialog`].  Populate header, footer, and content children.
pub struct WindowDialogHandle<'a> {
    header: &'a mut Option<AnyElement>,
    footer: &'a mut Option<AnyElement>,
    children: &'a mut Vec<AnyElement>,
}

impl<'a> WindowDialogHandle<'a> {
    pub fn header(&mut self, el: impl IntoElement) -> &mut Self {
        *self.header = Some(el.into_any_element());
        self
    }

    pub fn footer(&mut self, el: impl IntoElement) -> &mut Self {
        *self.footer = Some(el.into_any_element());
        self
    }

    pub fn child(&mut self, el: impl IntoElement) -> &mut Self {
        self.children.push(el.into_any_element());
        self
    }

    pub fn children(&mut self, els: impl IntoIterator<Item: IntoElement>) -> &mut Self {
        self.children.extend(els.into_iter().map(|e| e.into_any_element()));
        self
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Open a dialog in an independent window.
///
/// The `build` closure receives a [`WindowDialogHandle`] for setting header,
/// footer, and content children.  The window is created with the given
/// [`WindowDialogOptions`].
///
/// # Example
///
/// ```ignore
/// open_window_dialog(cx, WindowDialogOptions::default().title("About"), |dlg, _, cx| {
///     dlg.header(DialogHeader::new().child(DialogTitle::new().child("bit7z Archiver")))
///        .footer(DialogFooter::new().child(Button::new("ok").label("OK")));
/// });
/// ```
pub fn open_window_dialog<F>(
    cx: &mut AsyncApp,
    options: WindowDialogOptions,
    build: F,
) where
    F: FnOnce(&mut WindowDialogHandle, &mut Window, &mut App) + 'static,
{
    let title = options.title.clone();

    cx.spawn(async move |cx| {
        let dialog_size = options.dialog_size();
        let bounds = cx.update(|app| WindowBounds::Windowed(Bounds::centered(None, dialog_size, app)));
        let win_opts = options.clone().build_window_options(bounds);

        let _ = cx.open_window(win_opts, move |window, cx| {
            window.set_window_title(&title);

            // Build dialog content into temporary buffers first,
            // then move them into the entity (avoids needing &mut App
            // inside cx.new()).
            let mut header: Option<AnyElement> = None;
            let mut footer: Option<AnyElement> = None;
            let mut children: Vec<AnyElement> = Vec::new();

            let mut handle = WindowDialogHandle {
                header: &mut header,
                footer: &mut footer,
                children: &mut children,
            };
            build(&mut handle, window, cx);

            let entity = cx.new(|cx| WindowDialogEntity {
                focus_handle: cx.focus_handle(),
                header,
                footer,
                children,
            });

            cx.new(|cx| Root::new(entity, window, cx))
        });
    }).detach();
}
