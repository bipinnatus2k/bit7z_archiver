---
name: example-development
description: How to create, structure, and maintain example applications in the gpui-component project. Covers the complete example creation workflow: directory layout, Cargo.toml setup, entry point pattern with gpui_platform::application(), gpui_component::init() placement, window creation with async spawn, Root wrapping, asset handling with gpui-component-assets, state management patterns for examples (unit struct vs Entity-based), subscription/callback wiring, and the one-example-one-feature principle. Use when creating a new example in the examples/ directory or modifying an existing one.
---

# Example Development

## Principl: One Example, One Feature

Each example demonstrates exactly one feature or component. This makes it easier for users to understand and adapt the code.

Examples live in `examples/<name>/` with:
```
examples/<name>/
├── Cargo.toml
├── src/
│   └── main.rs         (single file, no nested modules)
```

---

## Entry Point Pattern (Mandatory Structure)

Every example follows this exact structure:

```rust
fn main() {
    // [Choice A] Without assets (no icons/images):
    let app = gpui_platform::application();

    // [Choice B] With assets (icons, images):
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        // [1] MUST call init first — before any window is opened
        gpui_component::init(cx);

        // [2] Optional: configure window options
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(800.), px(600.)), cx)),
            ..Default::default()
        };

        // [3] Open window in async context
        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                // [4] Create the main view
                let view = cx.new(|_| Example);

                // [5] MUST wrap in Root — it's the top-level view
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();  // detach the async future
    });
}
```

### Rule Summary

| Step | Code | Required? |
|------|------|-----------|
| Create application | `gpui_platform::application()` | Always |
| Add assets | `.with_assets(Assets)` | If using icons/images |
| Init GPUI Component | `gpui_component::init(cx)` | **Always, first in run()** |
| Async window | `cx.spawn(async move { ... }).detach()` | Always |
| Create view | `cx.new(\|_\| Example)` | Always |
| Wrap in Root | `cx.new(\|cx\| Root::new(view, window, cx))` | **Always, top-level** |

---

## Cargo.toml Template

### Basic (no assets)
```toml
[package]
name = "example_name"
description = "A brief description of what this example demonstrates."
version = "0.5.1"
publish = false
edition.workspace = true

[dependencies]
anyhow.workspace = true
gpui.workspace = true
gpui_platform.workspace = true
gpui-component = { workspace = true }

[lints]
workspace = true
```

### With assets (icons, images)
```toml
[dependencies]
gpui-component-assets = { workspace = true }
```

### With custom binary name
```toml
[[bin]]
name = "example_name"
path = "src/main.rs"
```
This is only needed if the example name doesn't match the crate name.

---

## Example Categories

### Stateless Examples (unit struct)

For simple examples that don't manage persistent state:

```rust
pub struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}
```

**Examples:** `hello_world`, `dialog_overlay`

### Stateful Examples (fields on struct)

For examples that manage state:

```rust
pub struct Example {
    count: u32,
}

impl Example {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .child(format!("Count: {}", self.count))
            .child(
                Button::new("increment")
                    .label("+1")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();  // trigger re-render
                    })),
            )
    }
}
```

**Examples:** `sidebar`, `focus_trap`

### Entity-Based Examples (with subscriptions)

For complex examples with cross-component state:

```rust
pub struct Example {
    input_state: Entity<InputState>,
    display_text: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Enter name"));

        let _subscriptions = vec![cx.subscribe_in(&input_state, window, {
            let input_state = input_state.clone();
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    this.display_text = format!("Hello, {}!", input_state.read(cx).value()).into();
                    cx.notify()
                }
                _ => {}
            }
        })];

        Self { input_state, display_text: SharedString::default(), _subscriptions }
    }
}
```

**Examples:** `input`

---

## Component Composition Examples

### Button + Dialog
```rust
Button::new("show-dialog")
    .outline()
    .label("Open Dialog")
    .on_click(cx.listener(|_, _, window, cx| {
        window.open_dialog(cx, move |dialog, _, _| {
            dialog.title("Dialog Title")
                .child("Dialog content")
        });
    })),
```

### Using Root Layers for Overlays
When using Root's overlay layers outside of the Root element:
```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .child(main_content)
        .children(Root::render_dialog_layer(window, cx))
        .children(Root::render_sheet_layer(window, cx))
        .children(Root::render_notification_layer(window, cx))
}
```

### Show Notifications
```rust
window.push_notification("Message text", cx);

// With Notification struct
let note = Notification::new()
    .message("Action completed.")
    .id::<MyAction>();
window.push_notification(note, cx);
```

---

## Full Example Reference: Input

This example (`examples/input/src/main.rs`) demonstrates the state pattern, subscriptions, and assets usage:

```rust
use gpui::*;
use gpui_component::{
    input::{Input, InputEvent, InputState},
    *,
};
use gpui_component_assets::Assets;

pub struct Example {
    input_state: Entity<InputState>,
    display_text: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Enter your name"));
        let _subscriptions = vec![cx.subscribe_in(&input_state, window, {
            let input_state = input_state.clone();
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = input_state.read(cx).value();
                    this.display_text = format!("Hello, {}!", value).into();
                    cx.notify()
                }
                _ => {}
            }
        })];
        Self { input_state, display_text: SharedString::default(), _subscriptions }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .p_5()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child(Input::new(&self.input_state))
            .child(self.display_text.clone())
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(800.), px(600.)), cx)),
            ..Default::default()
        };
        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| Example::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            }).expect("Failed to open window");
        }).detach();
    });
}
```

---

## Running Examples

```bash
# Run any example
cargo run --example <name>

# Specific examples
cargo run --example hello_world
cargo run --example input
cargo run --example sidebar
cargo run --example system_monitor

# Build only (no run)
cargo build --example <name>
```

---

## Styling Tips for Examples

1. **Use `h_flex()` and `v_flex()`** for layout — these are convenience aliases from gpui-component
2. **Container layout:** Use `div().size_full().items_center().justify_center()` to center content
3. **Gap:** Use `.gap_2()`, `.gap_4()`, `.gap_6()` for spacing
4. **Padding:** Use `.p_4()`, `.p_5()`, `.p_8()` for container padding
5. **Theme colors:** Always use `cx.theme().xxx` instead of hardcoded colors
6. **Conditional styles:** Use `.when(condition, \|this\| this.style())`

---

## Example Checklist

When creating a new example:

- [ ] Does it demonstrate exactly one feature?
- [ ] Is `gpui_component::init(cx)` called first in `run()`?
- [ ] Is the first window view wrapped in `Root::new()`?
- [ ] Are imports clean (no unused imports)?
- [ ] Is `gpui_component_assets::Assets` used (if icons/images needed)?
- [ ] Does `Cargo.toml` have `edition.workspace = true` and `[lints] workspace = true`?
- [ ] Are component IDs unique within the example?
- [ ] Does the example build and run without errors?
