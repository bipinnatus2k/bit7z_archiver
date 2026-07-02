//! Design constants for consistent spacing, breakpoints, and layout.

/// Responsive breakpoints (in pixels)
pub const BREAKPOINT_SM: f32 = 640.0;
pub const BREAKPOINT_MD: f32 = 768.0;
pub const BREAKPOINT_LG: f32 = 1024.0;
pub const BREAKPOINT_XL: f32 = 1280.0;

/// Layout dimensions
pub const STATUS_BAR_HEIGHT: f32 = 32.0;
pub const TOOLBAR_HEIGHT: f32 = 48.0;
pub const SIDEBAR_MIN_WIDTH: f32 = 200.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 320.0;
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 240.0;

/// Spacing scale (4px base)
pub const SPACING_XS: f32 = 4.0;
pub const SPACING_SM: f32 = 8.0;
pub const SPACING_MD: f32 = 12.0;
pub const SPACING_LG: f32 = 16.0;
pub const SPACING_XL: f32 = 24.0;
pub const SPACING_2XL: f32 = 32.0;

/// Icon sizes
pub const ICON_SIZE_SM: f32 = 16.0;
pub const ICON_SIZE_MD: f32 = 20.0;
pub const ICON_SIZE_LG: f32 = 24.0;

/// Touch target minimum size (for accessibility)
pub const TOUCH_TARGET_MIN: f32 = 44.0;

/// Animation durations (in milliseconds)
pub const ANIMATION_DURATION_FAST: u64 = 150;
pub const ANIMATION_DURATION_NORMAL: u64 = 200;
pub const ANIMATION_DURATION_SLOW: u64 = 300;

/// Window sizes
pub const WINDOW_MIN_WIDTH: f32 = 640.0;
pub const WINDOW_MIN_HEIGHT: f32 = 480.0;
pub const WINDOW_DEFAULT_WIDTH: f32 = 800.0;
pub const WINDOW_DEFAULT_HEIGHT: f32 = 600.0;
