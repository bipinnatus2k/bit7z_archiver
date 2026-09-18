//! Elevation shadow scale for the design system.
//!
//! GPUI has no CSS-style `.shadow-*` utility; the primitive is a
//! `Vec<BoxShadow>` applied via the [`gpui::Styled::shadow`] method. This
//! module provides a generic named scale ([`Shadow`]) whose values mirror the
//! Tailwind box-shadow reference, plus a [`StyledShadow`] ext so components can
//! write `el.shadow_level(Shadow::Md)`.

use gpui::{BoxShadow, Styled, hsla, point, px};
use smallvec::{SmallVec, smallvec};

/// Named elevation levels. Values mirror the Tailwind box-shadow scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shadow {
    /// Subtle 1px shadow (`shadow-sm`).
    Sm,
    /// Default card shadow (`shadow`).
    Base,
    /// Medium raised shadow (`shadow-md`).
    Md,
    /// Large popover/dropdown shadow (`shadow-lg`).
    Lg,
    /// Extra-large modal shadow (`shadow-xl`).
    Xl,
}

fn black(alpha: f32) -> gpui::Hsla {
    hsla(0., 0., 0., alpha)
}

impl Shadow {
    /// The box-shadow layers for this elevation level.
    pub fn box_shadows(self) -> SmallVec<[BoxShadow; 2]> {
        match self {
            Shadow::Sm => smallvec![BoxShadow {
                color: black(0.05),
                offset: point(px(0.), px(1.)),
                blur_radius: px(2.),
                spread_radius: px(0.),
                inset: false,
            }],
            Shadow::Base => smallvec![
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(1.)),
                    blur_radius: px(3.),
                    spread_radius: px(0.),
                    inset: false,
                },
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(1.)),
                    blur_radius: px(2.),
                    spread_radius: px(-1.),
                    inset: false,
                },
            ],
            Shadow::Md => smallvec![
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(6.),
                    spread_radius: px(-1.),
                    inset: false,
                },
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(2.)),
                    blur_radius: px(4.),
                    spread_radius: px(-2.),
                    inset: false,
                },
            ],
            Shadow::Lg => smallvec![
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(10.)),
                    blur_radius: px(15.),
                    spread_radius: px(-3.),
                    inset: false,
                },
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(6.),
                    spread_radius: px(-4.),
                    inset: false,
                },
            ],
            Shadow::Xl => smallvec![
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(20.)),
                    blur_radius: px(25.),
                    spread_radius: px(-5.),
                    inset: false,
                },
                BoxShadow {
                    color: black(0.1),
                    offset: point(px(0.), px(8.)),
                    blur_radius: px(10.),
                    spread_radius: px(-6.),
                    inset: false,
                },
            ],
        }
    }
}

/// Extends [`gpui::Styled`] with a named shadow scale.
pub trait StyledShadow: Styled + Sized {
    /// Applies the box-shadow layers for the given [`Shadow`] level.
    fn shadow_level(self, level: Shadow) -> Self {
        self.shadow(level.box_shadows().to_vec())
    }
}

impl<E: Styled> StyledShadow for E {}
