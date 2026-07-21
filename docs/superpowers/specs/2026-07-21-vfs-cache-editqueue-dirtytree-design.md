# VFS + Cache + EditQueue + DirtyTree 改造设计

## 背景与目标

当前 `Bit7zRepository` 是扁平的 FFI 封装：`list_page`/`list_directory` 每次调用都走 FFI 枚举全部条目，`apply_changes` 立即写回归档，不支持撤销/重做或延迟保存。

目标是将底层改造为**通用虚拟文件系统 + 缓存 + 内存编辑队列 + 脏树标记**的分层架构，为上层提供延迟保存、撤销/重做能力，并为未来兼容磁盘镜像、远程目录等 backend 留出扩展点。

### 约束

- **内部使用**，不对外暴露挂载接口
- **上层尽量少动**，VFS 主要作为 `Bit7zRepository` 的内部实现细节
- **优先保证现有功能正常**
- 编辑队列**不持久化**（第一阶段）
- 写回策略：第一阶段全量重写，ZIP 等可追加格式未来能力化

## 核心概念

### 读写分离接口

```rust
// domain/vfs/mod.rs

pub trait VfsReader: Send + Sync {
    fn root(&self) -> VfsNodeId;
    fn list(&self, dir: VfsNodeId) -> Result<Vec<VfsNodeId>, VfsError>;
    fn metadata(&self, node: VfsNodeId) -> Result<VfsMetadata, VfsError>;
    fn read(&self, node: VfsNodeId, range: Range<u64>) -> Result<Bytes, VfsError>;
}

pub trait VfsWriter: Send + Sync {
    fn write(&self, dir: VfsNodeId, name: &str, data: &[u8]) -> Result<VfsNodeId, VfsError>;
    fn rename(&self, node: VfsNodeId, new_parent: VfsNodeId, new_name: &str) -> Result<(), VfsError>;
    fn delete(&self, node: VfsNodeId) -> Result<(), VfsError>;
    fn mkdir(&self, dir: VfsNodeId, name: &str) -> Result<VfsNodeId, VfsError>;
}

pub trait Vfs: VfsReader + VfsWriter {}
```

只读 backend（如 RAR）只实现 `VfsReader`；可写 backend 实现 `Vfs`。

### 三棵树模型

| 树 | 命名 | 职责 |
|---|---|---|
| 基准状态 | `BaseTree` | 从 backend 加载的原始状态，代表已持久化到磁盘的内容 |
| 工作状态 | `WorkingTree` | 应用编辑后的逻辑状态，视图直接绑定 |
| 变更标记 | `DirtyTree` | 记录 Working 与 Base 的差异（脏子树标记） |

### 节点模型

- `VfsNodeId`：稳定节点身份，重命名不改 ID
- 打开归档时**完整构建路径树**，元数据（大小、时间、CRC 等）**懒加载**
- `VfsNodeId` ↔ `original_index` 映射由 backend 维护

### 编辑队列

- `EditOperation`：单条变更（Add/Delete/Rename/Mkdir）
- `EditTransaction`：一批可撤销的变更（对应一次用户操作）
- `EditQueue`：撤销/重做栈，内存版，不持久化

## 架构图

```
┌──────────────────────────────────────────────────┐
│        Presentation Layer (Views, 少改动)          │
│   Toolbar: [Save] [Undo] [Redo]  关闭时: 未保存提示  │
│   ArchiveState: list_page → WorkingTree (缓存)       │
└──────────────┬───────────────────────────────────┘
               │ ArchiveRepository trait
┌──────────────▼───────────────────────────────────┐
│              Bit7zRepository (改造)                 │
│                                                    │
│  ┌──────────────────────────────────────┐        │
│  │      ArchiveVfs (infra/vfs)          │        │
│  │  ┌──────────┐ ┌──────────┐ ┌───────┐ │        │
│  │  │BaseTree  │ │Working   │ │Dirty  │ │        │
│  │  │(路径全,+ │ │Tree      │ │Tree   │ │        │
│  │  │ attr懒)  │ │          │ │       │ │        │
│  │  └──────────┘ └──────────┘ └───────┘ │        │
│  │  ┌────────────────────────────┐      │        │
│  │  │       EditQueue            │      │        │
│  │  │  [Transaction1] [Tx2] ...  │      │        │
│  │  └────────────────────────────┘      │        │
│  └──────────────────────────────────────┘        │
│                                                    │
│  Handles: HashMap<u64, FfiHandle> (for preview/extract) │
└──────────────┬───────────────────────────────────┘
               │ bit7z FFI (autocxx)
┌──────────────▼───────────────────────────────────┐
│              bit7z C++ Library                     │
└──────────────────────────────────────────────────┘
```

## ArchiveRepository trait 扩展

```rust
pub trait ArchiveRepository: Send + Sync {
    // 现有方法保持不变
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError>;
    fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError>;
    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError>;
    fn list_directory(&self, archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError>;
    fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError>;
    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path, options: &ExtractOptions) -> Result<(), ArchiveError>;
    fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError>;
    fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError>;
    fn close(&self, archive: &ArchiveHandle);
    fn plan_changes(&self, archive: &ArchiveHandle, change_set: &ChangeSet) -> Result<ExecutionPlan, ArchiveError>;
    fn apply_changes(&self, archive: &ArchiveHandle, plan: &ExecutionPlan, options: &WriteOptions) -> Result<(), ArchiveError>;

    // 新增：延迟保存能力
    fn commit(&self, archive: &ArchiveHandle) -> Result<(), ArchiveError>;
    fn undo(&self, archive: &ArchiveHandle) -> Result<bool, ArchiveError>;
    fn redo(&self, archive: &ArchiveHandle) -> Result<bool, ArchiveError>;
    fn has_unsaved_changes(&self, archive: &ArchiveHandle) -> bool;
}
```

### 语义变化

- `apply_changes` 不再立即写回磁盘，而是：
  1. 将 `ExecutionPlan` 转换为 `EditTransaction`
  2. 推入 `EditQueue`
  3. 更新 `WorkingTree` 和 `DirtyTree`
  4. 返回成功（此时仅在内存中生效）

- `commit() ` 触发真正写回：
  1. 遍历 `DirtyTree` 生成 `ChangeSet`
  2. 调用 bit7z writer/editor 执行写回
  3. 成功后 `BaseTree = WorkingTree`，清空 `DirtyTree`

## 数据流

### 打开归档
1. `open()` → 调用 bit7z FFI 打开 reader
2. 读取全部条目路径，构建 `BaseTree`（只建路径树，元数据懒加载）
3. 克隆为 `WorkingTree`，`DirtyTree` 为空

### 列表
1. `list_page()` → 从 `WorkingTree` 按偏移/分页返回条目
2. 元数据首次访问时从 backend 懒加载并缓存

### 编辑
1. `apply_changes(plan)` → 遍历 plan，生成 `EditOperation` 列表
2. 封装为 `EditTransaction` 入队
3. 应用各操作更新 `WorkingTree`
4. 更新 `DirtyTree` 标记被修改的子树

### 撤销
1. `undo()` → 从 `EditQueue` 弹出最近一个 Transaction
2. 反向撤销各操作，恢复 `WorkingTree`
3. 更新 `DirtyTree`

### 保存
1. `commit()` → 遍历 `DirtyTree` 收集变更新节点
2. 生成 `ChangeSet`，调用 bit7z writer/editor 写回
3. 成功后同步 `BaseTree = WorkingTree`，清空 `DirtyTree`

### 关闭
1. 检查 `has_unsaved_changes()`，如返回 true 则提示用户
2. 调用现有 `close()`

## 新增与修改的文件清单

### 新增

| 文件 | 内容 |
|---|---|
| `crates/domain/src/vfs/mod.rs` | VfsNodeId, VfsNode, VfsMetadata, VfsError |
| `crates/domain/src/vfs/tree.rs` | BaseTree, WorkingTree, DirtyTree |
| `crates/domain/src/vfs/queue.rs` | EditOperation, EditTransaction, EditQueue |
| `crates/domain/src/vfs/write_plan.rs` | WritePlan |
| `crates/infrastructure/vfs/src/lib.rs` | ArchiveVfs 实现 |

### 修改

| 文件 | 改动 |
|---|---|
| `crates/domain/src/lib.rs` | 新增 `pub mod vfs;` |
| `crates/domain/src/repository.rs` | ArchiveRepository trait 加 4 个方法 |
| `crates/infrastructure/persistence/src/lib.rs` | Bit7zRepository 内部使用 ArchiveVfs |
| `crates/presentation/views/src/toolbar.rs` | 新增 Save / Undo / Redo 按钮 |
| `crates/presentation/dialogs/src/progress.rs` | 保存进度对话框 |
| `crates/runtime/gui/src/lib.rs` | 窗口关闭前检查未保存 |
| `crates/presentation/views/src/root.rs` | 处理保存/撤销/重做 action |
| `crates/runtime/cli/src/lib.rs` | CLI 调用 commit 等新方法 |
| 测试文件 | MockArchiveRepository 实现新方法 |

## 第一阶段 MVP 范围

1. `domain::vfs` 核心类型与三棵树
2. `infrastructure::vfs::ArchiveVfs` 实现
3. `Bit7zRepository` 内部集成 VFS
4. `ArchiveRepository` trait 扩展
5. `MockArchiveRepository` 新方法
6. 工具栏 Save / Undo / Redo
7. 关闭窗口未保存提示
8. CLI 提交模式

## 风险点

- `original_index` 映射：edit 操作会改变节点在归档中的位置，`VfsNodeId` → `original_index` 的映射需要在 `commit()` 时重建
- 撤销 `DirtyTree` 的恢复：需要能准确回退脏标记，不能漏标或误标
- 与现有提取/预览的协作：extract 和 preview 仍使用 `original_index`，需要确保 WorkingTree 中的 index 在保存前保持有效
