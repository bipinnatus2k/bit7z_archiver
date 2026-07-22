use gpui::{Action, App, Context, DispatchPhase, Interactivity, Window};
use std::any::TypeId;

pub(crate) trait ExInteractivity {}

impl ExInteractivity for Interactivity {}
