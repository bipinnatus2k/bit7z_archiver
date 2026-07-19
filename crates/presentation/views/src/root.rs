use gpui::prelude::FluentBuilder;
use crate::app_shell::AppShell;
use crate::archive_sidebar::{ArchiveSideBar, BrowserIntent};
use crate::archive_file_list::{ArchiveFileList, FileListIntent};
use crate::menu::{self, Menu};
use crate::preview_panel::PreviewPanel;
use crate::status_bar::AppStatusBar;
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
    pub archive_browser: Entity<ArchiveSideBar>,
    pub entry_list: Entity<ArchiveFileList>,
    pub preview_panel: Entity<PreviewPanel>,
    pub status_bar: Entity<AppStatusBar>,
}

impl RootView {
    pub fn new(
        app_shell: WeakEntity<AppShell>,
        preview_panel: Entity<PreviewPanel>,
        archive_browser: Entity<ArchiveSideBar>,
        entry_list: Entity<ArchiveFileList>,
        cx: &mut Context<Self>,
    ) -> Self {
        let menu = cx.new(|cx| Menu::new(false, cx));
        let toolbar = cx.new(|_| Toolbar::new());
        let status_bar = cx.new(|_| AppStatusBar::default());

        cx.subscribe::<Toolbar, ToolbarIntent>(&toolbar, move |this, _, intent, cx| {
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
        cx.subscribe::<ArchiveSideBar, BrowserIntent>(&archive_browser, move |this, _, intent, cx| {
            match intent {
                BrowserIntent::NavigateInto(dir) => { if let Some(s) = this.app_shell.upgrade() { s.update(cx, |s, cx| s.navigate_into(dir, cx)); } }
                BrowserIntent::SetFilter(text) => { if let Some(s) = this.app_shell.upgrade() { s.update(cx, |s, cx| s.set_filter(text, cx)); } }
                BrowserIntent::OpenRecentFile(path) => { if let Some(s) = this.app_shell.upgrade() { s.update(cx, |s, cx| s.handle_open_archive(std::path::Path::new(path), None, cx)); } }
            }
        }).detach();

        cx.subscribe::<ArchiveFileList, FileListIntent>(&entry_list, move |this, _, intent, cx| {
            match intent {
                FileListIntent::SelectionChanged(indices) => {
                    this.state.selection = indices.iter().copied().collect();
                    this.state.selection_anchor = None;
                    this.sync_child_views(cx);
                    if let (Some(idx), Some(s)) = (indices.first(), this.app_shell.upgrade()) {
                        let h = s.read(cx).state.handle.clone();
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
                FileListIntent::SortByColumn(col, asc) => { this.state.apply_sort(*col, *asc); this.sync_child_views(cx); }
                FileListIntent::NavigateUp => { this.state.navigate_up(); this.sync_child_views(cx); this.load_current_directory(cx); }
                FileListIntent::OpenEntry => cx.emit(Intent::OpenEntry),
                FileListIntent::PreviewEntry => {},
                FileListIntent::ExtractSelected => cx.emit(Intent::Extract),
                FileListIntent::TestSelected => cx.emit(Intent::TestSelected),
                FileListIntent::RenameEntry(_) => {},
                FileListIntent::DeleteSelected => cx.emit(Intent::DeleteSelected),
                FileListIntent::Checksum(_) => {},
                FileListIntent::SelectAll => { this.state.select_all(); this.entry_list.update(cx, |c, cx| c.select_all_entries(cx)); this.sync_child_views(cx); }
                FileListIntent::ClearSelection => { this.state.clear_selection(); this.entry_list.update(cx, |c, cx| c.clear_selection(cx)); this.sync_child_views(cx); }
                FileListIntent::Refresh => cx.emit(Intent::Refresh),
                FileListIntent::ShowProperties => cx.emit(Intent::ShowProperties),
            }
        }).detach();

        let focus_handle = cx.focus_handle();
        Self { app_shell, state: AppState::new(), focus_handle, menu, toolbar, archive_browser, entry_list, preview_panel, status_bar }
    }

    fn load_current_directory(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.app_shell.upgrade() { s.update(cx, |s, cx| s.load_current_directory(cx)); }
    }

    pub fn sync_state(&mut self, state: &AppState, sidebar_collapsed: bool, cx: &mut Context<Self>) {
        self.state = state.clone(); self.sync_child_views(cx);
        self.menu.update(cx, |c, cx| {
            c.set_sidebar_collapsed(sidebar_collapsed);
            c.set_state(state.handle.is_some(), state.has_selection(), state.selection.len() == 1, cx);
        });
    }

    fn sync_child_views(&mut self, cx: &mut Context<Self>) {
        let e = self.state.displayed_entries().to_vec();
        let st = self.state.status.clone();
        let p = self.state.current_path.clone();
        let is_open = self.state.handle.is_some();
        self.entry_list.update(cx, |c, cx| c.set_state(e, st, p, cx));
        self.toolbar.update(cx, |c, _| c.set_state(is_open, self.state.is_ready(), self.state.has_selection()));
        self.archive_browser.update(cx, |c, cx| { c.set_collapsed(false, cx); c.set_state(self.state.filtered_subdirs(), vec![]); });
        self.status_bar.update(cx, |c, _| c.left(&self.state.status_text()));
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
                    "a" if cmd && !shift => { this.state.select_all(); this.entry_list.update(cx, |c, cx| c.select_all_entries(cx)); this.sync_child_views(cx); }
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
            .on_action(cx.listener(|this, _: &menu::SelectAll, _, cx| { this.state.select_all(); this.entry_list.update(cx, |c, cx| c.select_all_entries(cx)); this.sync_child_views(cx); }))
            .on_action(cx.listener(|this, _: &menu::InvertSelection, _, cx| { this.state.invert_selection(); this.sync_child_views(cx); }))
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
