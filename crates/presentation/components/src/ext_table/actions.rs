use gpui::{actions, Action};
use serde::Deserialize;

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = ext_table, no_json)]
pub struct Confirm {
    pub secondary: bool,
}

actions!(ext_table, [Cancel, SelectUp, SelectDown, SelectLeft, SelectRight, SelectFirst, SelectLast, SelectPrevColumn, SelectNextColumn, SelectPageUp, SelectPageDown]);
