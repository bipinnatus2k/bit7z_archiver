---
name: component-development
description: How to create, modify, and maintain UI components in the gpui-component library. Covers the three component categories (stateless with RenderOnce, stateful with Entity+Render, composite with composition), the builder pattern, trait implementations (IntoElement, Styled, ParentElement, RenderOnce, Sizable, Disableable, Selectable), StyleRefinement usage, callback patterns using Rc<dyn Fn>, element IDs for interactivity, event binding, theme integration via cx.theme(), and component registration. Use when adding a new component to crates/ui/src or significantly modifying an existing component.
---

# Component Development

## Decision Flow: Which Component Type?

When creating a new component, use this decision tree:

```
Does the component manage mutable data that persists across renders?
├── YES → Is the data complex (text input, selection, table)?
│   ├── YES → Stateful component: Entity<State> + RenderOnce element
│   └── NO  → Stateless component with RenderOnce
└── NO → Is this a pre-configured combination of existing components?
    ├── YES → Composite component (delegate to existing components)
    └── NO → Stateless component with RenderOnce
```

| Type | Traits | State | File Location | Examples |
|------|--------|-------|---------------|----------|
| Stateless | `IntoElement`, `Styled`, `ParentElement`, `RenderOnce` | All fields on struct | Single `.rs` or directory | `Button`, `Badge`, `Separator`, `Table`, `Dialog` |
| Stateful | Same as stateless, but state is `Entity<StateType>` | `Entity<StateType>` held as field | Directory (e.g., `input/`, `select/`) | `Input`, `Select`, `DataTable`, `ColorPicker` |
| Composite | Varies (delegates to base) | Depends on base component | Single `.rs` or `window_ext.rs` | `AlertDialog`, `DropdownButton`, `ButtonGroup` |

---

## Stateless Component Pattern (RenderOnce)

This is the most common pattern. The component is a struct with configuration fields, implementing `IntoElement` (derive macro), `Styled`, `ParentElement` (optional), and `RenderOnce`.

### Minimal Stateless Component

```rust
use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
};
use crate::StyledExt;

#[derive(IntoElement)]
pub struct MyComponent {
    id: ElementId,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MyComponent {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    // Builder methods
    pub fn some_config(mut self, value: bool) -> Self {
        self.config = value;
        self
    }
}

impl ParentElement for MyComponent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MyComponent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MyComponent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            // Default styles first...
            .rounded(cx.theme().radius)
            .bg(cx.theme().tokens.background)
            // Then merge user styles (so user overrides defaults)
            .refine_style(&self.style)
            // Then children
            .children(self.children)
    }
}
```

### Key Points

1. **`#[derive(IntoElement)]`** — This is a GPUI derive macro that converts the struct into a GPUI element. It is required on every component.

2. **`style: StyleRefinement`** field — Store user-applied styles. Initialized as `Default::default()`.

3. **`Styled` trait** — Return `&mut self.style`. This allows callers to chain styling methods like `.bg()`, `.rounded()`, `.text_color()`.

4. **`refine_style(&self.style)`** in render — Place this **after** default styles but **before** `.children()`. This ensures user styles override component defaults but are then overridden by nothing.

5. **`ParentElement` trait** — Required for components that accept child elements. `extend()` stores children in `Vec<AnyElement>`.

6. **`ElementId`** — Every interactive component needs a unique ID for focus management, state lookup, and testability. Pass it in construction.

### Builder Method Conventions

All configuration methods follow this pattern:
```rust
pub fn method_name(mut self, value: Type) -> Self {
    self.field = value;
    self
}
```

**Naming:** Use clear, descriptive names matching shadcn/ui conventions:
- `label()` / `icon()` / `tooltip()` for content
- `primary()` / `danger()` / `ghost()` for variants (via `ButtonVariants` trait)
- `disabled()` / `selected()` for states
- `on_click()` / `on_hover()` for event handlers

**Do NOT add comments** to builder methods unless the existing code has them.

---

## Stateful Component Pattern

Stateful components separate state management from rendering. The state lives in an `Entity<StateType>`, and the element reads from it during `render()`.

### State Entity Pattern (e.g., InputState, SelectState, TableState)

```rust
// state.rs — the managed state
pub struct MyState {
    value: SharedString,
    focus_handle: FocusHandle,
}

impl MyState {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            value: SharedString::default(),
            focus_handle: cx.focus_handle(),
        }
    }

    // Read methods
    pub fn value(&self) -> SharedString { self.value.clone() }

    // Mutation methods
    pub fn set_value(&mut self, value: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.value = value.into();
        cx.emit(MyEvent::Change);
    }

    pub fn focus_handle(&self) -> FocusHandle { self.focus_handle.clone() }
}
```

### Stateful Element

```rust
// my_component.rs
#[derive(IntoElement)]
pub struct MyComponent {
    state: Entity<MyState>,
    style: StyleRefinement,
    disabled: bool,
}

impl MyComponent {
    pub fn new(state: &Entity<MyState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for MyComponent { ... }

impl RenderOnce for MyComponent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let value = state.value();

        div()
            .child(value.to_string())
            // ... render using state
    }
}
```

### Subscription Pattern (Change Notifications)

Stateful components typically emit events that parent views subscribe to:

```rust
// In the parent view:
let _subscriptions = vec![
    cx.subscribe_in(&input_state, window, |this, _, event: &InputEvent, _window, cx| {
        match event {
            InputEvent::Change => {
                this.display_text = input_state.read(cx).value();
                cx.notify();
            }
            _ => {}
        }
    }),
];
```

**Important:** Store subscriptions in the parent entity to keep them alive. If the parent is dropped, subscriptions are cleaned up automatically.

### Event Emission in State

```rust
// MyState method
pub fn set_value(&mut self, value: &str, window: &mut Window, cx: &mut App) {
    self.value = value.into();
    cx.emit(MyEvent::Change);
}
```

### Feature-gated State

When state structs differ by cargo feature, use `#[cfg(feature = "time")]`:

```rust
#[cfg(feature = "time")]
pub struct MyStory {
    pub date: Entity<DatePickerState>,
    // ... time-dependent fields
}

#[cfg(not(feature = "time"))]
pub struct MyStory {
    // ... without date field
}
```

---

## Composite Component Pattern

Composite components wrap existing components with pre-configured settings.

### Simple Composite (via WindowExt)

```rust
// In window_ext.rs
pub trait WindowExt {
    fn open_my_component(&mut self, cx: &mut App, config: Config);
}

impl WindowExt for Window {
    fn open_my_component(&mut self, cx: &mut App, config: Config) {
        self.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title("My Component")
                .child(config.build_content())
        });
    }
}
```

### Wrapper Composite

```rust
pub struct MyComposite {
    inner: Button,  // delegate to base component
    extra_config: bool,
}

impl MyComposite {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            inner: Button::new(id).primary().label("Default"),
            extra_config: false,
        }
    }
}
```

---

## Theme Integration

Components access theme colors through `cx.theme()` (ActiveTheme trait):

### Semantic Token Colors

The `ThemeTokens` struct provides semantic colors:

| Token | Purpose |
|-------|---------|
| `cx.theme().tokens.background` | Main background |
| `cx.theme().tokens.foreground` | Main foreground/text |
| `cx.theme().tokens.muted` | Muted background |
| `cx.theme().tokens.muted_foreground` | Muted foreground |
| `cx.theme().tokens.primary` / `primary_foreground` | Primary brand |
| `cx.theme().tokens.secondary` / `secondary_foreground` | Secondary brand |
| `cx.theme().tokens.danger` / `danger_foreground` | Danger/error |
| `cx.theme().tokens.warning` / `warning_foreground` | Warning |
| `cx.theme().tokens.success` / `success_foreground` | Success |
| `cx.theme().tokens.info` / `info_foreground` | Info |
| `cx.theme().tokens.border` | Default border |
| `cx.theme().tokens.input` | Input border/bg |
| `cx.theme().tokens.ring` | Focus ring |
| `cx.theme().tokens.popover` | Popover/dropdown bg |

### Direct Colors

Accessible via `Deref<Target=ThemeColor>`:
- `cx.theme().background`, `cx.theme().foreground`
- `cx.theme().primary`, `cx.theme().secondary`
- `cx.theme().muted`, `cx.theme().accent`
- `cx.theme().border`, `cx.theme().input`, `cx.theme().ring`
- `cx.theme().radius`, `cx.theme().radius_lg` (border radii in Pixels)
- `cx.theme().shadow` (bool: whether shadows are enabled)

### Button Token Colors

| Token | Normal | Hover | Active |
|-------|--------|-------|--------|
| Default | `tokens.button` | `tokens.button_hover` | `tokens.button_active` |
| Primary | `tokens.button_primary` | `tokens.button_primary_hover` | `tokens.button_primary_active` |
| Danger | `tokens.button_danger` | `tokens.button_danger_hover` | `tokens.button_danger_active` |

### Color Manipulation

Use the `Colorize` trait for color operations:
```rust
cx.theme().foreground.opacity(0.5)
cx.theme().primary.mix_oklab(cx.theme().transparent, 0.2)
cx.theme().secondary.lighten(0.1)
cx.theme().secondary.darken(0.1)
```

---

## Event Handling in Components

### Click Handler Pattern

```rust
pub fn on_click(
    mut self,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Self {
    self.on_click = Some(Rc::new(handler));
    self
}
```

Store as `Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>`.

In render:
```rust
.when_some(self.on_click, |this, handler| {
    this.on_click(move |event, window, cx| {
        handler(event, window, cx);
    })
})
```

### Keyboard Binding Pattern

In `init()` (module-level):
```rust
pub fn init(cx: &mut App) {
    actions!(my_component, [MyAction]);
    cx.bind_keys([
        KeyBinding::new("escape", MyAction, None),
    ]);
}
```

In the component:
```rust
.on_action(|_: &MyAction, window, cx| {
    // handle action
})
```

### Stored Callback Type Guidelines

| Use case | Type |
|----------|------|
| Simple click | `Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>` |
| Click with return value | `Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool>` |
| Generic event | `Rc<dyn Fn(&EventType, &mut Window, &mut App)>` |
| No args | `Rc<dyn Fn(&mut Window, &mut App)>` |

---

## Interactivity and Focus

### Adding Focus Support

```rust
// In state or component struct:
focus_handle: FocusHandle,

// In constructor:
focus_handle: cx.focus_handle(),

// In render:
.track_focus(&self.focus_handle)
// or with tab index:
.track_focus(&self.focus_handle.tab_index(0).tab_stop(true))
```

### Focus Ring

Use the `FocusableExt` trait from `crates/ui/src/styled.rs`:
```rust
.focus_ring(is_focused, px(0.), window, cx)
```

This renders an outer ring matching the component's border radius, color `cx.theme().ring`.

### Preventing Default Mouse Behavior

```rust
.on_mouse_down(MouseButton::Left, |_, window, cx| {
    window.prevent_default();  // prevent text selection
    crate::global_state::GlobalState::suppress_text_selection(cx);
})
```

---

## Size Integration

Implement `Sizable` for components that support xs/sm/md/lg:

```rust
impl Sizable for MyComponent {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
```

Use the `StyleSized` trait for concrete size values:
```rust
// In render:
.input_size(self.size)      // full input sizing
.input_text_size(self.size)  // just the text size
.input_h(self.size)          // just the height
.size_with(self.size)        // generic size (h_5/h_6/h_8/h_11)
```

---

## Variant System

For components with visual variants, use an enum + trait:

```rust
pub trait MyVariants: Sized {
    fn with_variant(self, variant: MyVariant) -> Self;
    fn default(self) -> Self { self.with_variant(MyVariant::Default) }
    fn special(self) -> Self { self.with_variant(MyVariant::Special) }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum MyVariant {
    #[default]
    Default,
    Special,
    Custom(MyCustomVariant),
}
```

The variant enum methods compute colors based on `cx.theme()` and `outline` boolean, with `normal()`, `hovered()`, `active()`, `selected()`, and `disabled()` style computations.

---

## Disableable and Selectable Traits

```rust
impl Disableable for MyComponent {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for MyComponent {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    fn is_selected(&self) -> bool { self.selected }
}
```

---

## Module Registration

After creating a component file:

### Single-file component
Add to `crates/ui/src/lib.rs`:
```rust
pub mod my_component;
```

### Directory component
Add to `crates/ui/src/lib.rs`:
```rust
pub mod my_component;
```
The `mod.rs` inside the directory re-exports:
```rust
mod my_component;
pub use my_component::*;
```

### Component that needs init()
If the component needs keybinding registration or global state, add an `init(cx: &mut App)` function and register it in `crates/ui/src/lib.rs`'s `init()` function:
```rust
pub fn init(cx: &mut App) {
    // ...
    my_component::init(cx);
}
```

---

## Tooltip Integration

Components that need tooltip support follow this pattern:

```rust
// Simple text tooltip
.tooltip("Click to save")

// Tooltip with keybinding display
.tooltip_with_action("Save", &SaveAction, None)

// Custom tooltip builder (advanced)
.managed_tooltip(move |window, cx| {
    Tooltip::new("Custom tooltip content".into())
        .build(window, cx)
})
```

The `ManagedTooltipExt` trait (from `crates/ui/src/tooltip.rs`) provides `.managed_tooltip()` on any element.

---

## Conditional Rendering

Use `.when()` and `.when_some()` from GPUI's `FluentBuilder`:

```rust
.when(condition, |this| this.some_style())
.when(!condition, |this| this.other_style())
.when_some(optional_value, |this, value| this.child(value.to_string()))
```

---

## Hover and Active States

For interactive components that change style on hover/active:

```rust
.bg(normal_color)
.hover(|this| this.bg(hover_color))
.active(|this| this.bg(active_color))
```

---

## Logging and Debug

Use the `tracing` crate for debug logging:
```rust
tracing::trace!("Message");    // verbose debug
tracing::debug!("Message");    // debug
tracing::info!("Message");     // info
tracing::warn!("Message");     // warning
tracing::error!("Message");    // error
```

### Performance Measurement

```rust
// Simple measurement
gpui_component::measure("operation_name", || {
    // code to measure
});

// Conditional measurement
gpui_component::measure_if("operation_name", condition, || {
    // code to measure
});
```
Requires `GPUI_MEASUREMENTS=1` or `ZED_MEASUREMENTS=1` env variable.

---

## AlertDialog Reference Example

The AlertDialog component (`crates/ui/src/dialog/alert_dialog.rs`) demonstrates composite + WindowExt patterns:

```rust
// Usage
window.open_alert_dialog(cx, |alert, _, _| {
    alert.title("Warning")
        .description("You have unsaved changes.")
        .show_cancel(true)
        .on_confirm(|_, window, cx| {
            window.push_notification("Confirmed", cx);
            true
        })
});
```

Key design decisions:
- `description` uses `SharedString` instead of `AnyElement` because dialog builders need `Fn` (callable multiple times), and `AnyElement` is not Clone
- Center-aligned layout (icon on top, not left)
- Center-aligned footer (not right-aligned like Dialog)
- Implementation lives in `window_ext.rs` as a convenience method on `Window`

---

## Button Reference Structure

The Button component (`crates/ui/src/button/button.rs`) is the most feature-complete stateless component. Key patterns to reference:

- `ButtonVariant` enum + `ButtonVariants` trait for variant system
- `ButtonRounded` enum for border radius control
- `ButtonCustomVariant` builder for fully custom styles
- `ButtonVariantStyle` struct computed per state (normal/hovered/active/selected/disabled)
- `StyleRefinement` for user style merging
- `FocusableExt::focus_ring()` for focus indication
- `stateful` div via `base: Stateful<Div>` for interactivity
- `on_mouse_down` for preventing text selection
- `managed_tooltip` integration
- Loading state with spinner icon overlay

---

## Dialog Structure Reference

The Dialog component (`crates/ui/src/dialog/`) demonstrates the composite sub-element pattern:

| File | Purpose |
|------|---------|
| `dialog.rs` | Main Dialog container with trigger/content/header/footer |
| `header.rs` | DialogHeader (wraps title + description) |
| `footer.rs` | DialogFooter (wraps action buttons) |
| `title.rs` | DialogTitle |
| `description.rs` | DialogDescription |
| `content.rs` | DialogContent |
| `alert_dialog.rs` | Pre-configured AlertDialog composite |

Dialog is a **stateless** component — state is managed by Root through `window.open_dialog()`. It uses `Animation::new()` with `cubic_bezier` easing for slide-down/fade-in transitions.
