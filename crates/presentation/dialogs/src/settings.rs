use gpui::*;
use gpui_component::v_flex;

pub struct SettingsDialog {
    panel: Entity<bit7z_pres_settings::panels::SettingsPanel>,
}

impl SettingsDialog {
    pub fn open(cx: &mut AsyncApp) {
        cx.spawn(async move |cx| {
            let panel = cx.new(|_| bit7z_pres_settings::panels::SettingsPanel::new());
            bit7z_pres_settings::register_panel_entity(&panel);

            let _ = cx.open_window(
                WindowOptions {
                    titlebar: TitlebarOptions {
                        title: Some(SharedString::from("Settings")),
                        appears_transparent: false,
                        traffic_light_position: None,
                    }
                    .into(),
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(150.), px(150.)),
                        size(px(480.), px(500.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    focus: true,
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|_cx| SettingsDialog { panel });
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        })
        .detach();
    }
}

impl Render for SettingsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().child(self.panel.clone())
    }
}
