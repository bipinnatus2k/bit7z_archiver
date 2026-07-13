use crate::app_shell::AppShell;
use crate::archive_browser::{ArchiveBrowser, BrowserIntent};
use crate::archive_file_list::{ArchiveFileList, FileListIntent};
use crate::menu::{self, Menu};
use crate::preview_panel::PreviewPanel;
use crate::status_bar::StatusBar;
use crate::toolbar::{Toolbar, ToolbarIntent};
use bit7z_pres_view_models::intent::Intent;
use bit7z_pres_view_models::AppState;
use bit7z_infra_events::ArchiveVmEvent;
use gpui::*;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_component::{Root, v_flex};

pub struct RootView {
    pub app_shell: WeakEntity<AppShell>,
    pub state: AppState,
    focus_handle: FocusHandle,
    pub menu: Entity<Menu>,
    pub toolbar: Entity<Toolbar>,
    pub archive_browser: Entity<ArchiveBrowser>,
    pub entry_list: Entity<ArchiveFileList>,
    pub preview_panel: Entity<PreviewPanel>,
    pub status_bar: Entity<StatusBar>,
}

impl RootView {
    pub fn new(
        app_shell: WeakEntity<AppShell>,
        state: AppState,
        preview_panel: Entity<PreviewPanel>,
        archive_browser: Entity<ArchiveBrowser>,
        entry_list: Entity<ArchiveFileList>,
        cx: &mut Context<Self>,
    ) -> Self {
        let menu = cx.new(|cx| Menu::new(state.clone(), cx));
        let toolbar = cx.new(|_| Toolbar::new(state.clone()));
        let status_bar = cx.new(|_| StatusBar::new(state.clone()));

        cx.subscribe::<Toolbar, ToolbarIntent>(&toolbar, move |_, _, intent, cx| {
            match intent {
                ToolbarIntent::OpenArchive => cx.emit(Intent::RequestOpenArchive),
                ToolbarIntent::CreateArchive => cx.emit(Intent::RequestCreateArchive),
                ToolbarIntent::AddFiles => cx.emit(Intent::RequestAddFiles),
                ToolbarIntent::ExtractSelected => cx.emit(Intent::Extract),
                ToolbarIntent::TestArchive => cx.emit(Intent::TestAll),
                ToolbarIntent::CloseArchive => cx.emit(Intent::CloseArchive),
                ToolbarIntent::ShowSettings => { cx.spawn(async move |_, cx| { bit7z_pres_dialogs::settings::SettingsDialog::open(cx); }).detach(); }
            }
        }).detach();

        let pp = preview_panel.clone();
        cx.subscribe::<ArchiveBrowser, BrowserIntent>(&archive_browser, move |this, _, intent, cx| {
            match intent {
                BrowserIntent::NavigateInto(dir) => { if let Some(s) = this.app_shell.upgrade() { s.update(cx, |s, cx| s.navigate_into(dir, cx)); } }
                BrowserIntent::OpenRecentFile(path) => { if let Some(s) = this.app_shell.upgrade() { s.update(cx, |s, cx| s.handle_open_archive(std::path::Path::new(path), None, cx)); } }
            }
        }).detach();

        cx.subscribe::<ArchiveFileList, FileListIntent>(&entry_list, move |this, _, intent, cx| {
            match intent {
                FileListIntent::SelectionChanged(indices) => {
                    this.state.selection.update(|s| { *s = indices.iter().copied().collect(); });
                    this.state.selection_anchor.set(None);
                    if let (Some(idx), Some(s)) = (indices.first(), this.app_shell.upgrade()) {
                        let h = s.read(cx).state.handle.get();
                        let panel = pp.clone();
                        let idx = *idx;
                        let uc = s.read(cx).use_cases.clone();
                        cx.spawn(async move |this, cx| {
                            panel.update(cx, |p, _| p.set_loading());
                            if let Some(ref handle) = h {
                                match uc.preview(handle, idx, 1_048_576) {
                                    Ok(data) => panel.update(cx, |p, _| p.set_data(Some(data))),
                                    Err(_) => panel.update(cx, |p, _| p.set_data(None)),
                                }
                            }
                            let _ = this.update(cx, |_, cx| cx.notify());
                        }).detach();
                    }
                }
                FileListIntent::SortByColumn(col, asc) => { this.state.apply_sort(*col, *asc); }
                FileListIntent::NavigateUp => { this.state.navigate_up(); if let Some(s) = this.app_shell.upgrade() { s.update(cx, |s, cx| s.load_current_directory(cx)); } }
                FileListIntent::OpenEntry => cx.emit(Intent::OpenEntry),
                FileListIntent::PreviewEntry => {},
                FileListIntent::ExtractSelected => cx.emit(Intent::Extract),
                FileListIntent::TestSelected => cx.emit(Intent::TestSelected),
                FileListIntent::RenameEntry(_) => {},
                FileListIntent::DeleteSelected => cx.emit(Intent::DeleteSelected),
                FileListIntent::Checksum(_) => {},
                FileListIntent::SelectAll => { this.state.select_all(); }
                FileListIntent::ClearSelection => { this.state.clear_selection(); }
                FileListIntent::Refresh => cx.emit(Intent::Refresh),
                FileListIntent::ShowProperties => cx.emit(Intent::ShowProperties),
            }
        }).detach();

        let focus_handle = cx.focus_handle();
        Self { app_shell, state, focus_handle, menu, toolbar, archive_browser, entry_list, preview_panel, status_bar }
    }

}

impl Focusable for RootView { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus_handle.clone() } }
impl EventEmitter<Intent> for RootView {}
impl EventEmitter<ArchiveVmEvent> for RootView {}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().relative()
            .on_key_down(cx.listener(|this: &mut Self, event: &gpui::KeyDownEvent, _: &mut Window, cx: &mut Context<Self>| {
                let key = event.keystroke.key.as_str();
                let cmd = event.keystroke.modifiers.platform || event.keystroke.modifiers.control;
                let shift = event.keystroke.modifiers.shift;
                let alt = event.keystroke.modifiers.alt;
                match key {
                    "a" if cmd && !shift => { this.state.select_all(); }
                    "o" if cmd => cx.emit(Intent::RequestOpenArchive),
                    "n" if cmd && shift => cx.emit(Intent::RequestNewFolder),
                    "n" if cmd => cx.emit(Intent::RequestCreateArchive),
                    "e" if cmd => cx.emit(Intent::Extract),
                    "t" if cmd => cx.emit(Intent::TestAll),
                    "d" if cmd || key == "Backspace" || key == "Delete" => cx.emit(Intent::DeleteSelected),
                    "enter" if alt => cx.emit(Intent::ShowProperties),
                    "enter" => cx.emit(Intent::OpenEntry),
                    "f5" => cx.emit(Intent::Refresh),
                    "f4" => cx.emit(Intent::RequestNewFile),
                    "f2" => { if let Some(idx) = this.state.first_selected_index() { cx.emit(ArchiveVmEvent::RequestRename { index: idx, new_name: String::new() }); } }
                    _ => {}
                }
            }))
            .on_action(cx.listener(|_, _: &menu::OpenArchive, _, cx| cx.emit(Intent::RequestOpenArchive)))
            .on_action(cx.listener(|_, _: &menu::CreateArchive, _, cx| cx.emit(Intent::RequestCreateArchive)))
            .on_action(cx.listener(|_, _: &menu::AddFiles, _, cx| cx.emit(Intent::RequestAddFiles)))
            .on_action(cx.listener(|_, _: &menu::TestSelected, _, cx| cx.emit(Intent::TestSelected)))
            .on_action(cx.listener(|_, _: &menu::TestAll, _, cx| cx.emit(Intent::TestAll)))
            .on_action(cx.listener(|_, _: &menu::CloseArchive, _, cx| cx.emit(Intent::CloseArchive)))
            .on_action(cx.listener(|_, _: &menu::ShowProperties, _, cx| cx.emit(Intent::ShowProperties)))
            .on_action(cx.listener(|_, _: &menu::DeleteSelected, _, cx| cx.emit(Intent::DeleteSelected)))
            .on_action(cx.listener(|_, _: &menu::ChecksumCrc32, _, cx| cx.emit(Intent::RequestChecksum { algorithm: "CRC32".into() })))
            .on_action(cx.listener(|_, _: &menu::ChecksumMd5, _, cx| cx.emit(Intent::RequestChecksum { algorithm: "MD5".into() })))
            .on_action(cx.listener(|_, _: &menu::ChecksumSha1, _, cx| cx.emit(Intent::RequestChecksum { algorithm: "SHA1".into() })))
            .on_action(cx.listener(|_, _: &menu::ChecksumSha256, _, cx| cx.emit(Intent::RequestChecksum { algorithm: "SHA256".into() })))
            .on_action(cx.listener(|this, _: &menu::SelectAll, _, _cx| { this.state.select_all(); }))
            .on_action(cx.listener(|this, _: &menu::InvertSelection, _, _cx| { this.state.invert_selection(); }))
            .on_action(cx.listener(|this, _: &menu::RenameSelected, _, cx| { if let Some(idx) = this.state.first_selected_index() { cx.emit(ArchiveVmEvent::RequestRename { index: idx, new_name: String::new() }); } }))
            .on_action(cx.listener(|this, _: &menu::ToggleSidebar, _, cx| { if let Some(s) = this.app_shell.upgrade() { s.update(cx, |s, cx| s.toggle_sidebar(cx)); } }))
            .on_action(cx.listener(|_, _: &menu::ShowSettings, _, cx| { cx.spawn(async move |_, cx| { bit7z_pres_dialogs::settings::SettingsDialog::open(cx); }).detach(); }))
            .on_action(cx.listener(|_, _: &menu::About, _, cx| bit7z_pres_dialogs::about::AboutDialog::open(cx)))
            .child(self.menu.clone())
            .child(self.toolbar.clone())
            .child(div().flex_1().child(
                h_resizable("main-hz")
                    .child(resizable_panel().size(px(255.)).size_range(px(200.)..px(320.)).child(self.archive_browser.clone()))
                    .child(v_resizable("main-vt")
                        .child(resizable_panel().child(self.entry_list.clone()))
                        .child(resizable_panel().size(px(200.)).size_range(px(100.)..px(500.)).child(self.preview_panel.clone()))
                    )
            ))
            .child(self.status_bar.clone())
            .children(Root::render_dialog_layer(window, cx))
    }
}
