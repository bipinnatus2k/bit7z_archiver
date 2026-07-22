use std::sync::{Arc, Mutex};

use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_domain::preferences::PreferencesRepository;
use bit7z_infra_tray::TrayManager;
use bit7z_rt_ipc::GuiCommand;
use gpui::App;

/// Aggregate of all core application services.
/// Follows Zed's AppState pattern — a single struct injected into views
/// instead of each view pulling dependencies from disparate Globals.
pub struct AppState {
    pub service: Arc<ArchiveService>,
    pub preferences_repo: Arc<dyn PreferencesRepository>,
    pub tray: Arc<TrayManager>,
    pub ipc_receiver: Arc<Mutex<crossbeam_channel::Receiver<GuiCommand>>>,
}

impl AppState {
    pub fn new(
        service: Arc<ArchiveService>,
        preferences_repo: Arc<dyn PreferencesRepository>,
        tray: Arc<TrayManager>,
        ipc_receiver: Arc<Mutex<crossbeam_channel::Receiver<GuiCommand>>>,
    ) -> Self {
        Self {
            service,
            preferences_repo,
            tray,
            ipc_receiver,
        }
    }

    pub fn global(cx: &App) -> Arc<Self> {
        cx.global::<GlobalAppState>().0.clone()
    }

    pub fn set_global(state: Arc<Self>, cx: &mut App) {
        cx.set_global(GlobalAppState(state));
    }
}

struct GlobalAppState(Arc<AppState>);
impl gpui::Global for GlobalAppState {}
