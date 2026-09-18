//! The prelude of this crate. When building UI in this app you almost always want to import this.

pub use gpui::prelude::*;
pub use gpui::{
    AbsoluteLength, AnyElement, App, Context, DefiniteLength, Div, Element, ElementId,
    InteractiveElement, ParentElement, Pixels, Rems, RenderOnce, SharedString, Styled, Window, div,
    px, relative, rems,
};

pub use component::{
    Component, ComponentScope, example_group, example_group_with_title, single_example,
};
pub use ui_macros::RegisterComponent;

// pub use crate::DynamicSpacing;
pub use crate::animation::{AnimationDirection, AnimationDuration, DefaultAnimations};
pub use crate::styles::focus_ring::{focus_ring, focus_ring_error, focus_ring_primary};
pub use crate::styles::{
    PlatformStyle, Severity, rems_from_px, vh, vw,
};
pub use crate::styles::{Shadow, StyledShadow, palette};
// pub use crate::tab_switcher_preview;
// pub use crate::terminal_panel_preview;
// pub use crate::terminal_view_preview;
// pub use crate::traits::clickable::*;
// pub use crate::traits::disableable::*;
// pub use crate::traits::fixed::*;
// pub use crate::traits::styled_ext::*;
// pub use crate::traits::toggleable::*;
// pub use crate::traits::visible_on_hover::*;
// pub use gpui_component::{
//     ActionPanel, Alert, AlertDialog, AppShell, AspectRatio, Badge, BadgeColor, BadgeVariant,
//     Breadcrumb, BreadcrumbItem, CalendarPreview, Card, CardVariant, CarouselPreview, Chart,
//     ChartKind, Checkbox, CodeEditor, Combobox, Command, CommandPalette, CommandPaletteItem,
//     Container, DatePickerPreview, DescriptionList, DescriptionListMode, Drawer, EmptyState, Feed,
//     FileInput, Form, FormField, HoverCard, InputGroup, InputOtp, Item, Kbd, LayoutTable,
//     LayoutTableBody, LayoutTableCaption, LayoutTableCell, LayoutTableFooter, LayoutTableHead,
//     LayoutTableHeader, LayoutTableRow, MediaObject, Menubar, MultiSelect, Navbar,
//     NavigationMenuPreview, PageHeading, Pagination, Pane, PaneGroup, PaneGroupPreview, RadioButton,
//     ResizablePreview, SearchInput, SectionHeading, SegmentedControl, Select, SheetSide, Sidebar,
//     SidebarItem, Skeleton, Slider, SonnerStackPreview, Spinner, StatsCard, StatsTrend, Stepper,
//     StepperStep, Switch, TabSwitcher, TabSwitcherItem, TerminalPanel, TerminalView, TextInput,
//     Textarea, TitleBar, ToastStack, ToggleGroup, ToggleGroupItem, ToggleGroupMode, VerticalNav,
//     VerticalNavItem,
// };
pub use gpui_component::button::{
    Button, ButtonGroup, ButtonVariant
};
pub use gpui_component::{Colorize,ColorName,ThemeColor};
// pub use crate::{
//     CannotRemoveLastPane, ClosePane, FocusDown, FocusLeft, FocusRight, FocusUp, Member, PaneAxis,
//     SplitDirection, SplitDown, SplitLeft, SplitRight, SplitUp, TabContent, TabId,
//     register_pane_keybindings,
// };
// pub use crate::{Headline, HeadlineSize};
pub use gpui_component::{Icon, IconName};
pub use gpui_component::label::{Label};
// pub use crate::{
//     code_editor_preview, combobox_preview, multi_select_preview, search_input_preview,
// };
pub use gpui_component::{h_flex, v_flex};
// pub use gpui_component::{
//     h_group, h_group_lg, h_group_sm, h_group_xl, v_group, v_group_lg, v_group_sm, v_group_xl,
// };
pub use gpui_component::theme::ActiveTheme;
pub use gpui_component::avatar::Avatar;
