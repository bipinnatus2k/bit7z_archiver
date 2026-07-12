use gpui::{
    App, AppContext, AsyncApp, Bounds, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, SharedString, Size, Styled, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, px,
    size,
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

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum CloseAction {
    RemoveWindow,
    CloseDialog,
}

// ---------------------------------------------------------------------------
// Window options
// ---------------------------------------------------------------------------

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
            window_decorations: {
                #[cfg(target_os = "linux")]
                { Some(WindowDecorations::Client) }
                #[cfg(not(target_os = "linux"))]
                { None }
            },
            window_background: WindowBackgroundAppearance::Opaque,
        }
    }
}

impl WindowDialogOptions {
    pub fn dialog_size(&self) -> Size<Pixels> {
        size(self.width, self.height.unwrap_or(self.width * 0.75))
    }

    pub fn centered_bounds(&self, size: Size<Pixels>, cx: &App) -> WindowBounds {
        WindowBounds::Windowed(Bounds::centered(None, size, cx))
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
// WindowDialogEntity — wraps a content Entity in a dialog card.
//
// Renders the same visual card as gpui-component's Dialog (background,
// border, rounded corners, padding) but WITHOUT the overlay, anchored
// positioning, and animations that only make sense for same-window
// modal overlays.
//
// The content entity is rendered fresh every frame via Entity::clone().
// ---------------------------------------------------------------------------

const DIALOG_CTX: &str = "WindowDialog";

gpui::actions!(window_dialog, [CloseWindowDialog, ConfirmWindowDialog]);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("escape", CloseWindowDialog, Some(DIALOG_CTX)),
        gpui::KeyBinding::new("enter", ConfirmWindowDialog, Some(DIALOG_CTX)),
    ]);
}

struct WindowDialogEntity<E: Render> {
    focus_handle: FocusHandle,
    content: Entity<E>,
}

impl<E: Render> Focusable for WindowDialogEntity<E> {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl<E: Render> Render for WindowDialogEntity<E> {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let p = px(16.);

        // No card wrapper — the window frame itself provides the boundary.
        // The content entity fills the window directly with uniform padding.
        v_flex()
            .id("window-dialog")
            .size_full()
            .key_context(DIALOG_CTX)
            .track_focus(&self.focus_handle)
            .p(p)
            .child(self.content.clone())
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Open a dialog in an independent window.
///
/// The `build` closure receives the new window's [`Window`] and [`App`] and
/// returns a content entity.  That entity is rendered each frame inside a
/// dialog card (border, rounded corners, background) without any overlay
/// or anchored positioning.
///
/// # Example
///
/// ```ignore
/// open_window_dialog(cx, opts, |_window, cx| cx.new(|_| AboutContent));
/// ```
pub fn open_window_dialog<E, F>(
    cx: &mut App,
    options: WindowDialogOptions,
    build: F,
) where
    E: Render + 'static,
    F: FnOnce(&mut Window, &mut App) -> Entity<E> + 'static,
{
    let title = options.title.clone();
    let dialog_size = options.dialog_size();
    let bounds = options.centered_bounds(dialog_size, cx);
    let win_opts = options.build_window_options(bounds);

    let _ = cx.open_window(win_opts, move |window, cx| {
        window.set_window_title(&title);
        let content = build(window, cx);
        entity_in_window(content, window, cx)
    });
}

/// Same as [`open_window_dialog`] but accepts `&mut AsyncApp`.
pub fn open_window_dialog_async<E, F>(
    cx: &mut AsyncApp,
    options: WindowDialogOptions,
    build: F,
) where
    E: Render + 'static,
    F: FnOnce(&mut Window, &mut App) -> Entity<E> + 'static,
{
    cx.spawn(async move |cx| {
        let dialog_size = options.dialog_size();
        let bounds = cx.update(|app| options.centered_bounds(dialog_size, app));
        let title = options.title.clone();
        let win_opts = options.build_window_options(bounds);

        let _ = cx.open_window(win_opts, move |window, cx| {
            window.set_window_title(&title);
            let content = build(window, cx);
            entity_in_window(content, window, cx)
        });
    }).detach();
}

fn entity_in_window<E: Render>(
    content: Entity<E>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Entity<gpui_component::Root> {
    let entity = cx.new(|cx| WindowDialogEntity {
        focus_handle: cx.focus_handle(),
        content,
    });
    cx.new(|cx| Root::new(entity, window, cx))
}
