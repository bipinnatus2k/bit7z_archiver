# OpenSpec `core-operations-completion` Implementation Plan

## Overview
This plan addresses the critical wiring gaps identified in the code review. The infrastructure is complete; we need to connect UI actions to backend operations.

---

## Phase 1: Context Menu Expansion (High Priority)

### File: `src/adapters/views/archive_file_list.rs`

**Current state (lines 172-238):** Only Extract, Refresh, Select All, Clear Selection, Checksum submenu

**Add these menu items with handlers:**
1. **Open** (Enter) → Emit `ArchiveVmEvent::RequestOpenEntry`
2. **View** (Ctrl+V) → Emit `ArchiveVmEvent::RequestViewEntry`
3. **Edit** (F4) → Emit `ArchiveVmEvent::RequestEditEntry`
4. **Test Selected** → Emit `ArchiveVmEvent::RequestTestEntries { selected_only: true }`
5. **Test All** → Emit `ArchiveVmEvent::RequestTestEntries { selected_only: false }`
6. **New Folder** → Emit `ArchiveVmEvent::RequestNewFolder`
7. **New File** → Emit `ArchiveVmEvent::RequestNewFile`
8. **Properties** (Alt+Enter) → Emit `ArchiveVmEvent::RequestProperties`

**Need to add event variants to `ArchiveVmEvent` in `archive_vm.rs`**

---

## Phase 2: Menu Bar Handlers (High Priority)

### File: `src/adapters/views/menu.rs`

**Empty handlers to wire (lines with `|_, _, _| {}`):**

| Line | Menu Item | Action |
|------|-----------|--------|
| 235 | File → New Folder | Emit `ArchiveVmEvent::RequestNewFolder` |
| 236 | File → New File | Emit `ArchiveVmEvent::RequestNewFile` |
| 263 | Edit → Copy | Copy selected entries to clipboard |
| 264 | Edit → Cut | Cut selected entries to clipboard |
| 265 | Edit → Paste | Paste entries from clipboard |
| 283-286 | View → Icon/List/Details modes | Update `ViewMode` in `ArchiveVm` |
| 288 | View → Flat View | Toggle `flat_view` in `ArchiveVm` |
| 290-293 | View → Show Toolbar/Status Bar/Preview/Tree | Toggle corresponding globals |
| 301-312 | Tools → Checksum submenu | Emit `ArchiveVmEvent::RequestChecksum { algorithm }` |
| 323 | Favorites → Add to Favorites | Add current path to favorites |
| 324 | Favorites → Organize Favorites | Open favorites dialog |
| 331 | Help → About | Show about dialog |

**Pattern:** Use `cx.emit(AppEvent::ArchiveEvent(event))` to send to root

---

## Phase 3: RootView Event Handlers (High Priority)

### File: `src/adapters/views/root.rs` (lines 61-171)

**Current empty/incomplete handlers:**

| Event | Current | Required Implementation |
|-------|---------|------------------------|
| `RequestShowAdd` (134-136) | Log only | Open `AddFilesDialog` via `cx.open_dialog()` |
| `RequestDelete` (162) | Empty | Call `archive_vm.update(cx, |vm, cx| vm.request_delete(cx))` |
| `RequestAddFiles` (163) | Empty | Call `archive_vm.update(cx, |vm, cx| vm.request_add_files(cx))` |
| `RequestTestEntries` (164) | Empty | Call `archive_vm.update(cx, |vm, cx| vm.request_test_entries(cx))` |
| `RequestRename` (165) | Empty | Open rename dialog for selected entry |
| `RequestNewFolder` | Missing | Call `NewFolderUseCase` with progress |
| `RequestNewFile` | Missing | Call `NewFileUseCase` with progress |
| `RequestOpenEntry` | Missing | Call `OpenEntryUseCase` |
| `RequestViewEntry` | Missing | Call `OpenEntryUseCase` with preview flag |
| `RequestEditEntry` | Missing | Extract to temp, open in editor, re-add on save |
| `RequestProperties` | Missing | Open `PropertiesEntriesDialog` |
| `RequestChecksum` | Missing | Call `CalculateChecksumUseCase` with progress |

---

## Phase 4: Progress Reporting for All Operations (High Priority)

### File: `src/adapters/view_models/archive_vm.rs`

**Existing progress patterns (lines 468-630):** Delete, Add Files, Test Selected use `ProgressState` global

**Add progress to:**

1. **Extract** (currently in `root.rs:74-102`): Move to `archive_vm.rs` as `request_extract()` method
2. **Create** (currently in `create.rs:359-366`): Move to `archive_vm.rs` as `request_create()` method
3. **Test All** (currently in `root.rs:123-132`): Replace blocking thread with `request_test_all()` using progress

**Pattern:**
```rust
fn request_extract(&mut self, cx: &mut Context<Self>) {
    let (tx, rx) = channel();
    cx.spawn(async move |this, cx| {
        let progress = ProgressState::get_global(cx);
        progress.start("Extracting...".into(), cx);
        let result = ExtractEntriesUseCase::execute(...).await;
        progress.finish(cx);
        this.emit(ArchiveVmEvent::ExtractCompleted(result));
    }).detach();
}
```

---

## Phase 5: Keyboard Shortcuts (Medium Priority)

### File: `src/adapters/views/root.rs` (lines 188-246)

**Fix handlers:**

| Shortcut | Current | Fix |
|----------|---------|-----|
| Ctrl+Shift+N (202-205) | Log only | Emit `ArchiveVmEvent::RequestNewFolder` |
| F4 (222-224) | Log only | Emit `ArchiveVmEvent::RequestEditEntry` |
| Ctrl+V (216-218) | Log only | Emit `ArchiveVmEvent::RequestViewEntry` |
| Alt+Enter (232-234) | Log only | Emit `ArchiveVmEvent::RequestProperties` |

---

## Phase 6: Add Files Dialog Execution (High Priority)

### File: `src/adapters/views/dialogs/add_files.rs` (lines 432-446)

**Current:** OK button emits event but async block does nothing

**Fix:**
1. In OK handler, collect selected files, format, level, encryption
2. Call `AddToArchiveUseCase` with progress channel
3. Emit `AddFilesDialogEvent::AddCompleted(result)` on done
4. Close dialog on success

**In `root.rs`:** Subscribe to `AddFilesDialogEvent::AddRequested` and forward to `archive_vm.request_add_files()`

---

## Phase 7: Create Dialog Fix (High Priority)

### File: `src/adapters/views/dialogs/create.rs` (lines 359-366)

**Current:** Creates archive but doesn't add files; emits `Canceled`

**Fix:**
1. On OK: Create archive using `CreateArchiveUseCase`
2. If files selected, immediately call `AddToArchiveUseCase` 
3. Use progress channel for both operations
4. Emit `CreateDialogEvent::CreateCompleted(result)` on success
5. Close dialog on success

---

## Phase 8: Test All with Progress (High Priority)

### File: `src/adapters/views/root.rs` (lines 123-132)

**Current:** Blocking `std::thread::spawn` without progress

**Fix:** Move to `archive_vm.rs` as `request_test_all()` using same pattern as `request_test_selected()` (lines 583-630)

---

## Phase 9: Linux Tray Progress Connection (Medium Priority)

### Files: 
- `src/adapters/view_models/progress_vm.rs` (lines 169-180)
- `src/adapters/shell/linux.rs` or tray module

**Fix:** When `ProgressState` is created, connect its `tray_sender` to the Linux tray's command channel

---

## Implementation Order

1. **Phase 1** - Context menu (enables basic entry operations)
2. **Phase 3** - RootView handlers (connects menu/context to VM)
3. **Phase 2** - Menu handlers (completes menu bar)
4. **Phase 4** - Progress for Extract/Create/Test All
5. **Phase 6** - Add Files dialog execution
6. **Phase 7** - Create dialog fix
7. **Phase 8** - Test All progress
8. **Phase 5** - Keyboard shortcuts
9. **Phase 9** - Linux tray progress

---

## Testing Checklist

After each phase:
- [ ] `cargo check` passes
- [ ] `cargo build` passes
- [ ] `cargo clippy` passes
- [ ] Manual test: Open archive → right-click entry → verify all context menu items work
- [ ] Manual test: Menu bar → verify all items trigger correct actions
- [ ] Manual test: Keyboard shortcuts → verify all work
- [ ] Manual test: Long operations (extract/create/add/test) → verify progress window appears
- [ ] Manual test: Linux tray → verify progress updates in tray tooltip