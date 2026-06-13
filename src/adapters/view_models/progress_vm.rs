use gpui::*;

#[derive(Debug, Clone)]
pub struct ProgressState {
    pub is_active: bool,
    pub message: String,
    pub current: u64,
    pub total: u64,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self { is_active: false, message: String::new(), current: 0, total: 0 }
    }
}

impl Global for ProgressState {}

impl ProgressState {
    pub fn start(message: impl Into<String>, total: u64, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_active = true;
            state.message = message.into();
            state.current = 0;
            state.total = total;
        });
    }

    pub fn update(current: u64, message: impl Into<String>, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.current = current;
            state.message = message.into();
        });
    }

    pub fn complete(message: impl Into<String>, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_active = false;
            state.message = message.into();
        });
    }

    pub fn error(message: impl Into<String>, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_active = false;
            state.message = message.into();
        });
    }

    pub fn percent(&self) -> f32 {
        if self.total == 0 { 0.0 } else { self.current as f32 / self.total as f32 }
    }
}