---
name: testing-and-quality
description: Testing practices, code quality checks, and CI commands for the gpui-component project. Covers the testing philosophy (simplicity first, builder pattern tests, complex logic testing), GPUI test infrastructure (TestAppContext, add_empty_window, gpui::test attribute), component test patterns (builder assertions, clickable logic, variant methods, gradient/token tests), lint tooling (cargo clippy, cargo fmt, typos, cargo machete), and verification workflow before completion. Use when writing tests for components, running quality checks, or verifying code changes.
---

# Testing & Quality

## Testing Philosophy

The project follows principles from `.claude/COMPONENT_TEST_RULES.md`:

1. **Simplicity First** — Focus tests on complex logic and core functionality. Avoid excessive tests for trivial getters/setters.
2. **Builder Pattern Testing** — Every component should have a `test_*_builder` test covering the builder pattern.
3. **Complex Logic Testing** — Test conditional branching, state transitions, and edge cases.

**Per user configuration:** Tests do not need to be run as part of the normal development workflow. They are run explicitly when needed.

---

## Test Infrastructure

The project uses GPUI's built-in test support:

```toml
# In Cargo.toml
[dev-dependencies]
gpui = { workspace = true, features = ["test-support"] }
```

### Test Attribute

GPUI provides the `#[gpui::test]` attribute which sets up `TestAppContext`:

```rust
#[gpui::test]
fn test_button_builder(_cx: &mut gpui::TestAppContext) {
    // Test code here
}
```

For tests that need the theme system initialized:
```rust
#[gpui::test]
fn test_with_init(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);  // Initialize gpui-component
    let window = cx.add_empty_window();
    window.update(|_, cx| {
        // Access theme: cx.theme()
    });
}
```

---

## Component Test Patterns

### Builder Pattern Test

Every component needs a test that verifies all builder methods work correctly:

```rust
#[gpui::test]
fn test_button_builder(_cx: &mut gpui::TestAppContext) {
    let button = Button::new("complex-button")
        .label("Save Changes")
        .primary()
        .outline()
        .large()
        .tooltip("Click to save")
        .compact()
        .loading(false)
        .disabled(false)
        .selected(false)
        .tab_index(1)
        .tab_stop(true)
        .dropdown_caret(false)
        .rounded(ButtonRounded::Medium)
        .on_click(|_, _, _| {});

    assert_eq!(button.label, Some("Save Changes".into()));
    assert_eq!(button.variant, ButtonVariant::Primary);
    assert!(button.outline);
    assert_eq!(button.size, Size::Large);
    assert!(button.tooltip.is_some());
    assert!(button.compact);
    assert!(!button.loading);
    assert!(!button.disabled);
    assert!(!button.selected);
    assert_eq!(button.tab_index, 1);
    assert!(button.tab_stop);
    assert!(!button.dropdown_caret);
    assert!(matches!(button.rounded, ButtonRounded::Medium));
}
```

### Clickable/Disabled Logic Test

Test state transitions and conditional behavior:

```rust
#[gpui::test]
fn test_button_clickable_logic(_cx: &mut gpui::TestAppContext) {
    let clickable = Button::new("test").on_click(|_, _, _| {});
    assert!(clickable.clickable());

    let disabled = Button::new("test").disabled(true).on_click(|_, _, _| {});
    assert!(!disabled.clickable());

    let loading = Button::new("test").loading(true).on_click(|_, _, _| {});
    assert!(!loading.clickable());
}
```

### Variant Method Tests

Test enum variant predicates and helper methods:

```rust
#[gpui::test]
fn test_button_variant_methods(_cx: &mut gpui::TestAppContext) {
    assert!(ButtonVariant::Link.is_link());
    assert!(ButtonVariant::Text.is_text());
    assert!(ButtonVariant::Ghost.is_ghost());
    assert!(ButtonVariant::Link.no_padding());
    assert!(ButtonVariant::Text.no_padding());
    assert!(!ButtonVariant::Ghost.no_padding());
}
```

### Theme-Aware Tests

For components that depend on theme tokens:

```rust
#[gpui::test]
fn test_primary_button_uses_gradient_background_tokens(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_empty_window();
    window.update(|_, cx| {
        let config = serde_json::from_value::<ThemeConfig>(serde_json::json!({
            "name": "Gradient",
            "mode": "light",
            "colors": {
                "button.primary.background": "linear-gradient(135deg, #4F46E5, #06B6D4)",
                "button.primary.hover.background": "linear-gradient(145deg, #4338CA, #0891B2)",
            }
        })).unwrap();
        Theme::global_mut(cx).apply_config(&Rc::new(config));

        assert_eq!(
            ButtonVariant::Primary.normal(false, cx).bg,
            cx.theme().tokens.button_primary.into()
        );
        assert_eq!(
            ButtonVariant::Primary.hovered(false, cx).bg,
            cx.theme().tokens.button_primary_hover.into()
        );
    });
}
```

### Outline Style Tests

Test outline vs filled behavior:

```rust
#[gpui::test]
fn test_outline_selected_uses_outline_active_style(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let window = cx.add_empty_window();
    window.update(|_, cx| {
        let variant = ButtonVariant::Danger;
        let active_style = variant.active(true, cx);
        let selected_style = variant.selected(true, cx);
        assert_eq!(selected_style.bg, active_style.bg);
        assert_ne!(selected_style.bg, cx.theme().tokens.danger_active.into());
    });
}
```

### Unit Tests for Utility Types

Test Size conversions, `as_str()`, `from_str()`, `max()`, `min()`:

```rust
#[test]
fn test_size_max_min() {
    assert_eq!(Size::Small.min(Size::XSmall), Size::Small);
    assert_eq!(Size::XSmall.min(Size::Small), Size::Small);
    assert_eq!(Size::Large.max(Size::Small), Size::Small);
}

#[test]
fn test_size_as_str() {
    assert_eq!(Size::XSmall.as_str(), "xs");
    assert_eq!(Size::Medium.as_str(), "md");
}

#[test]
fn test_size_from_str() {
    assert_eq!(Size::from_str("xs"), Size::XSmall);
    assert_eq!(Size::from_str("lg"), Size::Large);
    assert_eq!(Size::from_str("unknown"), Size::Medium);
}
```

---

## Command Reference

### Running Tests

```bash
# Run all tests
cargo test --all

# Run tests for the UI crate
cargo test -p gpui-component

# Run doc tests
cargo test -p gpui-component --doc

# Run a specific test
cargo test -p gpui-component test_button_builder

# Run tests with output
cargo test -p gpui-component -- --nocapture
```

### Lint Checks

```bash
# Clippy (must pass with --deny warnings)
cargo clippy -- --deny warnings

# Format check
cargo fmt --check

# Format (auto-fix)
cargo fmt

# Spell check
typos

# Unused dependency check
cargo machete
```

### Full Verification (before commit/PR)

Run all checks:
```bash
cargo clippy -- --deny warnings && cargo fmt --check && typos && cargo machete
```

---

## Code Quality Rules

### Denied Lints

These cause compilation failures:
- `dbg_macro = "deny"` — never commit `dbg!()` calls
- `todo = "deny"` — never commit `todo!()` macros

### Allowed Lints

These are explicitly allowed (don't worry about them):
- `style` — all style lints
- `type_complexity` — complex types are OK
- `module_inception` — nested modules are OK

### Import Order

Follow the conventions from existing files. Typically:
1. `std` imports
2. `gpui` imports
3. `gpui_component` imports  
4. External crate imports
5. `crate::` imports

### File Naming

- Component files: lowercase with underscores (`button_group.rs`, `alert_dialog.rs`)
- Story files: `<component>_story.rs` (`button_story.rs`)
- Module directories: lowercase (`button/`, `dialog/`, `input/`)

---

## Verification Before Completion

Before claiming work is complete:
1. Run `cargo clippy -- --deny warnings` — fix all warnings
2. Run `cargo fmt --check` — format all code
3. Run `typos` — fix any spelling errors
4. Run `cargo machete` — fix unused dependencies
