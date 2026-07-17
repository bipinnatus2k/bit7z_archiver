use gpui_component::{Icon, Sizable, Size, progress::ProgressCircle, spinner::Spinner};
use gpui::{App, IntoElement, RenderOnce, Window, prelude::FluentBuilder};

/// Button icon which can be an Icon, Spinner, or Progress use for `icon` method of Button.
#[derive(IntoElement)]
pub struct ToolbarButtonIcon {
    icon: ToolbarButtonIconVariant,
    loading_icon: Option<Icon>,
    loading: bool,
    size: Size,
}

impl<T> From<T> for ToolbarButtonIcon
where
    T: Into<ToolbarButtonIconVariant>,
{
    fn from(icon: T) -> Self {
        ToolbarButtonIcon::new(icon)
    }
}

impl ToolbarButtonIcon {
    /// Creates a new ButtonIcon with the given icon.
    pub fn new(icon: impl Into<ToolbarButtonIconVariant>) -> Self {
        Self {
            icon: icon.into(),
            loading_icon: None,
            loading: false,
            size: Size::Medium,
        }
    }

    pub(crate) fn loading_icon(mut self, icon: Option<Icon>) -> Self {
        self.loading_icon = icon;
        self
    }

    pub(crate) fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl Sizable for ToolbarButtonIcon {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// Button icon which can be an Icon, Spinner, Progress, or ProgressCircle use for `icon` method of Button.
#[doc(hidden)]
#[derive(IntoElement)]
pub enum ToolbarButtonIconVariant {
    Icon(Icon),
    Spinner(Spinner),
    Progress(ProgressCircle),
}

impl<T> From<T> for ToolbarButtonIconVariant
where
    T: Into<Icon>,
{
    fn from(icon: T) -> Self {
        Self::Icon(icon.into())
    }
}

impl ToolbarButtonIconVariant {
    /// Returns true if the ButtonIconKind is an Icon.
    #[inline]
    pub(crate) fn is_spinner(&self) -> bool {
        matches!(self, Self::Spinner(_))
    }

    /// Returns true if the ButtonIconKind is a Progress or ProgressCircle.
    #[inline]
    pub(crate) fn is_progress(&self) -> bool {
        matches!(self, Self::Progress(_))
    }
}

impl Sizable for ToolbarButtonIconVariant {
    fn with_size(self, size: impl Into<Size>) -> Self {
        match self {
            Self::Icon(icon) => Self::Icon(icon.with_size(size)),
            Self::Spinner(spinner) => Self::Spinner(spinner.with_size(size)),
            Self::Progress(progress) => Self::Progress(progress.with_size(size)),
        }
    }
}

impl RenderOnce for ToolbarButtonIconVariant {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            Self::Icon(icon) => icon.into_any_element(),
            Self::Spinner(spinner) => spinner.into_any_element(),
            Self::Progress(progress) => progress.into_any_element(),
        }
    }
}

impl RenderOnce for ToolbarButtonIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        if self.loading {
            if self.icon.is_spinner() || self.icon.is_progress() {
                self.icon.with_size(self.size).into_any_element()
            } else {
                Spinner::new()
                    .when_some(self.loading_icon, |this, icon| this.icon(icon))
                    .with_size(self.size)
                    .into_any_element()
            }
        } else {
            self.icon.with_size(self.size).into_any_element()
        }
    }
}
