# Settings UI, FS Crate & Theme System Design

Date: 2026-07-12

## Overview

Port and adapt selected patterns from Zed's settings system, file system abstraction, and theme system into `bit7z_archiver`. The goal is to get a working settings UI with typed controls, file-watching auto-reload, and a functional theme system.

## Phase 1: Trim `fs` Crate

**Current state:** The `crates/application/fs/` crate is a direct copy from Zed. It fails to compile due to 7 missing internal crate deps and 4 missing external crate deps.

**Trim strategy:**

| Action | Details |
|---|---|
| Remove `git::repository::*` | Delete `open_repo()` from trait and impl |
| Remove `trash::*` | Delete trash/restore/TrashItem entire feature |
| Remove `async_tar::Archive` | Delete `extract_tar_file()` from trait and impl |
| Remove `telemetry::event!` | Delete single call site |
| Remove `proto::Timestamp` | Delete commented-out code |
| Remove `collections` | Delete test-support code blocks |
| Remove test-support feature | Delete `#[cfg(feature = "test-support")]` blocks entirely |
| Replace `util::normalize_path` | Inline: `fn normalize_path(path: &Path) -> PathBuf` that resolves `.`, `..`, double separators |
| Replace `paths::temp_dir()` | `std::env::temp_dir()` |
| Replace `util::maybe` | Inline `Option::and_then` / `if let` patterns |
| Replace `util::command::new_command` | `std::process::Command::new` |
| Replace `util::extend_sorted` | Inline simple merge into vec |
| Replace `util::ResultExt` | Inline `log_err` as `inspect_err(|e| log::error!("{e}"))` |
| Replace `rope::Rope` | `String` |
| Replace `text::LineEnding` | Simple `enum LineEnding { Native, Lf, CrLf }` defined in fs crate |
| Replace `text::chunks_with_line_ending` | Simple `str::split_inclusive` based impl |
| Keep `notify` | Uncomment `notify = "9.0.0-rc.4"` |
| Keep `libc` | Uncomment `libc.workspace = true` (Unix-only, harmless on Windows behind `#[cfg(unix)]`) |

**Resulting Cargo.toml dependencies:**

```toml
[dependencies]
anyhow.workspace = true
async-channel.workspace = true
async-trait.workspace = true
futures.workspace = true
gpui.workspace = true
ignore.workspace = true
log.workspace = true
parking_lot.workspace = true
serde.workspace = true
serde_json.workspace = true
smol.workspace = true
tempfile.workspace = true
thiserror.workspace = true
notify = "9.0.0-rc.4"
is_executable = "1.0.5"

[target.'cfg(unix)'.dependencies]
libc.workspace = true

[target.'cfg(target_os = "windows")'.dependencies]
windows.workspace = true
dunce.workspace = true
```

## Phase 2: Settings System

### Architecture

```
gpui-component::setting::Settings  (UI shell)
         │ getter/setter closures
         ▼
SettingFieldRenderer (adapter: TypeId → SettingField<T> factory)
         │
         ▼
SettingsStore (Global<ArchiverSettings> + serde_json + fs_watcher)
```

### UI Layer: gpui-component `setting::Settings`

Use as-is. Provides:
- Sidebar navigation with page selection
- Search filtering (by title, description, keywords)
- Page → Group → Item hierarchy
- Item-level reset-to-default

### Controls (ported from Zed)

#### NumberField (from `zed/settings_ui/components/number_field.rs`)

Ported features:
- Read/Edit dual mode (click to edit, blur to commit)
- Shift/Alt large/small step modifiers
- Generic type support via `NumberFieldType` trait (f32, f64, u32, u64, i32, i64, usize, NonZero*)

Integration: Wrap as `SettingFieldElement`:
```rust
impl SettingFieldElement for NumberField<T> {
    type Element = NumberFieldView;
    fn render_field(&self, options, window, cx) -> Self::Element;
}
```

#### EnumVariantDropdown (from `zed/settings_ui/components/dropdown.rs`)

Ported features:
- Generic over any enum that implements `strum::VariantArray + strum::VariantNames`
- Renders as `PopoverMenu` (from gpui-component) with selectable items

Integration: Wrap as `SettingFieldElement`.

#### ThemePicker / FontPicker (from `zed/settings_ui/components/*.rs`)

Simplified port:
- Replace Zed's `Picker`+`PickerDelegate` framework with gpui-component `Dialog`
- ThemePicker: opens a searchable dialog listing themes from `ThemeRegistry`
- FontPicker: opens a searchable dialog listing fonts from system

Integration: via `SettingItem::render()` (fully custom element).

### SettingFieldRenderer

Adapted from Zed's pattern: a Global registry mapping `TypeId` → render function.

```rust
struct SettingFieldRenderer {
    registry: HashMap<TypeId, Box<dyn Fn(&SettingDef, &mut App) -> SettingItem>>,
}

impl SettingFieldRenderer {
    fn register<T: SettingsFieldType>(&mut self, render_fn: ...);
    fn render(&self, def: &SettingDef, cx: &mut App) -> SettingItem;
}
```

### Backend: SettingsStore

```rust
struct SettingsStore {
    settings: ArchiverSettings,
    path: PathBuf,
    watcher: Option<notify::RecommendedWatcher>,  // simple notify watcher for config file
    dirty: bool,
}
impl Global for SettingsStore;
```

File watching for the single config file is done directly with `notify` (not via `fs::fs_watcher::FsWatcher`, which is designed for the heavier Zed worktree watching pattern).

```rust
impl SettingsStore {
    fn init(cx: &mut App);            // Load from file or create defaults, start watching
    fn save(&self) -> Result<()>;     // serde_json to file
    fn load(path: &Path) -> Self;     // serde_json from file
    fn watch(&mut self, cx: &mut App); // notify watcher on config file
}
```

**Reload flow:**
```
config.json changed → notify event → SettingsStore::load() → update Global
→ emit SettingsChanged event → UI re-reads via getter closures
```

### `ArchiverSettings` Definition

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ArchiverSettings {
    // General
    default_format: ArchiveFormat,       // 7z, zip, tar, gz, etc.
    compression_level: CompressionLevel, // Store, Fastest, Normal, Maximum
    output_directory: Option<String>,

    // Appearance
    theme_name: String,                  // ThemeRegistry key
    theme_mode: ThemeMode,               // Light | Dark | System
    ui_font: Option<String>,             // font family name
    ui_font_size: Option<f32>,

    // Behavior
    confirm_before_delete: bool,
    show_hidden_files: bool,
    recent_files_max: u32,
}
```

## Phase 3: Theme System

### Current Problem

- `bit7z-pres-theme::Theme` and `gpui-component::Theme` are two separate `Global` types
- UI components use `cx.theme()` which returns `gpui-component::Theme` (uninitialized → broken)
- `bit7z-pres-theme` colors are hard-coded with no runtime loading or switching

### Solution

**Remove `crates/presentation/theme/` crate.** It is replaced by `gpui-component::theme::*` which already provides:

- `Theme` struct with 100+ semantic colors + radius/shadow/font config
- `ActiveTheme` trait (`cx.theme()` → `&Theme`)
- `ThemeRegistry` Global for listing/loading themes
- `ThemeConfig` + `default-theme.json` JSON loading
- `ThemeMode` (Light / Dark / System) with system appearance sync
- Font configuration (UI font family + size)
- `Theme::change(mode, config, cx)` for runtime switching

**Init sequence at app startup:**

```rust
fn main() {
    gpui::App::new().run(|cx| {
        gpui_platform::application::init(cx);
        gpui_component::init(cx);           // includes theme::init(cx)

        // Load persisted settings
        SettingsStore::init(cx);

        // Apply persisted theme
        if let Some(settings) = SettingsStore::try_global(cx) {
            let theme_registry = ThemeRegistry::global(cx);
            if let Some(config) = theme_registry.get(&settings.theme_name) {
                Theme::change(settings.theme_mode, Some(config.clone()), cx);
            }
        }
    });
}
```

**Font configuration:** gpui-component's `Theme` already has font fields:
- `ui_font_family: Option<SharedString>`
- `ui_font_size: Pixels`
- `buffer_font_family: Option<SharedString>`
- `buffer_font_size: Pixels`

These are set via `Theme::change()` → `apply_config()` which reads from `ThemeConfig`.

### ThemePicker UI Integration

The `ThemePicker` settings control will:
1. List all themes from `ThemeRegistry::global(cx).list()`
2. On selection: call `Theme::change(mode, config, cx)` + update `SettingsStore`
3. Persist selection to `ArchiverSettings.theme_name`

## Dependencies (new/changed)

### New workspace dependencies to add

```toml
# For theme system (already resolved via gpui-component git dep)
# No additional workspace deps needed — gpui-component provides everything

# For settings backend
serde_json.workspace = true    # already in workspace
notify = "9.0.0-rc.4"         # for fs_watcher

# For controls
strum = { version = "0.26", features = ["derive"] }  # EnumVariantDropdown
```

### Crates removed

```toml
# Remove from workspace members:
"crates/presentation/theme"
```

### Crate structure: `crates/presentation/settings/`

```
Cargo.toml
src/
├── mod.rs              # pub fn init(cx); pub use gpui_component::theme::*;
├── backend/
│   ├── mod.rs
│   ├── store.rs        # SettingsStore Global
│   ├── content.rs      # ArchiverSettings struct
│   └── watcher.rs      # fs_watcher integration
├── renderer/
│   ├── mod.rs
│   └── registry.rs     # SettingFieldRenderer
└── controls/
    ├── mod.rs
    ├── number.rs       # NumberField
    ├── dropdown.rs     # EnumVariantDropdown
    ├── theme_picker.rs # ThemePicker
    └── font_picker.rs  # FontPicker
```

## Implementation Order

1. **Phase 1: Trim fs crate** — get it compiling
2. **Phase 3 first: Replace theme system** — just use gpui-component's theme, delete old crate. This is a quick win.
3. **Phase 2: Settings system** — build backend → controls → renderer → wire up UI
