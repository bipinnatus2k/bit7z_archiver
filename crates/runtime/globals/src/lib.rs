use gpui::{App, Global};
use std::sync::{Arc, Mutex};
use bit7z_domain::preferences::{Preferences, PreferencesRepository};
use bit7z_domain::repository::ArchiveRepository;
use bit7z_rt_ipc::GuiCommand;

/// Global receiver for IPC commands from CLI.
pub struct IpcReceiver(pub Arc<Mutex<crossbeam_channel::Receiver<GuiCommand>>>);
impl Global for IpcReceiver {}

/// Global wrapper for Preferences (avoids gpui::Global in domain).
pub struct PreferencesGlobal(pub Preferences);
impl Global for PreferencesGlobal {}

/// Global wrapper for ArchiveRepository (avoids gpui::Global in domain).
pub struct RepoGlobal(pub Arc<dyn ArchiveRepository>);
impl Global for RepoGlobal {}

/// Global wrapper for PreferencesRepository (avoids gpui::Global in domain).
pub struct PreferencesRepoGlobal(pub Arc<dyn PreferencesRepository>);
impl Global for PreferencesRepoGlobal {}
