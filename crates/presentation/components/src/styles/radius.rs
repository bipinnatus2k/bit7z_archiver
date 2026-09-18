//! shadcn `--radius` scale mapped to GPUI corner-radius builders.
//!
//! shadcn (Tailwind v4 template, `globals.css`) defines:
//! - `--radius: 0.625rem` (10px at 16px root)
//! - `--radius-sm: calc(var(--radius) - 4px)` → 6px
//! - `--radius-md: calc(var(--radius) - 2px)` → 8px
//! - `--radius-lg: var(--radius)` → 10px
//! - `--radius-xl: calc(var(--radius) + 4px)` → 14px
//!
//! GPUI's own Tailwind-style corner radius steps (see
//! `crates/gpui_macros/src/styles.rs`'s `corner_suffixes()`, the source of
//! truth for these px values) are a fixed rem scale that does **not** line
//! up 1:1 with shadcn's `calc()`-derived values:
//! - `.rounded_sm()` → 4px (0.25rem)
//! - `.rounded_md()` → 6px (0.375rem)
//! - `.rounded_lg()` → 8px (0.5rem)
//! - `.rounded_xl()` → 12px (0.75rem)
//! - `.rounded_2xl()` → 16px (1rem)
//!
//! The constants below are **documentation only** — plain `&str` labels
//! naming which GPUI builder method to call for each shadcn step, not
//! callable radius values themselves. `RADIUS_SM`/`RADIUS_MD` line up with
//! an exact GPUI step; `RADIUS_LG`/`RADIUS_XL` don't have an exact match and
//! are pinned to the nearest larger step so the four constants stay
//! strictly ordered smallest-to-largest instead of two of them colliding on
//! the same builder. New or aligned components should call the matching
//! builder directly rather than inventing ad-hoc pixel radii; this module
//! exists purely as the reference mapping.

/// shadcn `--radius-sm` (6px). GPUI: `.rounded_md()` — exact match (6px).
pub const RADIUS_SM: &str = "rounded_md";

/// shadcn `--radius-md` (8px). GPUI: `.rounded_lg()` — exact match (8px).
pub const RADIUS_MD: &str = "rounded_lg";

/// shadcn `--radius-lg` / base `--radius` (10px). GPUI: `.rounded_xl()` —
/// no exact 10px step exists; this is the nearest larger one (12px).
pub const RADIUS_LG: &str = "rounded_xl";

/// shadcn `--radius-xl` (14px). GPUI: `.rounded_2xl()` — no exact 14px step
/// exists; this is the nearest larger one (16px), kept distinct from
/// [`RADIUS_LG`] so the two shadcn steps don't collide on one GPUI builder.
pub const RADIUS_XL: &str = "rounded_2xl";

/// Full pill — shadcn badges/chips. GPUI: `.rounded_full()`.
pub const RADIUS_FULL: &str = "rounded_full";
