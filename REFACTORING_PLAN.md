# bit7z_archiver 架构重构方案

## 背景

基于对现有代码的深度分析，本方案提出一系列架构改进，目标是：

1. 修复 C++ 桥接层的并发安全隐患
2. 统一写入操作模型，减少文件重写次数
3. 消除领域层泄漏的实现细节
4. 引入脏标记机制，避免不可信的缓存
5. 优化内存使用和 API 人体工学

---

## 1. C++ 桥接层修复

### 1.1 `static std::string` 数据竞争（Critical Bug）

**问题**：`demo.h:73-85` 中 `bit7z_item_path`、`bit7z_item_name` 等函数使用 `static std::string` 作为返回值缓冲区，多线程调用时存在数据竞争。

**修复**：改为 `thread_local` 或调用者提供缓冲区。

```cpp
// 方案 A：thread_local（最小改动）
inline const char* bit7z_item_path(void* reader_ptr, uint32_t index) {
    thread_local std::string s;  // 每个线程独立
    s = static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].path();
    return s.c_str();
}

// 方案 B：调用者缓冲区（更安全，与 bit7z_item_compression_method 系列一致）
inline int32_t bit7z_item_path_ex(void* reader_ptr, uint32_t index,
                                   char* out_buf, uint32_t buf_size) {
    auto& item = static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index];
    return tstring_to_utf8(item.path(), out_buf, buf_size);
}
```

**建议**：方案 B 更安全（无 TLS 开销，无指针失效风险），但需要修改 Rust 侧调用。方案 A 改动最小，可先快速修复。

### 1.2 并发读取限制

**结论**：同一 `BitArchiveReader` 实例**不支持**并发读取。

原因：
- `static`/`thread_local` 缓冲区仅解决数据竞争，不解决 COM 接口线程安全
- 提取回调（`setProgressCallback` 等）修改 reader 内部状态
- 7-Zip COM 实现不保证同一实例并发安全

**不同实例**可以并发读取（独立的 C++ 对象、独立的 COM 接口）。

**当前影响**：`RwLock<RepositoryInner>` 的读锁允许多个读操作"同时"进行，但实际上因为上述限制，这是假并发。

**建议**：
- **短期**：将 `RwLock` 降级为 `Mutex`（语义更准确，避免误导）
- **中期**：引入 per-operation reader 模式，允许真正的并发读

---

## 2. 统一变更集模型

### 2.1 问题

当前 `add()`、`delete()`、`rename()` 各自独立执行，每次都是 `reader→writer/editor→reader` 往返。

```
之前：delete → reader→editor→reader  +  rename → reader→editor→reader  +  add → reader→writer→reader
      = 3 次文件重写
```

7-Zip 每次 `applyChanges()`/`compressTo()` 都要重写整个归档文件，合并操作意味着只重写一次。

### 2.2 领域层：ChangeSet

```rust
// src/domain/archive.rs

#[derive(Debug, Clone)]
pub enum ArchiveChange {
    Add {
        fs_path: PathBuf,
        archive_path: String,
    },
    Update {
        fs_path: PathBuf,
        archive_path: String,
    },
    Delete {
        index: u32,
    },
    Rename {
        index: u32,
        new_path: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    changes: Vec<ArchiveChange>,
}

impl ChangeSet {
    pub fn new() -> Self { Self { changes: Vec::new() } }

    pub fn add(&mut self, fs_path: PathBuf, archive_path: String) {
        self.changes.push(ArchiveChange::Add { fs_path, archive_path });
    }

    pub fn update(&mut self, fs_path: PathBuf, archive_path: String) {
        self.changes.push(ArchiveChange::Update { fs_path, archive_path });
    }

    pub fn delete(&mut self, index: u32) {
        self.changes.push(ArchiveChange::Delete { index });
    }

    pub fn rename(&mut self, index: u32, new_path: String) {
        self.changes.push(ArchiveChange::Rename { index, new_path });
    }

    pub fn is_empty(&self) -> bool { self.changes.is_empty() }

    pub fn len(&self) -> usize { self.changes.len() }

    pub fn iter(&self) -> impl Iterator<Item = &ArchiveChange> {
        self.changes.iter()
    }
}
```

### 2.3 规划层：ExecutionPlan

```rust
// src/application/plan.rs

#[derive(Debug)]
pub struct ExecutionPlan {
    pub deletes: Vec<u32>,
    pub renames: Vec<(u32, String)>,
    pub adds: Vec<(PathBuf, String)>,
    pub updates: Vec<(PathBuf, String)>,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub change_index: usize,
    pub archive_path: String,
    pub existing: ArchiveEntry,
    pub incoming_size: u64,
    pub incoming_mtime: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Apply,
    Skip,
}

impl ExecutionPlan {
    pub fn has_writes(&self) -> bool {
        !self.deletes.is_empty()
            || !self.renames.is_empty()
            || !self.adds.is_empty()
            || !self.updates.is_empty()
    }

    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    pub fn apply_resolutions(&mut self, resolutions: &[ConflictResolution]) {
        debug_assert_eq!(resolutions.len(), self.conflicts.len());

        for (conflict, resolution) in self.conflicts.drain(..).zip(resolutions.iter()) {
            if *resolution == ConflictResolution::Apply {
                // 冲突条目作为 Update 加入执行列表
                // fs_path 需要从原始 ChangeSet 中查找
                // 这里简化处理，实际实现需保留原始 fs_path
            }
        }
    }
}
```

### 2.4 规划函数

```rust
// src/application/plan.rs

pub fn plan_changes(
    snapshot: &[ArchiveEntry],
    change_set: &ChangeSet,
) -> ExecutionPlan {
    let mut plan = ExecutionPlan {
        deletes: Vec::new(),
        renames: Vec::new(),
        adds: Vec::new(),
        updates: Vec::new(),
        conflicts: Vec::new(),
    };

    let existing_paths: HashMap<&str, &ArchiveEntry> = snapshot.iter()
        .map(|e| (e.path.as_str(), e))
        .collect();

    for (i, change) in change_set.iter().enumerate() {
        match change {
            ArchiveChange::Add { fs_path, archive_path } => {
                if let Some(existing) = existing_paths.get(archive_path.as_str()) {
                    plan.conflicts.push(Conflict {
                        change_index: i,
                        archive_path: archive_path.clone(),
                        existing: (*existing).clone(),
                        incoming_size: fs::metadata(fs_path).map(|m| m.len()).unwrap_or(0),
                        incoming_mtime: file_mtime(fs_path),
                    });
                } else {
                    plan.adds.push((fs_path.clone(), archive_path.clone()));
                }
            }
            ArchiveChange::Update { fs_path, archive_path } => {
                plan.updates.push((fs_path.clone(), archive_path.clone()));
            }
            ArchiveChange::Delete { index } => {
                plan.deletes.push(*index);
            }
            ArchiveChange::Rename { index, new_path } => {
                plan.renames.push((*index, new_path.clone()));
            }
        }
    }

    plan
}
```

### 2.5 适配器层：统一执行

```rust
// src/adapters/repository.rs

impl ArchiveRepository for Bit7zRepository {
    fn plan_changes(&self, archive: &ArchiveHandle, change_set: &ChangeSet)
        -> Result<ExecutionPlan, ArchiveError>
    {
        let guard = self.inner.read().map_err(|_| {
            ArchiveError::Internal("RwLock poisoned".into())
        })?;

        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal("handle not found".into()))?;

        // 快照当前条目
        let snapshot = read_all_entries(raw);

        // 规划
        let plan = plan_changes(&snapshot, change_set);

        Ok(plan)
    }

    fn apply_changes(&self, archive: &ArchiveHandle, plan: &ExecutionPlan)
        -> Result<(), ArchiveError>
    {
        if !plan.has_writes() {
            return Ok(());
        }

        let mut guard = self.inner.write().map_err(|_| {
            ArchiveError::Internal("RwLock poisoned".into())
        })?;

        // 写入开始：标记缓存为脏
        guard.cache_state.insert(archive.id, CacheState::Dirty);

        // 移除旧 reader
        guard.handles.remove(&archive.id);

        let archive_path = /* 从 archive 元数据获取 */;
        let format = detect_writer_format(&archive_path);

        let result = if needs_editor(plan) {
            execute_with_editor(&guard.lib, &archive_path, format, plan, &guard.cancel, &guard.paused)
        } else {
            execute_with_writer(&guard.lib, &archive_path, format, plan, &guard.cancel, &guard.paused)
        };

        // 重新打开 reader（无论成功失败）
        match ArchiveReader::open(&guard.lib, &archive_path, None) {
            Ok(reader) => {
                guard.handles.insert(archive.id, FfiHandle::reader(reader.into_raw() as *mut _));
            }
            Err(_) => {
                guard.handles.remove(&archive.id);
            }
        }

        // 缓存保持 Dirty，下次 list_page 会触发 FFI 扫描

        result
    }
}

fn needs_editor(plan: &ExecutionPlan) -> bool {
    !plan.deletes.is_empty() || !plan.renames.is_empty()
}

fn execute_with_editor(
    lib: &Library,
    path: &Path,
    format: WriterFormat,
    plan: &ExecutionPlan,
    cancel: &Option<Arc<AtomicBool>>,
    paused: &Option<Arc<AtomicBool>>,
) -> Result<(), ArchiveError> {
    let editor = Editor::open(lib, path, format, None)
        .map_err(|e| ArchiveError::Internal(format!("editor open: {}", e)))?;

    // 删除（倒序，避免索引偏移）
    let mut sorted_deletes = plan.deletes.clone();
    sorted_deletes.sort_unstable_by(|a, b| b.cmp(a));
    for &idx in &sorted_deletes {
        editor.delete(idx)
            .map_err(|e| ArchiveError::Internal(format!("delete: {}", e)))?;
    }

    // 重命名
    for &(idx, ref new_path) in &plan.renames {
        editor.rename(idx, new_path)
            .map_err(|e| ArchiveError::Internal(format!("rename: {}", e)))?;
    }

    // 添加
    for (fs_path, arc_path) in &plan.adds {
        editor.add_item(fs_path, arc_path)
            .map_err(|e| ArchiveError::Internal(format!("add: {}", e)))?;
    }

    // 更新
    for (fs_path, arc_path) in &plan.updates {
        editor.update_item(arc_path, fs_path)
            .map_err(|e| ArchiveError::Internal(format!("update: {}", e)))?;
    }

    // 设置进度/暂停/取消回调
    setup_write_callbacks(&editor, cancel, paused)?;

    // 一次性提交
    editor.apply()
        .map_err(|e| ArchiveError::Internal(format!("apply: {}", e)))?;

    Ok(())
}

fn execute_with_writer(
    lib: &Library,
    path: &Path,
    format: WriterFormat,
    plan: &ExecutionPlan,
    cancel: &Option<Arc<AtomicBool>>,
    paused: &Option<Arc<AtomicBool>>,
) -> Result<(), ArchiveError> {
    let writer = Writer::open(lib, path, format, None)
        .map_err(|e| ArchiveError::Internal(format!("writer open: {}", e)))?;

    let has_updates = !plan.updates.is_empty();
    writer.set_update_mode(if has_updates {
        UpdateMode::Update
    } else {
        UpdateMode::Append
    });

    // 添加
    for (fs_path, arc_path) in &plan.adds {
        writer.add_item(fs_path, arc_path)
            .map_err(|e| ArchiveError::Internal(format!("add: {}", e)))?;
    }

    // 更新
    for (fs_path, arc_path) in &plan.updates {
        writer.add_item(fs_path, arc_path)
            .map_err(|e| ArchiveError::Internal(format!("update: {}", e)))?;
    }

    // 设置进度/暂停/取消回调
    setup_write_callbacks(&writer, cancel, paused)?;

    // 执行压缩
    writer.compress_to(path)
        .map_err(|e| ArchiveError::Internal(format!("compress: {}", e)))?;

    Ok(())
}
```

### 2.6 写入回调（含暂停/取消）

```rust
// src/adapters/repository.rs

struct WriteCtx {
    progress: ProgressSender,
    cancel: Option<Arc<AtomicBool>>,
    paused: Option<Arc<AtomicBool>>,
}

extern "C" fn compress_progress_callback(
    processed: u64,
    total: u64,
    ctx: *mut c_void,
) -> i32 {
    let ctx = unsafe { &*(ctx as *const WriteCtx) };

    // 取消
    if let Some(ref cancel) = ctx.cancel {
        if cancel.load(Ordering::Relaxed) {
            return 0;
        }
    }

    // 暂停（与提取共用同一模式）
    if let Some(ref paused) = ctx.paused {
        while paused.load(Ordering::Relaxed) {
            if let Some(ref cancel) = ctx.cancel {
                if cancel.load(Ordering::Relaxed) {
                    return 0;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    let _ = ctx.progress.send(ProgressUpdate {
        bytes_done: processed,
        bytes_total: total,
        ..
    });

    1
}
```

---

## 3. 脏标记缓存

### 3.1 设计原则

- 写入后一律视为脏，不信任任何缓存
- 下次 `list_page` 时按需重新获取
- 类似 UI 的脏树标记思想

### 3.2 实现

```rust
// src/adapters/repository.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Valid,
    Dirty,
}

struct RepositoryInner {
    lib: bit7z::Library,
    handles: HashMap<u64, FfiHandle>,
    overwrite_mode: OverwriteMode,
    cancel: Option<Arc<AtomicBool>>,
    paused: Option<Arc<AtomicBool>>,
    progress_notifier: Option<Box<dyn ProgressNotifier>>,
    cache_state: HashMap<u64, CacheState>,
}

impl Bit7zRepository {
    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize)
        -> Result<Page<ArchiveEntry>, ArchiveError>
    {
        let guard = self.inner.read().map_err(|_| {
            ArchiveError::Internal("RwLock poisoned".into())
        })?;

        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal("handle not found".into()))?;

        // 无论缓存状态如何，都走 FFI 扫描
        // 缓存状态仅用于未来优化（如按需刷新特定条目）
        let entries = read_all_entries_via_ffi(raw);

        Ok(Page::new(entries, offset, Some(entries.len())))
    }
}
```

### 3.3 写入后标记脏

```rust
fn apply_changes(&self, archive: &ArchiveHandle, plan: &ExecutionPlan) -> Result<(), ArchiveError> {
    let mut guard = self.inner.write()?;

    // 写入开始：标记脏
    guard.cache_state.insert(archive.id, CacheState::Dirty);

    // 移除旧 reader
    guard.handles.remove(&archive.id);

    // 执行写入...
    let result = execute_write(...);

    // 重新打开 reader
    match ArchiveReader::open(...) {
        Ok(reader) => {
            guard.handles.insert(archive.id, FfiHandle::reader(reader.into_raw() as *mut _));
        }
        Err(_) => {
            guard.handles.remove(&archive.id);
        }
    }

    // 缓存保持 Dirty，下次 list_page 会触发 FFI 扫描

    result
}
```

### 3.4 缓存失效触发条件

| 事件 | 行为 |
|------|------|
| 首次打开 | `Dirty` → 首次 `list_page` 触发 FFI 扫描 |
| 写入操作（add/update/delete/rename） | 标记 `Dirty` |
| 写入失败 | 缓存保持 `Dirty` |
| 外部修改（mtime 变化，可选） | 标记 `Dirty` |
| `close()` | 移除缓存状态 |

---

## 4. 领域层重构

### 4.1 移除 trait 中的实现细节

**当前问题**：`ArchiveRepository` trait 包含 `set_progress_notifier`、`set_cancel_flag` 等，这些是适配器层关注点。

**修复**：将操作配置内联到方法签名中。

```rust
// src/domain/repository.rs

pub struct WriteOptions {
    pub overwrite_mode: OverwriteMode,
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub notifier: Arc<dyn ProgressNotifier>,
}

pub struct ExtractOptions {
    pub overwrite_mode: OverwriteMode,
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub notifier: Arc<dyn ProgressNotifier>,
}

pub trait ArchiveRepository: Send + Sync {
    // 读操作
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError>;
    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError>;
    fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError>;
    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path, options: &ExtractOptions) -> Result<(), ArchiveError>;

    // 写操作：统一为变更集
    fn plan_changes(&self, archive: &ArchiveHandle, change_set: &ChangeSet) -> Result<ExecutionPlan, ArchiveError>;
    fn apply_changes(&self, archive: &ArchiveHandle, plan: &ExecutionPlan, options: &WriteOptions) -> Result<(), ArchiveError>;

    fn close(&self, archive: &ArchiveHandle);
}
```

### 4.2 移除冗余的 Use Case

| 现有模块 | 变化 |
|----------|------|
| `application/add_to.rs` | 删除，合并到 `ModifyArchiveUseCase` |
| `application/delete.rs` | 删除 |
| `application/rename.rs` | 删除 |
| `application/extract.rs` | 保留，但签名改为接受 `ExtractOptions` |

新增：

```rust
// src/application/modify.rs

pub struct ModifyArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl ModifyArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn plan(&self, archive: &ArchiveHandle, change_set: ChangeSet)
        -> Result<ExecutionPlan, ArchiveError>
    {
        self.repo.plan_changes(archive, &change_set)
    }

    pub fn execute(&self, archive: &ArchiveHandle, plan: &ExecutionPlan, options: &WriteOptions)
        -> Result<(), ArchiveError>
    {
        if plan.has_conflicts() {
            return Err(ArchiveError::Conflict(plan.conflicts.clone()));
        }

        self.repo.apply_changes(archive, plan, options)
    }
}
```

---

## 5. Application 层调用流程

### 5.1 完整流程

```rust
// 在 view model 或 CLI 中

async fn handle_modify_archive(
    use_case: &ModifyArchiveUseCase,
    archive: &ArchiveHandle,
    changes: ChangeSet,
    options: WriteOptions,
    ui: &mut UiContext,
) -> Result<(), ArchiveError> {
    // 1. 规划
    let mut plan = use_case.plan(archive, changes)?;

    // 2. 如果有冲突，显示对话框让用户解决
    if plan.has_conflicts() {
        let resolutions = ui.show_conflict_dialog(&plan.conflicts).await?;
        plan.apply_resolutions(&resolutions);
    }

    // 3. 执行（含进度、暂停、取消）
    use_case.execute(archive, &plan, options)?;

    Ok(())
}
```

### 5.2 冲突对话框

```rust
// src/adapters/views/dialogs/conflict.rs

impl ConflictDialog {
    async fn show(&mut self, conflicts: &[Conflict]) -> Vec<ConflictResolution> {
        let mut resolutions = Vec::with_capacity(conflicts.len());

        for conflict in conflicts {
            let choice = self.prompt_user(
                &conflict.existing,      // 旧文件信息
                conflict.incoming_size,  // 新文件大小
                conflict.incoming_mtime, // 新文件时间
            ).await;

            resolutions.push(match choice {
                UserChoice::Overwrite => ConflictResolution::Apply,
                UserChoice::Skip => ConflictResolution::Skip,
            });
        }

        resolutions
    }
}
```

---

## 6. 性能影响分析

| 方面 | 改进前 | 改进后 |
|------|--------|--------|
| 多次写入 | 每次操作重写整个归档 | 合并为一次重写 |
| 写入后扫描 | 立即全量扫描（阻塞写锁） | 延迟到下次 `list_page` |
| 并发读 | 假并发（static 变量竞争） | 明确单实例或 per-operation reader |
| 内存 | 无缓存 | 脏标记，按需加载 |
| 暂停/取消 | 仅提取支持 | 提取/压缩均支持 |

---

## 7. 迁移路径

### 阶段 1：修复 Critical Bug

- [ ] 修复 `demo.h` 中的 `static std::string` 数据竞争
- [ ] 将 `RwLock` 降级为 `Mutex`（语义修正）

### 阶段 2：引入 ChangeSet

- [ ] 新增 `domain/archive.rs` 中的 `ArchiveChange`、`ChangeSet`
- [ ] 新增 `application/plan.rs` 中的 `ExecutionPlan`、`plan_changes()`
- [ ] 新增 `application/modify.rs` 中的 `ModifyArchiveUseCase`
- [ ] 重构 `adapters/repository.rs`：新增 `plan_changes()`、`apply_changes()`

### 阶段 3：移除冗余代码

- [ ] 删除 `application/add_to.rs`、`delete.rs`、`rename.rs`
- [ ] 删除 `adapters/repository.rs` 中的 `add()`、`delete()`、`rename()`
- [ ] 更新 `application/mod.rs` 导出

### 阶段 4：重构 trait

- [ ] 移除 `set_progress_notifier`、`set_cancel_flag` 等
- [ ] 新增 `ExtractOptions`、`WriteOptions`
- [ ] 更新所有调用点

### 阶段 5：脏标记缓存

- [ ] 新增 `CacheState` 枚举
- [ ] 在 `RepositoryInner` 中添加 `cache_state` 字段
- [ ] 写入后标记 `Dirty`
- [ ] 可选：实现按需刷新特定条目

---

## 8. 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| `static std::string` 修复可能影响性能 | 先用方案 A（thread_local），后续评估方案 B |
| ChangeSet 引入增加复杂度 | 分阶段迁移，保留旧接口直到新接口稳定 |
| 脏标记导致频繁 FFI 扫描 | 当前可接受；后续可优化为按需刷新 |
| Editor/Writer 选择逻辑复杂 | 封装为内部函数，对外透明 |
| 暂停/取消在写入回调中的行为 | 与提取共用同一模式，已验证可行 |

---

## 9. 总结

本方案的核心改进：

1. **修复并发安全**：消除 `static` 变量竞争，明确并发模型
2. **统一写入模型**：ChangeSet + ExecutionPlan，减少文件重写
3. **消除领域泄漏**：移除 trait 中的实现细节，内联到方法签名
4. **脏标记缓存**：写入后不信任缓存，按需重新获取
5. **暂停/取消统一**：提取和压缩共用同一套机制

这些改进相互独立，可以分阶段实施，降低迁移风险。
