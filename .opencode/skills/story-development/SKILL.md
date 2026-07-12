---
name: story-development
description: How to create, structure, and maintain component stories in the gpui-component Story Gallery application. Covers the Story trait with title/description/new_view/on_active methods, section() helper for organized layouts, story registration in mod.rs + init(), Gallery navigation system, StoryContainer panel integration, conditional compilation with feature flags, shared state via Entity, subscription management for events, and control panels (checkboxes, button groups, switches) for interactive story state. Use when creating a new story in crates/story/src/stories/ or modifying an existing story.
---

# Story Development

## Architecture Overview

The Story Gallery (`crates/story/`) is a desktop application that showcases all components. Each component gets a **story** — an interactive demo that lives in a dock panel.

```
crates/story/src/
├── main.rs            — Binary entry point
├── lib.rs             — Library root: init(), StoryRoot, StoryContainer, section()
├── gallery.rs         — Gallery view: DockArea with all story panels
├── themes.rs          — Theme picker UI
├── title_bar.rs       — App title bar
├── stories/
│   ├── mod.rs         — Story trait definition + init() + re-exports
│   ├── button_story.rs
│   ├── dialog_story.rs
│   ├── ... (60+ stories)
```

---

## The Story Trait

Every story implements the `Story` trait defined in `crates/story/src/stories/mod.rs:155`:

```rust
pub trait Story: Render + Sized {
    /// The class name (auto-implemented via type_name)
    fn klass() -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }

    /// Display title in the panel tab
    fn title() -> &'static str;

    /// Short description shown in the gallery
    fn description() -> &'static str { "" }

    /// Whether the panel can be closed
    fn closable() -> bool { true }

    /// Zoom control configuration
    fn zoomable() -> Option<PanelControl> { Some(PanelControl::default()) }

    /// Title bar background color
    fn title_bg() -> Option<Hsla> { None }

    /// Content padding
    fn paddings() -> Pixels { px(16.) }

    /// Create the story view (Entity)
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render>;

    /// Called when panel becomes active/inactive
    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut App) {}
}
```

### Minimal Story Implementation

```rust
pub struct ButtonStory {
    focus_handle: FocusHandle,
}

impl ButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }
}

impl super::Story for ButtonStory {
    fn title() -> &'static str { "Button" }
    fn description() -> &'static str { "Displays a button or a component that looks like a button." }
    fn closable() -> bool { false }
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ButtonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ButtonStory { ... }
```

---

## The `section()` Helper

Stories organize content using the `section()` helper from `crates/story/src/lib.rs:353`:

```rust
pub(crate) fn section(title: impl Into<SharedString>) -> StorySection
```

`StorySection` is a styled group box with:
- A title bar
- A content area that centers children horizontally with flex-wrap and gap-4
- Max-width helpers: `.max_w_md()`, `.max_w_lg()`, `.max_w_xl()`, `.max_w_2xl()`
- Sub-title support via `.sub_title(element)`

**Usage pattern:**
```rust
v_flex()
    .gap_6()
    .child(
        section("Normal Buttons")
            .max_w_lg()
            .child(Button::new("btn1").label("Default"))
            .child(Button::new("btn2").primary().label("Primary"))
    )
    .child(
        section("Small Size")
            .child(Button::new("btn3").small().label("Small"))
    )
```

---

## Story Registration (CRITICAL)

Registering a story requires modifying **three files**. Do NOT stop after creating the story file — all three registrations are necessary.

### Step 1: Create the story file
Create `crates/story/src/stories/<name>_story.rs`.

### Step 2: Register in `stories/mod.rs` (REQUIRED)

Open `crates/story/src/stories/mod.rs` and add TWO things:
1. A `mod <name>_story;` declaration
2. A `pub use <name>_story::NameStory;` re-export

Add these alphabetically among the existing entries. The result should look like:

```rust
// --- Add these two lines ---
mod kbd_story;
pub use kbd_story::KbdStory;
// --- end of addition ---

mod label_story;
pub use label_story::LabelStory;
```

Feature-gated registration uses the same pattern with `#[cfg]`:
```rust
#[cfg(feature = "time")]
mod calendar_story;
#[cfg(feature = "time")]
pub use calendar_story::CalendarStory;
```

**IMPORTANT: ALWAYS produce/update the `mod.rs` file. This is the most commonly missed step.**

### Step 3: Register `init()` (if needed)

Stories with actions/keybindings need an `init()` function:

```rust
// In the story file:
pub fn init(_: &mut App) {}  // empty if no keybindings

// Then in stories/mod.rs:
pub(crate) fn init(cx: &mut App) {
    input_story::init(cx);
    combobox_story::init(cx);
    // add new story init here
    select_story::init(cx);
}
```

### Step 4: Register in StoryState (for serialization)

In `crates/story/src/lib.rs` `StoryState::to_story()`, add the match arm:

```rust
"ButtonStory" => story!(ButtonStory),
"NewStory" => story!(NewStory),  // Add here
```

### Step 5: Add to Gallery panels

In `crates/story/src/gallery.rs`, add `StoryContainer::panel::<NewStory>()` to the story panels list.

---

## Gallery Navigation System

The gallery (`crates/story/src/gallery.rs`) renders a sidebar navigation + `DockArea` for panels.

### Adding a story to the gallery

In `gallery.rs`, stories are organized into groups and added as panels:

```rust
// Example from gallery.rs structure:
fn render_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    // ... sidebar with navigation groups
}

fn story_panels(window: &mut Window, cx: &mut App) -> Vec<Entity<StoryContainer>> {
    vec![
        StoryContainer::panel::<ButtonStory>(window, cx),
        StoryContainer::panel::<InputStory>(window, cx),
        StoryContainer::panel::<DialogStory>(window, cx),
        // ...
    ]
}
```

### Opening a story in a new window

Use the `create_new_window()` helper:

```rust
create_new_window("Button", |window, cx| {
    ButtonStory::view(window, cx).into()
}, cx);
```

---

## Control Panels in Stories

Stories often include interactive controls (checkboxes, button groups, switches) that let users toggle component states.

### Checkbox Controls

```rust
.child(
    Checkbox::new("disabled-button")
        .label("Disabled")
        .checked(self.disabled)
        .on_click(cx.listener(|view, _, _, cx| {
            view.disabled = !view.disabled;
            cx.notify();
        })),
)
```

### Button Group Controls

```rust
.child(
    ButtonGroup::new("size").outline().small()
        .child(Button::new("large").selected(self.size == Size::Large).child("Large"))
        .child(Button::new("medium").selected(self.size == Size::Medium).child("Medium"))
        .on_click(cx.listener(|this, selecteds: &Vec<usize>, _, cx| {
            if selecteds.contains(&0) { this.size = Size::Large; }
            else if selecteds.contains(&1) { this.size = Size::Medium; }
            cx.notify();
        })),
)
```

### Toggle Button Groups

```rust
ButtonGroup::new("toggle-group")
    .outline()
    .multiple(true)  // allow multi-select
    .child(Button::new("opt1").label("Option 1"))
    .child(Button::new("opt2").label("Option 2"))
    .on_click(cx.listener(|this, selected: &Vec<usize>, _, cx| {
        // selected is a Vec of indices
        cx.notify();
    }))
```

### Switch Controls

```rust
Switch::new("layout")
    .checked(self.layout.is_horizontal())
    .label("Horizontal")
    .on_click(cx.listener(|this, checked: &bool, _, cx| {
        // checked is the new state
        cx.notify();
    }))
```

---

## Conditional Feature-Gated Stories

When a component is behind a cargo feature, use `#[cfg(feature = "...")]`:

```rust
// In mod.rs
#[cfg(feature = "time")]
mod calendar_story;
#[cfg(feature = "time")]
pub use calendar_story::CalendarStory;
```

In the story file, struct fields may differ by feature:

```rust
#[cfg(feature = "time")]
pub struct DialogStory {
    focus_handle: FocusHandle,
    input1: Entity<InputState>,
    date: Entity<DatePickerState>,  // time-dependent
}

#[cfg(not(feature = "time"))]
pub struct DialogStory {
    focus_handle: FocusHandle,
    input1: Entity<InputState>,
    // no date field
}
```

---

## Advanced Story Patterns

### Cross-Story Actions

Stories can define custom actions to demonstrate keyboard interaction:

```rust
#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = button_story, no_json)]
enum ButtonAction {
    Disabled,
    Loading,
    Selected,
    Compact,
}

// In render:
.on_action(cx.listener(|this, action: &ButtonAction, _, _| match action {
    ButtonAction::Disabled => this.disabled = !this.disabled,
    ButtonAction::Loading => this.loading = !this.loading,
    // ...
}))
```

### Focusable Story

Every story should implement `Focusable`:

```rust
impl Focusable for ButtonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
```

### Dynamic Story Updates

For components that need to show async or time-series data (like the chart story), use `cx.spawn()` with timers or async tasks.

### Subscriptions for Event Communication

When a story contains interactive components (like an Input) and needs to react to events, use the subscription pattern:

```rust
_subscriptions: Vec<Subscription>,

// In constructor:
_subscriptions = vec![
    cx.subscribe_in(&input_state, window, |this: &mut MyStory, _, event: &InputEvent, _window, cx| {
        match event {
            InputEvent::Change => { cx.notify(); }
            _ => {}
        }
    }),
];
```

---

## StoryState Serialization

`StoryState` (in `crates/story/src/lib.rs`) handles serialization of story panel state for layout persistence:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct StoryState {
    pub story_klass: SharedString,
}
```

When adding a new story, also add it to `StoryState::to_story()`:

```rust
fn to_story(&self, window: &mut Window, cx: &mut App) -> ... {
    match self.story_klass.to_string().as_str() {
        "ButtonStory" => story!(ButtonStory),
        "InputStory" => story!(InputStory),
        "NewStory" => story!(NewStory),  // Add here
        _ => unreachable!(),
    }
}
```

---

## Running the Story Gallery

```bash
cargo run                    # runs the story gallery (default member)
cargo run -p story           # explicit crate run
cargo run -p story-web       # WASM web version
```

---

## File Organization Summary

```
crates/story/src/stories/<name>_story.rs    → story implementation
crates/story/src/stories/mod.rs             → registration + Story trait
crates/story/src/lib.rs                     → StoryState + StoryContainer + section()
crates/story/src/gallery.rs                 → Gallery with DockArea panels
```

**Registration checklist — EVERY new story MUST check all:**
- [ ] Create `<name>_story.rs` with `Story` impl + `Focusable` + `Render`
- [ ] **MODIFY `stories/mod.rs`**: add `mod <name>_story;` + `pub use <name>_story::NameStory;`
- [ ] Add `pub fn init(_: &mut App) {}` if actions/keybindings are needed
- [ ] Register init call in `stories/mod.rs` `pub(crate) fn init()`
- [ ] **MODIFY `gallery.rs`**: add `StoryContainer::panel::<NameStory>()` to panels list
- [ ] **MODIFY `lib.rs`**: add `"NameStory" => story!(NameStory)` in `StoryState::to_story()` match
