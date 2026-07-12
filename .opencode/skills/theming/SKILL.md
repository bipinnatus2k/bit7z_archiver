---
name: theming
description: Deep analysis of the gpui-component theme system — Theme singleton, ActiveTheme trait, ThemeColor struct, ThemeTokens semantic colors, ThemeConfig JSON schema, light/dark mode switching, Oklab color space and Colorize trait, gradient background support, border radii, shadow system, font configuration, scrollbar display modes. Use when modifying colors, adding theme tokens, creating theme-aware components, implementing dark mode support, configuring theme defaults, or understanding color manipulation.
---

# Theming System

## Architecture

The theme system is located in `crates/ui/src/theme/`:

```
crates/ui/src/theme/
├── mod.rs           — Theme struct definition + init() + global access
├── color.rs         — Colorize trait (color manipulation utilities)
├── schema.rs        — ThemeConfig JSON schema definitions
├── registry.rs      — Theme registry (theme presets)
├── theme_color.rs   — ThemeColor struct (color properties)
├── default-colors.json  — Default light/dark color values
├── default-theme.json   — Complete default theme
```

### Core Types

```rust
// The main Theme singleton
pub struct Theme: Global {
    pub colors: ThemeColor,         // Individual color properties
    pub tokens: ThemeTokens,        // Semantic token colors
    pub highlight_theme: Arc<HighlightTheme>,  // Syntax highlighting
    pub light_theme: Rc<ThemeConfig>,
    pub dark_theme: Rc<ThemeConfig>,
    pub mode: ThemeMode,            // Light or Dark
    pub font_family: FontFamily,
    pub font_size: Pixels,
    pub mono_font_family: FontFamily,
    pub mono_font_size: Pixels,
    pub radius: Pixels,             // Default border radius
    pub radius_lg: Pixels,          // Large border radius
    pub shadow: bool,               // Whether shadows are enabled
    pub transparent: Hsla,          // Transparent color reference
    pub scrollbar_show: ScrollbarShow,
    pub notification: NotificationSettings,
    pub tile_grid_size: Pixels,
    pub tile_shadow: bool,
    pub tile_radius: Pixels,
    pub list: ListSettings,
    pub sheet: SheetSettings,
}
```

`Theme` implements `Deref<Target = ThemeColor>`, so all `ThemeColor` fields are accessible directly via `cx.theme().field_name`.

---

## Access Pattern

```rust
use gpui_component::{ActiveTheme, Theme};

// Read theme
cx.theme().background          // ThemeColor field (via Deref)
cx.theme().foreground          // ThemeColor field
cx.theme().tokens.primary      // ThemeTokens field
cx.theme().radius              // Direct Theme field
cx.theme().mode                // ThemeMode::Light or ThemeMode::Dark
cx.theme().shadow              // Whether shadows are enabled

// Modify theme
let mut theme = cx.theme().clone();
theme.mode = ThemeMode::Dark;
cx.set_global::<Theme>(theme);
window.refresh();
```

---

## ThemeColor Properties

Accessible via `cx.theme().property_name`. Full list from `crates/ui/src/theme/theme_color.rs`:

### Background/Foreground
| Property | Purpose |
|----------|---------|
| `background` | Main application background |
| `foreground` | Main text/foreground color |
| `muted` | Muted/subsidiary background |
| `muted_foreground` | Muted text color |
| `accent` | Accent background |
| `accent_foreground` | Accent text |

### Semantic Colors
| Property | Purpose |
|----------|---------|
| `primary` | Primary brand color |
| `primary_foreground` | Text on primary |
| `secondary` | Secondary brand |
| `secondary_foreground` | Text on secondary |
| `danger` | Danger/error color |
| `danger_foreground` | Text on danger |
| `warning` | Warning color |
| `warning_foreground` | Text on warning |
| `success` | Success color |
| `success_foreground` | Text on success |
| `info` | Info color |
| `info_foreground` | Text on info |

### UI Specific
| Property | Purpose |
|----------|---------|
| `border` | Default border color |
| `input` | Input field border/background |
| `input_foreground` | Input text |
| `ring` | Focus ring color |
| `link` | Link text color |
| `link_hover` | Link hover state |
| `link_active` | Link active state |
| `background_mixed` | Mixed background |
| `popover_foreground` | Popover text |
| `sidebar_primary` | Sidebar primary background |
| `sidebar_primary_foreground` | Sidebar primary text |
| `table_head_foreground` | Table header text |
| `table_row_border` | Table row border |
| `table_foot_foreground` | Table footer text |

### Button Colors
| Property | Purpose |
|----------|---------|
| `button_foreground` | Default button text |
| `button_primary_foreground` | Primary button text |
| `button_secondary_foreground` | Secondary button text |
| `button_danger` | Danger button border |
| `button_danger_foreground` | Danger button text |
| `button_warning` | Warning button border |
| `button_warning_foreground` | Warning button text |
| `button_success` | Success button border |
| `button_success_foreground` | Success button text |
| `button_info` | Info button border |
| `button_info_foreground` | Info button text |

### Custom Colors
| Property | Purpose |
|----------|---------|
| `cyan` | Cyan color |
| `magenta` | Magenta color |
| `yellow` | Yellow color |
| `transparent` | Transparent color |
| `overlay` | Overlay/modal backdrop |

---

## ThemeTokens (Semantic Tokens)

`ThemeTokens` provides semantic token access. These are used for component-specific colors:

```rust
pub struct ThemeTokens {
    pub background: Hsla,
    pub foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
    pub primary: ThemeToken,        // { background, foreground }
    pub primary_hover: ThemeToken,
    pub primary_active: ThemeToken,
    pub secondary: ThemeToken,
    pub secondary_hover: ThemeToken,
    pub secondary_active: ThemeToken,
    pub danger: ThemeToken,
    pub danger_hover: ThemeToken,
    pub danger_active: ThemeToken,
    pub warning: ThemeToken,
    pub warning_hover: ThemeToken,
    pub warning_active: ThemeToken,
    pub success: ThemeToken,
    pub success_hover: ThemeToken,
    pub success_active: ThemeToken,
    pub info: ThemeToken,
    pub info_hover: ThemeToken,
    pub info_active: ThemeToken,
    pub table: Hsla,
    pub table_head: Hsla,
    pub table_foot: Hsla,
    pub button: Background,              // can be Hsla or Gradient
    pub button_hover: Background,
    pub button_active: Background,
    pub button_primary: Background,
    pub button_primary_hover: Background,
    pub button_primary_active: Background,
    pub button_secondary: Background,
    pub button_secondary_hover: Background,
    pub button_secondary_active: Background,
    pub button_danger: Background,
    pub button_danger_hover: Background,
    pub button_danger_active: Background,
    pub button_warning: Background,
    pub button_warning_hover: Background,
    pub button_warning_active: Background,
    pub button_success: Background,
    pub button_success_hover: Background,
    pub button_success_active: Background,
    pub button_info: Background,
    pub button_info_hover: Background,
    pub button_info_active: Background,
    pub sidebar: Hsla,
    pub sidebar_primary: Hsla,
    pub sidebar_primary_foreground: Hsla,
    pub sidebar_accent: Hsla,
    pub sidebar_accent_foreground: Hsla,
    pub sidebar_border: Hsla,
    pub sidebar_ring: Hsla,
}
```

`ThemeToken` contains `background: Background` and `foreground: Hsla`.

---

## ThemeConfig (JSON Schema)

Themes are defined as JSON objects via `ThemeConfig`:

```json
{
  "name": "My Theme",
  "mode": "light",
  "colors": {
    "background": "#FFFFFF",
    "foreground": "#09090B",
    "primary.background": "linear-gradient(135deg, #4F46E5, #06B6D4)",
    "primary.foreground": "#FFFFFF",
    "button.primary.background": "#18181B",
    "button.primary.foreground": "#FAFAFA",
    "button.primary.hover.background": "#27272A",
    "button.primary.active.background": "#3F3F46",
    "radius": 8,
    "radius.lg": 12
  }
}
```

### Color Value Formats

Colors in `ThemeConfig` can be:
1. **Hex string** — `"#FFFFFF"`, `"#09090B"`, `"#4F46E5"`
2. **Linear gradient** — `"linear-gradient(135deg, #4F46E5, #06B6D4)"`
3. **CSS color names** — `"red"`, `"green"`, `"transparent"` (limited support)

### Theme Config Key Naming

Keys use dot notation following shadcn/ui conventions:
- `primary.background` / `primary.foreground`
- `primary.hover.background` / `primary.hover.foreground`
- `primary.active.background` / `primary.active.foreground`
- `button.primary.background` / `button.primary.hover.background`
- `muted`, `muted.foreground`
- `border`, `input`, `ring`
- `radius`, `radius.lg`
- `font.family`, `font.size`, `mono.font.family`

### Applying a Custom Theme

```rust
let config = serde_json::from_value::<ThemeConfig>(serde_json::json!({
    "name": "Custom",
    "mode": "dark",
    "colors": { ... }
})).unwrap();

Theme::global_mut(cx).apply_config(&Rc::new(config));
window.refresh();
```

---

## Color Manipulation (Colorize Trait)

Defined in `crates/ui/src/theme/color.rs`:

```rust
pub trait Colorize: Sized {
    /// Set alpha opacity (0.0 - 1.0)
    fn opacity(&self, opacity: f32) -> Self;

    /// Divide color by a divisor
    fn divide(&self, divisor: f32) -> Self;

    /// Invert RGB (not alpha)
    fn invert(&self) -> Self;

    /// Invert lightness only (preserves hue/saturation)
    fn invert_l(&self) -> Self;

    /// Lighten by amount (0.0 - 1.0)
    fn lighten(&self, amount: f32) -> Self;

    /// Darken by amount (0.0 - 1.0)
    fn darken(&self, amount: f32) -> Self;

    /// Mix with another color (factor: 0.0 = self, 1.0 = other)
    fn mix(&self, other: Self, factor: f32) -> Self;

    /// Mix in Oklab color space (more perceptually uniform)
    fn mix_oklab(&self, other: Self, factor: f32) -> Self;

    /// Shift hue by degrees (0-360)
    fn hue(&self, hue: f32) -> Self;

    /// Set saturation (0.0 - 1.0)
    fn saturation(&self, saturation: f32) -> Self;

    /// Set lightness (0.0 - 1.0)
    fn lightness(&self, lightness: f32) -> Self;

    /// Serialize to hex string (#RRGGBB)
    fn to_hex(&self) -> String;

    /// Parse from hex string (#RRGGBB or #RGB)
    fn parse_hex(hex: &str) -> Result<Self>;
}
```

### Common Color Operations in Components

```rust
// Disabled state: reduce opacity
cx.theme().foreground.opacity(0.5)

// Semi-transparent overlay
cx.theme().overlay.opacity(0.5)

// Hover effect: mix with transparent
cx.theme().input.mix_oklab(cx.theme().transparent, 0.5)

// Lighten/darken for hover/active
cx.theme().secondary.lighten(0.1)
cx.theme().secondary.darken(0.1)

// Outline buttons: use opacity of semantic colors
cx.theme().tokens.primary.background.opacity(0.1)  // normal
cx.theme().tokens.primary_hover.background.opacity(0.2)  // hover
cx.theme().tokens.primary_active.background.opacity(0.4)  // active
```

---

## Light/Dark Mode

### Checking Mode

```rust
if cx.theme().mode.is_dark() {
    // dark mode specific styles
} else {
    // light mode specific styles
}
```

### Switching Mode

```rust
// Direct toggle
let mut theme = cx.theme().clone();
theme.mode = ThemeMode::Dark;
cx.set_global::<Theme>(theme);
window.refresh();

// Or use Theme::change
Theme::change(ThemeMode::Dark, None, cx);
```

### Init Flow

```rust
// In theme::init():
pub fn init(cx: &mut App) {
    registry::init(cx);
    Theme::change(ThemeMode::Light, None, cx);  // defaults to light
    Theme::sync_scrollbar_appearance(cx);
}
```

---

## Border Radii

```rust
cx.theme().radius       // Default radius (used for buttons, inputs, cards)
cx.theme().radius_lg    // Large radius (used for dialogs, modals)
```

These are `Pixels` values configurable via `ThemeConfig`:
```json
{
  "colors": {
    "radius": 8,
    "radius.lg": 12
  }
}
```

---

## Shadow System

```rust
cx.theme().shadow  // bool — whether shadows are globally enabled
```

Individual elements check this before applying shadows:
```rust
.when(cx.theme().shadow && style.shadow, |this| this.shadow_xs())
```

Shadow display is toggled via the story gallery controls.

---

## Font Configuration

```rust
cx.theme().font_family        // System UI font
cx.theme().font_size          // Base font size
cx.theme().mono_font_family   // Monospace font (for code)
cx.theme().mono_font_size     // Monospace font size
```

Configured via `ThemeConfig`:
```json
{
  "colors": {
    "font.family": "system-ui",
    "font.size": 14,
    "mono.font.family": "JetBrains Mono",
    "mono.font.size": 13
  }
}
```

---

## Background Type

The `Background` enum supports both solid colors and gradients:

```rust
pub enum Background {
    Color(Hsla),
    Gradient(gpui::LinearGradient),
}
```

`From<Hsla>` and `From<gpui::LinearGradient>` are implemented, so components can use either:

```rust
// Solid color
cx.theme().tokens.button_primary.into()

// Check for gradient equality
assert_eq!(
    ButtonVariant::Primary.normal(false, cx).bg,
    cx.theme().tokens.button_primary.into()
);

// Gradient backgrounds in config
// "button.primary.background": "linear-gradient(135deg, #4F46E5, #06B6D4)"
```

---

## Theme Registration

### Default Themes

The default light and dark themes are defined in `default-colors.json` and `default-theme.json`. These are loaded during `theme::init()`.

### Theme Registry

Custom themes can be registered via `registry.rs`. The registry stores named themes that users can switch between.

### Theme Persistence

Theme selections are persisted and restored across application sessions through the global state.

---

## Helper Functions

```rust
// Parse a hex color string
gpui_component::try_parse_color("#4F46E5")

// HSL constructor
gpui_component::color::hsl(hue, saturation, lightness)

// Color swatch helpers (named colors)
gpui_component::red_500()
gpui_component::blue_500()
gpui_component::green_500()
gpui_component::yellow_500()
gpui_component::pink_500()
```

---

## Component Theme Integration Pattern

When designing a theme-aware component:

1. Use `cx.theme().tokens.*` for semantic colors (background, foreground, border)
2. Use `cx.theme().property_name` for specific colors (primary, danger, muted)
3. Use `cx.theme().radius` / `cx.theme().radius_lg` for border radii
4. Check `cx.theme().mode.is_dark()` for dark mode conditional styles
5. Check `cx.theme().shadow` before applying shadows
6. Use `Colorize::opacity()` for disabled state
7. Use `Colorize::mix_oklab()` for hover/active transitions (more perceptually uniform than mix)
8. Use `Background::from(Hsla)` / `Background::from(Gradient)` for button backgrounds
