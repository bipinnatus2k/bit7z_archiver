//! Focus ring wrapper with a true offset gap.
//!
//! GPUI has no `ring`/`ring-offset` utility. To reproduce Tailwind's
//! `ring-2 ring-offset-2` look (a ring separated from the element by a gap),
//! we WRAP the focusable content in an outer bordered container with inner
//! padding. The wrapper keeps its size in both states (transparent border when
//! unfocused) so focus does not shift layout.

use gpui::{Div, Hsla, IntoElement, ParentElement, Styled, div, px, transparent_black};

use crate::styles::palette;

/// Wraps `content` in a gapped focus ring of `color`, shown only when
/// `focused`. Layout is stable across focus states.
pub fn focus_ring(content: impl IntoElement, focused: bool, color: Hsla) -> Div {
    let ring_color = if focused { color } else { transparent_black() };
    div()
        .rounded_lg()
        .border_2()
        .border_color(ring_color)
        .p(px(2.))
        .child(content)
}

/// Focus ring in the primary accent color (default for inputs/buttons).
pub fn focus_ring_primary(content: impl IntoElement, focused: bool) -> Div {
    focus_ring(content, focused, palette::primary(500))
}

/// Focus ring in the danger color, for invalid/error fields.
pub fn focus_ring_error(content: impl IntoElement, focused: bool) -> Div {
    focus_ring(content, focused, palette::danger(500))
}
