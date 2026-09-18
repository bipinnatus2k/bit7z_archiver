//! Role-based color palette for the design system.
//!
//! ## shadcn mapping
//!
//! shadcn's `--destructive` / `--destructive-foreground` map to [`danger`]
//! (Tailwind `red` ramp). Use `danger(600)` for solid destructive backgrounds
//! and `danger(700)` for hover; pair with white foreground text on filled
//! destructive controls. Theme-aware neutrals use [`crate::styles::semantic`].
//!
//! Colors are organized by semantic ROLE (`neutral`, `primary`, `success`,
//! `warning`, `danger`, `info`), never by brand color name. Each role is a
//! `50..=950` shade ramp returning [`Hsla`].
//!
//! Hex values are sourced from the Tailwind v3 reference palette internally
//! (neutral←slate, primary/info←blue, success←green, warning←amber,
//! danger←red) — but this mapping is an implementation detail. Swapping the
//! ramps below re-themes every component without touching any call site.
//!
//! Accents/status use these swatches directly (mode-agnostic). Neutrals for
//! surfaces/borders/text should come from [`crate::styles::semantic`] instead,
//! so dark/light both work via the theme.

use gpui::{Hsla, rgb};

#[inline]
fn shade(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// Neutral ramp (source: Tailwind `slate`). Prefer `semantic::*` for
/// theme-aware surfaces/text; use this only for fixed neutral swatches.
pub fn neutral(shade_step: u16) -> Hsla {
    match shade_step {
        50 => shade(0xf8fafc),
        100 => shade(0xf1f5f9),
        200 => shade(0xe2e8f0),
        300 => shade(0xcbd5e1),
        400 => shade(0x94a3b8),
        500 => shade(0x64748b),
        600 => shade(0x475569),
        700 => shade(0x334155),
        800 => shade(0x1e293b),
        900 => shade(0x0f172a),
        950 => shade(0x020617),
        _ => shade(0x64748b),
    }
}

/// Primary accent ramp (source: Tailwind `blue`).
pub fn primary(shade_step: u16) -> Hsla {
    match shade_step {
        50 => shade(0xeff6ff),
        100 => shade(0xdbeafe),
        200 => shade(0xbfdbfe),
        300 => shade(0x93c5fd),
        400 => shade(0x60a5fa),
        500 => shade(0x3b82f6),
        600 => shade(0x2563eb),
        700 => shade(0x1d4ed8),
        800 => shade(0x1e40af),
        900 => shade(0x1e3a8a),
        950 => shade(0x172554),
        _ => shade(0x3b82f6),
    }
}

/// Informational ramp — shares the primary (blue) ramp.
pub fn info(shade_step: u16) -> Hsla {
    primary(shade_step)
}

/// Success ramp (source: Tailwind `green`).
pub fn success(shade_step: u16) -> Hsla {
    match shade_step {
        50 => shade(0xf0fdf4),
        100 => shade(0xdcfce7),
        200 => shade(0xbbf7d0),
        300 => shade(0x86efac),
        400 => shade(0x4ade80),
        500 => shade(0x22c55e),
        600 => shade(0x16a34a),
        700 => shade(0x15803d),
        800 => shade(0x166534),
        900 => shade(0x14532d),
        950 => shade(0x052e16),
        _ => shade(0x22c55e),
    }
}

/// Warning ramp (source: Tailwind `amber`).
pub fn warning(shade_step: u16) -> Hsla {
    match shade_step {
        50 => shade(0xfffbeb),
        100 => shade(0xfef3c7),
        200 => shade(0xfde68a),
        300 => shade(0xfcd34d),
        400 => shade(0xfbbf24),
        500 => shade(0xf59e0b),
        600 => shade(0xd97706),
        700 => shade(0xb45309),
        800 => shade(0x92400e),
        900 => shade(0x78350f),
        950 => shade(0x451a03),
        _ => shade(0xf59e0b),
    }
}

/// Danger/error ramp (source: Tailwind `red`).
pub fn danger(shade_step: u16) -> Hsla {
    match shade_step {
        50 => shade(0xfef2f2),
        100 => shade(0xfee2e2),
        200 => shade(0xfecaca),
        300 => shade(0xfca5a5),
        400 => shade(0xf87171),
        500 => shade(0xef4444),
        600 => shade(0xdc2626),
        700 => shade(0xb91c1c),
        800 => shade(0x991b1b),
        900 => shade(0x7f1d1d),
        950 => shade(0x450a0a),
        _ => shade(0xef4444),
    }
}
