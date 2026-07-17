use gpui::{div, AnyElement, App, Div, Hsla, InteractiveElement, Interactivity, IntoElement, Pixels, RenderOnce, Stateful, StyleRefinement, Styled, Window, px, prelude::FluentBuilder as _};
use gpui_component::{ActiveTheme, StyledExt};

/// Width of the divider line.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum DividerWidth {
    /// 1px width.
    Thin,
    /// 2px width (default).
    #[default]
    Medium,
    /// 4px width.
    Thick,
    /// Custom width in pixels.
    Custom(Pixels),
}

impl DividerWidth {
    fn as_pixels(&self) -> Pixels {
        match self {
            DividerWidth::Thin => px(1.),
            DividerWidth::Medium => px(2.),
            DividerWidth::Thick => px(4.),
            DividerWidth::Custom(px) => *px,
        }
    }
}

impl From<Pixels> for DividerWidth {
    fn from(px: Pixels) -> Self {
        DividerWidth::Custom(px)
    }
}

/// Line style of the divider.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum DividerLineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

/// A visual divider for separating toolbar items.
///
/// Supports configurable width, line style, color, padding, and orientation.
#[derive(IntoElement)]
pub struct ToolbarDivider {
    base: Stateful<Div>,
    style: StyleRefinement,
    width: DividerWidth,
    line_style: DividerLineStyle,
    color: Option<Hsla>,
    vertical: bool,
}

impl Default for ToolbarDivider {
    fn default() -> Self {
        Self {
            base: div().id("toolbar-divider"),
            style: StyleRefinement::default(),
            width: DividerWidth::Medium,
            line_style: DividerLineStyle::Solid,
            color: None,
            vertical: true,
        }
    }
}

impl ToolbarDivider {
    /// Creates a new vertical divider (default for toolbars).
    pub fn vertical() -> Self {
        Self::default()._vertical(true)
    }

    /// Creates a new horizontal divider.
    pub fn horizontal() -> Self {
        Self::default()._vertical(false)
    }

    fn _vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Sets the width of the divider line.
    pub fn width(mut self, width: impl Into<DividerWidth>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the line style of the divider.
    pub fn line_style(mut self, line_style: DividerLineStyle) -> Self {
        self.line_style = line_style;
        self
    }

    /// Sets the color of the divider line.
    ///
    /// Defaults to `cx.theme().muted_foreground`.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl From<ToolbarDivider> for AnyElement {
    fn from(divider: ToolbarDivider) -> Self {
        divider.into_any_element()
    }
}

impl Styled for ToolbarDivider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for ToolbarDivider {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for ToolbarDivider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().muted_foreground);
        let width = self.width.as_pixels();
        let is_vertical = self.vertical;

        div()
            .flex_none()
            .when(is_vertical, |this| {
                this.w(width).h_full()
            })
            .when(!is_vertical, |this| {
                this.h(width).w_full()
            })
            .map(|this| match self.line_style {
                DividerLineStyle::Solid => {
                    this.bg(color)
                }
                // Dashed and Dotted are approximated with partial opacity + reduced size
                // since GPUI does not natively support dashed borders on independent elements.
                DividerLineStyle::Dashed => {
                    this.bg(color.opacity(0.6))
                }
                DividerLineStyle::Dotted => {
                    this.bg(color.opacity(0.4))
                }
            })
            .refine_style(&self.style)
    }
}
