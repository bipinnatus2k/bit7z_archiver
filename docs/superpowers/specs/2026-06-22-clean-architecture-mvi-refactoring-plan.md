# Clean Architecture + DDD + MVI 重构计划

**日期：** 2026-06-22
**状态：** 待评审
**范围：** 全项目架构重构，分 3 个 Phase 按层级推进

---

## 一、现状问题

### 1.1 领域层污染

| 问题 | 位置 | 严重性 |
|------|------|--------|
| `gpui::Global` 侵入 domain | `src/domain/preferences.rs:19,34` | Critical |
| `gpui::Global` 侵入 domain | `src/domain/repository.rs:9-10` | Critical |
| FFI 原始指针在 domain 的 ArchiveHandle | `src/domain/archive.rs:132` | Critical |
| `crossbeam::channel::Sender` 在 domain trait | `src/domain/repository.rs:2` | Moderate |

### 1.2 UI 层耦合

| 问题 | 位置 | 严重性 |
|------|------|--------|
| 6 个子组件全部持有 `Entity<ViewModel>` | menu, toolbar, browser, file_list, status_bar, preview_panel | Critical |
| ViewModel 包含业务逻辑（filter, sort） | `archive_vm.rs:196-235` | Moderate |
| ViewModel 直接调 UseCase | `archive_vm.rs` 多处 | Moderate |
| View 直接调 UseCase + repo | `root.rs:97,133,146` | Moderate |
| Application 层包含 UI 事件 | `application/events.rs` | Moderate |
| Dialog 覆盖层嵌入 RootView render | `root.rs:434-449` | Moderate |

### 1.3 当前依赖方向

```
Domain → (gpui::Global, *mut c_void, crossbeam) ❌ 应无外部依赖
Application → OK（仅依赖 domain）
Adapters (Views) → 持 ViewModel Entity → 调 UseCase → OK（Adapter 可依赖应用）
RootView → 内含 Dialog 覆盖层 + 直接调 UseCase → 耦合
```

---

## 二、目标架构总览

```
┌─────────────────────────────────────────────────────────┐
│  Infrastructure / Framework                              │
│  GPUI, autocxx, vcpkg, IPC, CLI, theme                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Adapters Layer                                    │  │
│  │  ┌─────────────────────────────────────────────┐  │  │
│  │  │  Views (MVI components)                     │  │  │
│  │  │  ├── RootView (Reducer) ← 唯一持有 State    │  │  │
│  │  │  ├── RootController (Effect Handler)        │  │  │
│  │  │  ├── FileList, Toolbar, Menu, Browser       │  │  │
│  │  │  ├── StatusBar, PreviewPanel                │  │  │
│  │  │  └── All Dialogs (独立 MVI 周期 + 独立窗口)  │  │  │
│  │  ├─────────────────────────────────────────────┘  │  │
│  │  ├── ViewModels → 改为 State (纯数据)              │  │  │
│  │  ├── Events (从 application 迁移)                 │  │  │
│  │  ├── RepositoryImpl (Bit7zRepository)             │  │  │
│  │  └── bit7z FFI → 内部管理 raw pointer              │  │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Application Layer (Use Cases, DTOs)              │  │
│  │  依赖: Domain のみ                                 │  │
│  │  无 UI 事件, 无 framework 类型                     │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Domain Layer (Entities, VOs, Ports)              │  │
│  │  依赖: 无 (纯 Rust + std)                          │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 依赖方向

```
Domain ← Application ← Adapters ← Infrastructure
(纯)     (仅 domain)   (app+domain)  (GPUI/FFI)
```

---

## 三、DDD 领域模型

### Aggregate Root: `Archive`

```
Archive (由 ArchiveId 标识)
├── Value: ArchiveId(u64)
├── Entity: ArchiveEntry
│   ├── identity: original_index (u32)
│   ├── name, path, size, compressed_size
│   ├── is_directory, is_encrypted, is_symlink
│   ├── timestamps (modified, created, accessed)
│   └── properties (crc, attributes, compression_method, ...)
└── Value: ArchiveProperties
    ├── items_count, folders_count, files_count
    ├── total_size, packed_size
    └── is_encrypted, is_solid, ...
```

### Value Objects

```
ArchiveFormat { SevenZip | Zip | Tar | TarGz | TarBz2 | TarXz | Rar }
EncryptionMethod { Aes256 | ZipCrypto }
Password(SecretString)
EncryptionConfig { password, method, encrypt_filenames }
Page<T> { items, offset, total }
TestResult { total, passed, failed: Vec<TestFailure> }
TestFailure { entry_path, error, index, reason }
TestFailureReason { CrcMismatch | ReadError | UnsupportedOperation }
CreateFileItem { path, is_directory, size }
OverwriteMode
ProgressUpdate { file_current, file_total, ... }
```

### Ports (Domain Interfaces)

```rust
// src/domain/repository.rs
pub trait ArchiveRepository: Send + Sync {
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError>;
    fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError>;
    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError>;
    fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError>;
    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path) -> Result<(), ArchiveError>;
    fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError>;
    fn add(&self, archive: &mut ArchiveHandle, files: &[PathBuf], password: Option<&Password>) -> Result<(), ArchiveError>;
    fn delete(&self, archive: &mut ArchiveHandle, indices: &[u32]) -> Result<(), ArchiveError>;
    fn rename(&self, archive: &mut ArchiveHandle, index: u32, new_name: &str) -> Result<(), ArchiveError>;
    fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError>;
    fn close(&self, archive: ArchiveHandle);
    fn list_directory(&self, archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError>;
}

// src/domain/repository.rs — 新增
pub trait ProgressNotifier: Send {
    fn notify(&self, update: &ProgressUpdate);
}
```

### Error Types

```rust
pub enum ArchiveError {
    NotFound(String), UnsupportedFormat, Corrupt(String),
    WrongPassword, EncryptedArchiveRequiresPassword,
    Io(std::io::Error), Internal(String), Canceled,
    UnsupportedOperation, ReadOnlyArchive,
}
```

---

## 四、MVI 组件模型

### 4.1 主窗口组件树

```
RootView (Reducer)
├── state: ArchiveState (纯数据)
├── controller: RootController (Effect Handler)
├── file_list: Entity<FileList>
├── toolbar: Entity<Toolbar>
├── menu: Entity<Menu>
├── browser: Entity<ArchiveBrowser>
├── preview: Entity<PreviewPanel>
└── status_bar: Entity<StatusBar>

数据流: Intent → RootView → state.update() → sync_children()
Effect:  RootView → controller → UseCase / Dialog → Result → state.update()
```

### 4.2 组件接口契约

```
FileList:
  Input:  entries: Vec<LevelEntry>, selection: HashSet<u32>, sort_col, sort_asc
  Output: FileListIntent { RowClicked, SortByColumn, SelectAll, ClearSelection,
            DeleteSelected, Rename, ShowProperties, Checksum }

Toolbar:
  Input:  is_ready: bool, has_selection: bool
  Output: ToolbarIntent { OpenArchive, CreateArchive, ExtractSelected,
            AddFiles, TestArchive, ShowSettings }

Menu:
  Input:  (无 state 依赖)
  Output: MenuIntent { OpenArchive, ShowSettings }

ArchiveBrowser:
  Input:  subdirs: Vec<String>, current_path: String
  Output: BrowserIntent { NavigateInto, NavigateUp, NavigateRoot }

StatusBar:
  Input:  status_text: String
  Output: (无)

PreviewPanel:
  Input:  preview_data: Option<PreviewData>
  Output: (无)
```

### 4.3 Dialog 独立 MVI 周期

每个 Dialog 是一个独立 GPUI 窗口，拥有完整的 MVI 循环：

```
Dialog 外部接口（统一模式）:
  fn open(input, deps, cx) -> tokio::sync::oneshot::Receiver<Result>

Dialog 内部:
  State { dialog-specific fields }
  Intent { 用户操作 }
  EventEmitter<Intent> → 内部 subscribe → reduce state
  Confirm 时 → 执行 UseCase / 收集数据 → result_tx.send() → close_window
```

#### PasswordDialog

```
Input:  archive_path: String
Output: PasswordResult { Submitted(String) | Canceled }
Channel: tokio::sync::oneshot
MVI:
  State { archive_path, input_text, show_password, error_msg }
  Intent { UpdateInput, ToggleVisibility, Confirm, Cancel }
  副作用: 无（仅收集输入）
```

#### ExtractDialog

```
Input:  entries: Vec<ArchiveEntry>, repo, archive_handle
Output: ExtractResult { Success | Error(String) | Canceled }
Channel: tokio::sync::oneshot
MVI:
  State { entries, destination, overwrite_mode, keep_broken, is_extracting, progress }
  Intent { SelectDestination, SetOverwriteMode, ToggleKeepBroken, Confirm, Cancel }
  副作用: ExtractEntriesUseCase（Confirm 时触发）
```

#### CreateArchiveDialog

```
Input:  repo
Output: CreateResult { Success(PathBuf) | Error(String) | Canceled }
Channel: tokio::sync::oneshot
MVI:
  State { files, format, compression_level, encryption_config, destination, is_creating, progress }
  Intent { AddFiles, RemoveFile, SetFormat, SetCompression, SetEncryption, SetDestination, Confirm, Cancel }
  副作用: CreateArchiveUseCase（Confirm 时触发）
```

#### AddFilesDialog

```
Input:  repo, archive_handle
Output: AddResult { Success(u64) | Error(String) | Canceled }
Channel: tokio::sync::oneshot
MVI:
  State { files, is_adding, progress }
  Intent { AddFiles, RemoveFile, Confirm, Cancel }
  副作用: AddToArchiveUseCase（Confirm 时触发）
```

#### SettingsDialog

```
Input:  prefs_repo
Output: SettingsResult { Saved | Canceled }
Channel: tokio::sync::oneshot
MVI:
  State { prefs, modified }
  Intent { UpdateTheme, UpdateLanguage, ..., Save, Cancel }
  副作用: prefs_repo.save()（Save 时触发）
```

#### ProgressDialog（CLI 任务用）

```
Input:  title: String, operation params
Output: 任务结果报文
Channel: tokio::sync::oneshot
MVI:
  State { title, message, current, total, is_complete, error }
  Intent { Cancel }
  副作用: 对应 UseCase
```

### 4.4 跨窗口通信

```
主窗口                   Dialog 窗口
├─ ArchiveState          ├─ Self-contained State
├─ RootController        ├─ Own repo reference
├─ 打开 Dialog           ├─ 独立 MVI 循环
│  let rx = SomeDlg::    │  接收 Intent → reduce
│    open(deps, cx)      │  Confirm → 执行 UseCase
│  spawn async {         │  → result_tx.send(result)
│    rx.await             │  → close_window()
│    → reduce to state   │
│  }                     │
└──────────────────────  └──────────────────────
         oneshot channel
```

原则：
- Dialog 不引用主窗口的任何 Entity
- Dialog 不引用主窗口的 State
- Dialog 不持有主窗口的 ViewModel
- 所有结果通过 `tokio::sync::oneshot` 回传
- 主窗口通过 `cx.entity().downgrade()` 在 async task 中更新自己的状态

### 4.5 RootView 三明治结构

```
src/adapters/views/root/
├── mod.rs         — RootView struct + Render (~80行)
├── state.rs       — ArchiveState (纯数据 + 纯函数) (~200行)
├── controller.rs  — RootController (Effect Handler) (~150行)
├── intents.rs     — Intent 类型 + Handler 分发 (~150行)
└── children.rs    — 子组件创建 + sync_children (~80行)

RootView:    持有 state + controller + children
             订阅子组件 Intent → 分发到 state 或 controller
             不渲染任何 Dialog 覆盖层
             不直接调 UseCase / repo

RootController:    持有 repo + 可创建 UseCase + Dialog
                   方法返回 oneshot::Receiver<Result>
                   纯 struct，非 Entity，无 Context 引用
```

### 4.6 ArchiveState

```rust
pub struct ArchiveState {
    pub archive: Option<ArchiveHandle>,
    pub properties: Option<ArchiveProperties>,
    pub archive_password: Option<Password>,
    pub directory_cache: HashMap<String, Vec<ArchiveEntry>>,
    pub current_path: String,
    pub path_history: Vec<String>,
    pub level_entries: Vec<LevelEntry>,
    pub selection: HashSet<u32>,
    pub filter_text: String,
    pub sort_column: u32,
    pub sort_ascending: bool,
    pub status: ViewStatus,
    pub status_message: Option<String>,
}

impl ArchiveState {
    // 纯函数操作，无副作用，无 GPUI 类型
    pub fn update_selection(&mut self, row: usize, mods: &Modifiers);
    pub fn sort_by_column(&mut self, col: u32);
    pub fn filter_entries(&mut self, text: &str);
    pub fn navigate_into(&mut self, dir: &str);
    pub fn navigate_up(&mut self);
    pub fn selected_entries(&self) -> Vec<ArchiveEntry>;
    pub fn displayed_entries(&self) -> &[LevelEntry];
    pub fn status_text(&self) -> String;
}
```

---

## 五、CLI 集成方案

### 当前行为

```
bit7z extract archive.7z /out
  → 尝试 IPC 到已有 GUI
  → 失败 → 打开全量主窗口 → 自动执行
```

### MVI 行为

```
bit7z extract archive.7z /out
  → 尝试 IPC 到已有 GUI
  → 失败 → 打开 ProgressDialog 独立窗口
  → 窗口内执行提取
  → 完成 → 显示结果 → 用户关闭 → 进程退出

bit7z (无参数)
  → 全量主窗口（不变）
```

### CLI 命令 → Dialog 映射

| CLI 命令 | 窗口类型 |
|----------|---------|
| `open` | 全量主窗口（自动打开档案） |
| `extract` | ProgressDialog |
| `test` | ProgressDialog |
| `add` | ProgressDialog |
| `delete` | ProgressDialog |
| `rename` | ProgressDialog |
| `checksum` | ProgressDialog |
| `compress` | ProgressDialog |
| `list` | 终端输出（无窗口） |
| `preview` | PreviewDialog |
| `new-folder` | ProgressDialog |
| 无参数 | 全量主窗口 |

---

## 六、执行计划（3 Phase）

### Phase 1: 领域层净化

**目标：** Domain 零外部框架依赖，ArchiveHandle 去除 FFI 指针。

| 步骤 | 文件 | 变更 |
|------|------|------|
| 1.1 | `src/domain/preferences.rs:19` | 删除 `impl Global for Preferences {}` |
| 1.1 | `src/domain/preferences.rs:34` | 删除 `impl Global for PreferencesRepoGlobal {}` |
| 1.1 | `src/domain/repository.rs:9-10` | 删除 `impl Global for RepoGlobal {}` |
| 1.1 | `src/gui.rs` | 新建包装类型 `PreferencesGlobal(Preferences)` 实现 Global |
| 1.1 | `src/gui.rs` | 新建 `RepoGlobal(Arc<dyn ArchiveRepository>)` 实现 Global |
| 1.1 | 所有 `cx.global::<Preferences>()` | 改为 `cx.global::<PreferencesGlobal>().0.clone()` |
| 1.2 | `src/domain/archive.rs` | `ArchiveHandle` 删除 `raw: *mut c_void`，添加 `id: u64`；删除 `unsafe Send/Sync` |
| 1.2 | `src/adapters/repository.rs` | `Bit7zRepository` 新增 `HashMap<u64, RawCppHandle>` |
| 1.2 | `src/adapters/bit7z/mod.rs` | 新增 `RawCppHandle` 类型包装 FFI 指针 |
| 1.3 | `src/domain/repository.rs` | 新增 `ProgressNotifier` trait；`ArchiveRepository` 方法参数改为 `&dyn ProgressNotifier` |
| 1.3 | `src/application/progress.rs` | 适配 `ProgressNotifier` |
| 1.3 | `src/application/*.rs` | UseCase 方法签名适配新参数 |
| 1.4 | `src/adapters/repository.rs` | Bit7zRepository 实现 `ProgressNotifier`（delegate to crossbeam channel） |

**验证：** `cargo check && cargo test && cargo clippy`

---

### Phase 2: 应用层改造

**目标：** Application 仅依赖 domain，无 UI 类型。

| 步骤 | 文件 | 变更 |
|------|------|------|
| 2.1 | `src/application/events.rs` | 整个文件迁移到 `src/adapters/events.rs` |
| 2.1 | 所有引用 `crate::application::events` | 改为 `crate::adapters::events` |
| 2.2 | `src/application/progress.rs` | 改用 domain 的 `ProgressNotifier` trait |
| 2.3 | `src/application/*.rs` | 确认 UseCase 不再接收/返回任何 adapter/UI 类型 |

**验证：** `cargo check && cargo test && cargo clippy`

---

### Phase 3: 适配器层 + MVI 改造

**目标：** MVI 架构，状态提升，组件化，Dialog 独立窗口。

#### P3.1 — 提取 ArchiveState

| 步骤 | 文件 | 变更 |
|------|------|------|
| 3.1.1 | **新建** `src/adapters/view_models/archive_state.rs` | 从 `archive_vm.rs` 提取状态字段 + 纯函数 |
| 3.1.2 | `src/adapters/view_models/archive_vm.rs` | 删除状态字段、纯函数，只保留异步编排 |
| 3.1.3 | `src/adapters/events.rs` | 确认事件类型 |

#### P3.2 — 子组件 MVI 改造

| 步骤 | 文件 | 变更 |
|------|------|------|
| 3.2.1 | `src/adapters/views/menu.rs` | 删除 `archive_vm`；实现 `EventEmitter<MenuIntent>`；`set_state(无参数)` |
| 3.2.2 | `src/adapters/views/toolbar.rs` | 删除 `archive_vm`；实现 `EventEmitter<ToolbarIntent>`；`set_state(is_ready, has_selection)` |
| 3.2.3 | `src/adapters/views/status_bar.rs` | 删除 `archive_vm`；`set_state(status_text)` |
| 3.2.4 | `src/adapters/views/archive_browser.rs` | 删除 `archive_vm`；实现 `EventEmitter<BrowserIntent>`；`set_state(subdirs, path)` |
| 3.2.5 | `src/adapters/views/archive_file_list.rs` | 删除 `archive_vm`；实现 `EventEmitter<FileListIntent>`；`set_state(entries, selection, sort)` |
| 3.2.6 | `src/adapters/views/preview_panel.rs` | 删除 `preview_vm`；`set_state(Option<PreviewData>)` |

#### P3.3 — Dialog 独立窗口改造

| 步骤 | 文件 | 变更 |
|------|------|------|
| 3.3.1 | **新建** `src/adapters/views/dialogs/password/mod.rs` | PasswordDialog 独立窗口 + MVI |
| 3.3.1 | **新建** `src/adapters/views/dialogs/password/state.rs` | PasswordState + PasswordIntent |
| 3.3.2 | **新建** `src/adapters/views/dialogs/extract/mod.rs` | ExtractDialog 独立窗口 + MVI |
| 3.3.2 | **新建** `src/adapters/views/dialogs/extract/state.rs` | ExtractState + ExtractIntent |
| 3.3.3 | `src/adapters/views/dialogs/create.rs` | 重构为独立 MVI 窗口（同当前） |
| 3.3.4 | `src/adapters/views/dialogs/add_files.rs` | 重构为独立 MVI 窗口（同当前） |
| 3.3.5 | `src/adapters/views/dialogs/settings.rs` | 重构为独立 MVI 窗口（同当前） |
| 3.3.6 | **新建** `src/adapters/views/dialogs/progress/` | ProgressDialog（CLI 任务用） |
| 3.3.7 | `src/adapters/views/dialogs/mod.rs` | 更新模块声明 |
| 3.3.8 | 清理 5 个未使用 Dialog | 保留定义，确认其 MVI 接口 |
| 3.3.9 | 删除 Extract/Password Dialog 的 overlay 渲染 | 从 root.rs render 中移除 |

#### P3.4 — RootView + RootController

| 步骤 | 文件 | 变更 |
|------|------|------|
| 3.4.1 | **新建** `src/adapters/views/root/state.rs` | 从 `archive_vm.rs` 迁移 ArchiveState 定义 |
| 3.4.2 | **新建** `src/adapters/views/root/controller.rs` | RootController（Effect Handler） |
| 3.4.3 | **新建** `src/adapters/views/root/intents.rs` | 各组件 Intent 定义 + Handler 分发 |
| 3.4.4 | **新建** `src/adapters/views/root/children.rs` | 子组件创建 + sync_children |
| 3.4.5 | `src/adapters/views/root.rs` | 改为 `root/mod.rs`，精简为 Render + 订阅 |
| 3.4.6 | `src/adapters/views/mod.rs` | 更新模块声明 |

#### P3.5 — CLI ↔ Dialog 集成

| 步骤 | 文件 | 变更 |
|------|------|------|
| 3.5.1 | `src/main.rs` | CLI 命令映射到对应 Dialog 而非全量主窗口 |
| 3.5.2 | `src/gui.rs` | 支持传入命令行参数打开特定 Dialog |
| 3.5.3 | `src/cli.rs` | 保留终端输出命令（list等），GUI 命令改为触发 Dialog |

**验证：** `cargo build && cargo test && cargo clippy`

---

## 七、文件变更清单汇总

### Phase 1: 领域层净化

```
修改: src/domain/preferences.rs    — 删除 gpui::Global
修改: src/domain/repository.rs    — 删除 gpui::Global, 添加 ProgressNotifier
修改: src/domain/archive.rs       — ArchiveHandle 删除 raw, 添加 id
修改: src/adapters/repository.rs  — Bit7zRepository 新增 HashMap<u64, RawCppHandle>
修改: src/adapters/bit7z/mod.rs   — 新增 RawCppHandle 包装类型
修改: src/application/*.rs        — UseCase 方法签名适配
修改: src/gui.rs                  — 新增 Global 包装类型
修改: 所有引用 cx.global::<Preferences>() 的地方
新增: 0
删除: 0
```

### Phase 2: 应用层改造

```
移动: src/application/events.rs → src/adapters/events.rs
修改: src/application/progress.rs  — 使用 ProgressNotifier trait
修改: 所有引用 application::events 的文件
新增: 0
删除: 0 (events.rs 移动后删除原文件)
```

### Phase 3: 适配器层 + MVI

```
新增: src/adapters/view_models/archive_state.rs
新增: src/adapters/views/root/mod.rs
新增: src/adapters/views/root/state.rs
新增: src/adapters/views/root/controller.rs
新增: src/adapters/views/root/intents.rs
新增: src/adapters/views/root/children.rs
新增: src/adapters/views/dialogs/password/mod.rs
新增: src/adapters/views/dialogs/password/state.rs
新增: src/adapters/views/dialogs/extract/mod.rs
新增: src/adapters/views/dialogs/extract/state.rs
新增: src/adapters/views/dialogs/progress/mod.rs
新增: src/adapters/views/dialogs/progress/state.rs

修改: src/adapters/views/menu.rs
修改: src/adapters/views/toolbar.rs
修改: src/adapters/views/status_bar.rs
修改: src/adapters/views/archive_browser.rs
修改: src/adapters/views/archive_file_list.rs
修改: src/adapters/views/preview_panel.rs
修改: src/adapters/views/dialogs/create.rs
修改: src/adapters/views/dialogs/add_files.rs
修改: src/adapters/views/dialogs/settings.rs
修改: src/adapters/views/dialogs/mod.rs
修改: src/adapters/views/mod.rs
修改: src/adapters/view_models/archive_vm.rs
修改: src/adapters/events.rs (若有)
修改: src/main.rs
修改: src/gui.rs
修改: src/cli.rs

删除: src/adapters/views/root.rs（改为 root/mod.rs）
删除: src/application/events.rs (Phase 2 已移动)

未变更: src/domain/ (Phase 1 已完成)
未变更: src/application/ (Phase 2 已完成)
未变更: src/adapters/bit7z/ (不变或仅 RawCppHandle)
未变更: src/adapters/shell/, tray/, platform.rs, preferences_json.rs
未变更: src/worker.rs, ipc.rs, ipc_connect.rs, instance.rs
未变更: src/error.rs, theme.rs, ffi.rs, lib.rs
```

---

## 八、验证策略

### 每阶段验证

```shell
cargo check    # 类型检查
cargo test     # 单元测试 + 集成测试
cargo clippy   # Lint（无新增 warning）
```

### Phase 1 特别验证

```rust
// 1. domain 不依赖任何外部 crate（除 std）
// 2. ArchiveHandle 不包含 FFI 类型
// 3. ArchiveRepository trait 不包含 gpui::Global 或 crossbeam::Sender
// 4. Bit7zRepository 内部正确管理 raw pointer 生命周期
```

### Phase 2 特别验证

```rust
// 1. application/ 不 import 任何 adapter/GPUI 类型
// 2. application/events.rs 已删除
// 3. 所有 UseCase 签名使用 ProgressNotifier trait
```

### Phase 3 特别验证

```rust
// 1. 所有子组件不引用 ArchiveViewModel / PreviewViewModel
// 2. 所有 Dialog 通过 open() 静态方法 + oneshot receiver 交互
// 3. RootView 不渲染 Dialog overlay
// 4. RootView 不直接调 UseCase（委托 controller）
// 5. CLI 命令映射到独立 Dialog 而非全量主窗口
```

---

## 九、架构对比总结

| 维度 | 当前 | 重构后 |
|------|------|--------|
| **Domain 依赖** | GPUI, FFI, crossbeam | 纯 Rust std |
| **ArchiveHandle** | `raw: *mut c_void` | `id: u64`，指针在 adapter |
| **通知机制** | `crossbeam::Sender` 硬编码 | `ProgressNotifier` trait |
| **UI 事件** | `application/events.rs` | `adapters/events.rs` |
| **UI 状态** | ViewModel (GPUI Entity) | ArchiveState (纯数据) |
| **数据流** | 双向读写 VM | 单向 State → View, Intent → Up |
| **组件耦合** | 全部持有 `Entity<ViewModel>` | 零外部引用 |
| **Dialog** | Overlay 嵌入 RootView | 独立窗口 + 自洽 MVI |
| **跨窗口通信** | 无（直接调 VM） | `tokio::sync::oneshot` |
| **CLI 任务** | 打开全量主窗口 | 打开独立任务窗口 |
| **RootView 职责** | VM 引用 + dialog 渲染 + UseCase 调用 | State 持有 + Intent 分发 + Controller 委托 |
