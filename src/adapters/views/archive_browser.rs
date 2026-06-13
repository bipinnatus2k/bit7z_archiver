use crate::adapters::view_models::archive_vm::ArchiveViewModel;
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
        gpui_component::v_flex().w(px(240.)).p_2().gap_2()
            .child(Input::new(&self.input_state))
            .child(gpui_component::v_flex().text_sm().children(
                vm.entries.iter().filter(|e| e.is_directory).map(|e|
                    div().px_2().py_1().cursor_pointer().child(format!("📁 {}", e.name))
                ).collect::<Vec<_>>()
            ))
    }
}
