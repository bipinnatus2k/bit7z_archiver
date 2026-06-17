use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::domain::preferences::Preferences;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};

pub struct ArchiveBrowser {
    archive_vm: Entity<ArchiveViewModel>,
    input_state: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl ArchiveBrowser {
    pub fn new(archive_vm: Entity<ArchiveViewModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Filter..."));

        let _subscriptions = vec![cx.subscribe_in(&input_state, window, {
            let archive_vm = archive_vm.clone();
            let input_state = input_state.clone();
            move |_this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = input_state.read(cx).value();
                    archive_vm.update(cx, |vm, cx| vm.set_filter(&value, cx));
                }
                _ => {}
            }
        })];

        Self {
            archive_vm,
            input_state,
            _subscriptions,
        }
    }
}

impl Render for ArchiveBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let prefs = cx.global::<Preferences>();
        let recent_files = &prefs.archive.recent_files;
        let has_recent = !recent_files.is_empty();
        let theme = cx.global::<Theme>();

        gpui_component::v_flex().w(px(240.)).p_2().gap_2()
            // Filter input (always visible)
            .child(Input::new(&self.input_state))
            // Folder tree — current path's subdirectories
            .child(gpui_component::v_flex().text_sm().children(
                vm.current_subdirs().iter().map(|name|
                    div().px_2().py_1().cursor_pointer().child(format!("\u{1F4C1} {}", name))
                ).collect::<Vec<_>>()
            ))
            // Recent files section
            .when(has_recent, |el| el.child(
                gpui_component::v_flex().gap_1().pt_2()
                    .child(div().px_2().py_1().text_sm().font_weight(FontWeight::BOLD).text_color(theme.muted).child("Recent Files"))
                    .children(recent_files.iter().map(|path| {
                        let file_name = std::path::Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.clone());
                        let path_clone = path.clone();
                        let vm = self.archive_vm.clone();
                        div().px_2().py_1().cursor_pointer().text_sm()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |_this: &mut ArchiveBrowser, _event: &MouseDownEvent, _window: &mut Window, cx| {
                                let path_ref = std::path::Path::new(&path_clone);
                                vm.update(cx, |vm, cx| vm.open_archive(path_ref, None, cx));
                            }))
                            .child(format!("\u{1F4C2} {}", file_name))
                    }).collect::<Vec<_>>())
            ))
    }
}
