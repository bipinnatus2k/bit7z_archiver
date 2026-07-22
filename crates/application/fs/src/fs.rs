pub mod fs_watcher;

pub use fs_watcher::requires_poll_watcher;

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::time::Instant;

use anyhow::{Context as _, Result, anyhow};
use futures::stream::iter;
use gpui::App;
use gpui::BackgroundExecutor;
use gpui::Global;
use gpui::ReadGlobal as _;
use gpui::SharedString;
#[cfg(unix)]
use std::ffi::CString;

#[cfg(unix)]
use std::os::fd::{AsFd, AsRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
use std::mem::MaybeUninit;

use futures::{AsyncRead, Stream, StreamExt, future::BoxFuture};
use is_executable::IsExecutable;
use serde::{Deserialize, Serialize};
use smol::io::AsyncWriteExt;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            other => result.push(other),
        }
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SanitizedPath(PathBuf);

impl SanitizedPath {
    pub fn new(path: &Path) -> Self {
        Self(normalize_path(path))
    }
    pub fn unchecked_new(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
    pub fn new_arc(path: &Path) -> Arc<Self> {
        Arc::new(Self::new(path))
    }
    pub fn from_arc(arc: Arc<Self>) -> Self {
        (*arc).clone()
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
    pub fn starts_with(&self, base: &SanitizedPath) -> bool {
        self.0.starts_with(&base.0)
    }
}

impl AsRef<Path> for SanitizedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl std::borrow::Borrow<Path> for SanitizedPath {
    fn borrow(&self) -> &Path {
        &self.0
    }
}

pub trait Watcher: Send + Sync {
    fn add(&self, path: &Path) -> Result<()>;
    fn remove(&self, path: &Path) -> Result<()>;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum PathEventKind {
    Removed,
    Created,
    Changed,
    Rescan,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PathEvent {
    pub path: PathBuf,
    pub kind: Option<PathEventKind>,
}

impl From<PathEvent> for PathBuf {
    fn from(event: PathEvent) -> Self {
        event.path
    }
}

#[async_trait::async_trait]
pub trait Fs: Send + Sync {
    async fn create_dir(&self, path: &Path) -> Result<()>;
    async fn create_symlink(&self, path: &Path, target: PathBuf) -> Result<()>;
    async fn create_file(&self, path: &Path, options: CreateOptions) -> Result<()>;
    async fn create_file_with(
        &self,
        path: &Path,
        content: Pin<&mut (dyn AsyncRead + Send)>,
    ) -> Result<()>;
    async fn copy_file(&self, source: &Path, target: &Path, options: CopyOptions) -> Result<()>;
    async fn rename(&self, source: &Path, target: &Path, options: RenameOptions) -> Result<()>;

    async fn remove_dir(&self, path: &Path, options: RemoveOptions) -> Result<()>;
    async fn remove_file(&self, path: &Path, options: RemoveOptions) -> Result<()>;

    async fn open_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>>;
    async fn open_sync(&self, path: &Path) -> Result<Box<dyn io::Read + Send + Sync>>;
    async fn load(&self, path: &Path) -> Result<String> {
        Ok(String::from_utf8(self.load_bytes(path).await?)?)
    }
    async fn load_bytes(&self, path: &Path) -> Result<Vec<u8>>;
    async fn atomic_write(&self, path: PathBuf, text: String) -> Result<()>;
    async fn save(&self, path: &Path, text: &str) -> Result<()>;
    async fn write(&self, path: &Path, content: &[u8]) -> Result<()>;
    async fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
    async fn is_file(&self, path: &Path) -> bool;
    async fn is_dir(&self, path: &Path) -> bool;
    async fn metadata(&self, path: &Path) -> Result<Option<Metadata>>;
    async fn read_link(&self, path: &Path) -> Result<PathBuf>;
    async fn read_dir(
        &self,
        path: &Path,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PathBuf>>>>>;

    async fn watch(
        &self,
        path: &Path,
        latency: Duration,
    ) -> (
        Pin<Box<dyn Send + Stream<Item = Vec<PathEvent>>>>,
        Arc<dyn Watcher>,
    );

    async fn is_case_sensitive(&self) -> bool;
}

struct GlobalFs(Arc<dyn Fs>);

impl Global for GlobalFs {}

impl dyn Fs {
    /// Returns the global [`Fs`].
    pub fn global(cx: &App) -> Arc<Self> {
        GlobalFs::global(cx).0.clone()
    }

    /// Sets the global [`Fs`].
    pub fn set_global(fs: Arc<Self>, cx: &mut App) {
        cx.set_global(GlobalFs(fs));
    }
}

#[derive(Copy, Clone, Default)]
pub struct CreateOptions {
    pub overwrite: bool,
    pub ignore_if_exists: bool,
}

#[derive(Copy, Clone, Default)]
pub struct CopyOptions {
    pub overwrite: bool,
    pub ignore_if_exists: bool,
}

#[derive(Copy, Clone, Default)]
pub struct RenameOptions {
    pub overwrite: bool,
    pub ignore_if_exists: bool,
    /// Whether to create parent directories if they do not exist.
    pub create_parents: bool,
}

#[derive(Copy, Clone, Default)]
pub struct RemoveOptions {
    pub recursive: bool,
    pub ignore_if_not_exists: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct Metadata {
    pub inode: u64,
    pub mtime: MTime,
    pub is_symlink: bool,
    pub is_dir: bool,
    pub len: u64,
    pub is_fifo: bool,
    pub is_executable: bool,
    pub is_writable: bool,
}

/// Filesystem modification time. The purpose of this newtype is to discourage use of operations
/// that do not make sense for mtimes. In particular, it is not always valid to compare mtimes using
/// `<` or `>`, as there are many things that can cause the mtime of a file to be earlier than it
/// was. See ["mtime comparison considered harmful" - apenwarr](https://apenwarr.ca/log/20181113).
///
/// Do not derive Ord, PartialOrd, or arithmetic operation traits.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct MTime(SystemTime);

pub type JobId = usize;

#[derive(Clone, Debug)]
pub struct JobInfo {
    pub start: Instant,
    pub message: SharedString,
    pub id: JobId,
}

#[derive(Debug, Clone)]
pub enum JobEvent {
    Started { info: JobInfo },
    Completed { id: JobId },
}

pub type JobEventSender = futures::channel::mpsc::UnboundedSender<JobEvent>;
pub type JobEventReceiver = futures::channel::mpsc::UnboundedReceiver<JobEvent>;

struct JobTracker {
    id: JobId,
    subscribers: Arc<Mutex<Vec<JobEventSender>>>,
}

impl JobTracker {
    fn new(info: JobInfo, subscribers: Arc<Mutex<Vec<JobEventSender>>>) -> Self {
        let id = info.id;
        {
            let mut subs = subscribers.lock();
            subs.retain(|sender| {
                sender
                    .unbounded_send(JobEvent::Started { info: info.clone() })
                    .is_ok()
            });
        }
        Self { id, subscribers }
    }
}

impl Drop for JobTracker {
    fn drop(&mut self) {
        let mut subs = self.subscribers.lock();
        subs.retain(|sender| {
            sender
                .unbounded_send(JobEvent::Completed { id: self.id })
                .is_ok()
        });
    }
}

impl MTime {
    /// Conversion intended for persistence and testing.
    pub fn from_seconds_and_nanos(secs: u64, nanos: u32) -> Self {
        MTime(UNIX_EPOCH + Duration::new(secs, nanos))
    }

    /// Conversion intended for persistence.
    pub fn to_seconds_and_nanos_for_persistence(self) -> Option<(u64, u32)> {
        self.0
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| (duration.as_secs(), duration.subsec_nanos()))
    }

    /// Returns the value wrapped by this `MTime`, for presentation to the user. The name including
    /// "_for_user" is to discourage misuse - this method should not be used when making decisions
    /// about file dirtiness.
    pub fn timestamp_for_user(self) -> SystemTime {
        self.0
    }

    /// Temporary method to split out the behavior changes from introduction of this newtype.
    pub fn bad_is_greater_than(self, other: MTime) -> bool {
        self.0 > other.0
    }
}

pub struct RealFs {
    bundled_git_binary_path: Option<PathBuf>,
    executor: BackgroundExecutor,
    next_job_id: Arc<AtomicUsize>,
    job_event_subscribers: Arc<Mutex<Vec<JobEventSender>>>,
    is_case_sensitive: AtomicU8,
}

pub trait FileHandle: Send + Sync + std::fmt::Debug {
    fn current_path(&self, fs: &Arc<dyn Fs>) -> Result<PathBuf>;
}

impl FileHandle for std::fs::File {
    #[cfg(target_os = "macos")]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        use std::{
            ffi::{CStr, OsStr},
            os::unix::ffi::OsStrExt,
        };

        let fd = self.as_fd();
        let mut path_buf = MaybeUninit::<[u8; libc::PATH_MAX as usize]>::uninit();

        let result = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETPATH, path_buf.as_mut_ptr()) };
        anyhow::ensure!(result != -1, "fcntl returned -1");

        // SAFETY: `fcntl` will initialize the path buffer.
        let c_str = unsafe { CStr::from_ptr(path_buf.as_ptr().cast()) };
        anyhow::ensure!(!c_str.is_empty(), "Could find a path for the file handle");
        let path = PathBuf::from(OsStr::from_bytes(c_str.to_bytes()));
        Ok(path)
    }

    #[cfg(target_os = "linux")]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        let fd = self.as_fd();
        let fd_path = format!("/proc/self/fd/{}", fd.as_raw_fd());
        let new_path = std::fs::read_link(fd_path)?;
        if new_path
            .file_name()
            .is_some_and(|f| f.to_string_lossy().ends_with(" (deleted)"))
        {
            anyhow::bail!("file was deleted")
        };

        Ok(new_path)
    }

    #[cfg(target_os = "freebsd")]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        use std::{
            ffi::{CStr, OsStr},
            os::unix::ffi::OsStrExt,
        };

        let fd = self.as_fd();
        let mut kif = MaybeUninit::<libc::kinfo_file>::uninit();
        kif.kf_structsize = libc::KINFO_FILE_SIZE;

        let result = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_KINFO, kif.as_mut_ptr()) };
        anyhow::ensure!(result != -1, "fcntl returned -1");

        // SAFETY: `fcntl` will initialize the kif.
        let c_str = unsafe { CStr::from_ptr(kif.assume_init().kf_path.as_ptr()) };
        anyhow::ensure!(!c_str.is_empty(), "Could find a path for the file handle");
        let path = PathBuf::from(OsStr::from_bytes(c_str.to_bytes()));
        Ok(path)
    }

    #[cfg(target_os = "windows")]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::os::windows::io::AsRawHandle;

        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Storage::FileSystem::{
            FILE_NAME_NORMALIZED, GetFinalPathNameByHandleW,
        };

        let handle = HANDLE(self.as_raw_handle() as _);

        // Query required buffer size (in wide chars)
        let required_len =
            unsafe { GetFinalPathNameByHandleW(handle, &mut [], FILE_NAME_NORMALIZED) };
        anyhow::ensure!(
            required_len != 0,
            "GetFinalPathNameByHandleW returned 0 length"
        );

        // Allocate buffer and retrieve the path
        let mut buf: Vec<u16> = vec![0u16; required_len as usize + 1];
        let written = unsafe { GetFinalPathNameByHandleW(handle, &mut buf, FILE_NAME_NORMALIZED) };
        anyhow::ensure!(
            written != 0,
            "GetFinalPathNameByHandleW failed to write path"
        );

        let os_str: OsString = OsString::from_wide(&buf[..written as usize]);
        anyhow::ensure!(!os_str.is_empty(), "Could find a path for the file handle");
        Ok(PathBuf::from(os_str))
    }
}

pub struct RealWatcher {}

impl RealFs {
    pub fn new(git_binary_path: Option<PathBuf>, executor: BackgroundExecutor) -> Self {
        Self {
            bundled_git_binary_path: git_binary_path,
            executor,
            next_job_id: Arc::new(AtomicUsize::new(0)),
            job_event_subscribers: Arc::new(Mutex::new(Vec::new())),
            is_case_sensitive: Default::default(),
        }
    }

    #[cfg(target_os = "windows")]
    fn canonicalize(path: &Path) -> Result<PathBuf> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::Storage::FileSystem::GetVolumePathNameW;
        use windows::core::HSTRING;

        // std::fs::canonicalize resolves mapped network paths to UNC paths, which can
        // confuse some software. To mitigate this, we canonicalize the input, then rebase
        // the result onto the input's original volume root if both paths are on the same
        // volume. This keeps the same drive letter or mount point the caller used.

        let abs_path = if path.is_relative() {
            std::env::current_dir()?.join(path)
        } else {
            path.to_path_buf()
        };

        let path_hstring = HSTRING::from(abs_path.as_os_str());
        let mut vol_buf = vec![0u16; abs_path.as_os_str().len() + 2];
        unsafe { GetVolumePathNameW(&path_hstring, &mut vol_buf)? };
        let volume_root = {
            let len = vol_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(vol_buf.len());
            PathBuf::from(OsString::from_wide(&vol_buf[..len]))
        };

        let resolved_path = dunce::canonicalize(&abs_path)?;
        let resolved_root = dunce::canonicalize(&volume_root)?;

        if let Ok(relative) = resolved_path.strip_prefix(&resolved_root) {
            let mut result = volume_root;
            result.push(relative);
            Ok(result)
        } else {
            Ok(resolved_path)
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_without_replace(source: &Path, target: &Path) -> io::Result<()> {
    let source = path_to_c_string(source)?;
    let target = path_to_c_string(target)?;

    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renamex_np(source.as_ptr(), target.as_ptr(), libc::RENAME_EXCL) };

    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "windows")]
fn rename_without_replace(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW};
    use windows::core::PCWSTR;

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();

    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVE_FILE_FLAGS::default(),
        )
    }
    .map_err(|_| io::Error::last_os_error())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn path_to_c_string(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path contains interior NUL: {}", path.display()),
        )
    })
}

#[async_trait::async_trait]
impl Fs for RealFs {
    async fn create_dir(&self, path: &Path) -> Result<()> {
        Ok(smol::fs::create_dir_all(path).await?)
    }

    async fn create_symlink(&self, path: &Path, target: PathBuf) -> Result<()> {
        #[cfg(unix)]
        smol::fs::unix::symlink(target, path).await?;

        #[cfg(windows)]
        if smol::fs::metadata(&target).await?.is_dir() {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .args([path, target.as_path()])
                .status()?;

            if !status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to create junction from {:?} to {:?}",
                    path,
                    target
                ));
            }
        } else {
            smol::fs::windows::symlink_file(target, path).await?
        }

        Ok(())
    }

    async fn create_file(&self, path: &Path, options: CreateOptions) -> Result<()> {
        let mut open_options = smol::fs::OpenOptions::new();
        open_options.write(true).create(true);
        if options.overwrite {
            open_options.truncate(true);
        } else if !options.ignore_if_exists {
            open_options.create_new(true);
        }
        open_options
            .open(path)
            .await
            .with_context(|| format!("Failed to create file at {:?}", path))?;
        Ok(())
    }

    async fn create_file_with(
        &self,
        path: &Path,
        content: Pin<&mut (dyn AsyncRead + Send)>,
    ) -> Result<()> {
        let mut file = smol::fs::File::create(&path)
            .await
            .with_context(|| format!("Failed to create file at {:?}", path))?;
        futures::io::copy(content, &mut file).await?;
        Ok(())
    }

    async fn copy_file(&self, source: &Path, target: &Path, options: CopyOptions) -> Result<()> {
        if !options.overwrite && smol::fs::metadata(target).await.is_ok() {
            if options.ignore_if_exists {
                return Ok(());
            } else {
                anyhow::bail!("{target:?} already exists");
            }
        }

        smol::fs::copy(source, target).await?;
        Ok(())
    }

    async fn rename(&self, source: &Path, target: &Path, options: RenameOptions) -> Result<()> {
        if options.create_parents {
            if let Some(parent) = target.parent() {
                self.create_dir(parent).await?;
            }
        }

        if options.overwrite {
            smol::fs::rename(source, target).await?;
            return Ok(());
        }

        let use_metadata_fallback = {
            #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
            {
                let source = source.to_path_buf();
                let target = target.to_path_buf();
                match self
                    .executor
                    .spawn(async move { rename_without_replace(&source, &target) })
                    .await
                {
                    Ok(()) => return Ok(()),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                        if options.ignore_if_exists {
                            return Ok(());
                        }
                        return Err(error.into());
                    }
                    #[cfg(unix)]
                    Err(error)
                        if error.raw_os_error().is_some_and(|code| {
                            code == libc::ENOSYS
                                || code == libc::ENOTSUP
                                || code == libc::EOPNOTSUPP
                                || code == libc::EINVAL
                        }) =>
                    {
                        // For case when filesystem or kernel does not support atomic no-overwrite rename.
                        // EINVAL is returned by FUSE-based filesystems (e.g. NTFS via ntfs-3g)
                        // that don't support RENAME_NOREPLACE.
                        true
                    }
                    Err(error) => return Err(error.into()),
                }
            }

            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                // For platforms which do not have an atomic no-overwrite rename yet.
                true
            }
        };

        if use_metadata_fallback && smol::fs::metadata(target).await.is_ok() {
            if options.ignore_if_exists {
                return Ok(());
            } else {
                anyhow::bail!("{target:?} already exists");
            }
        }

        smol::fs::rename(source, target).await?;
        Ok(())
    }

    async fn remove_dir(&self, path: &Path, options: RemoveOptions) -> Result<()> {
        let result = if options.recursive {
            smol::fs::remove_dir_all(path).await
        } else {
            smol::fs::remove_dir(path).await
        };
        match result {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound && options.ignore_if_not_exists => {
                Ok(())
            }
            Err(err) => Err(err)?,
        }
    }

    async fn remove_file(&self, path: &Path, options: RemoveOptions) -> Result<()> {
        #[cfg(windows)]
        if let Ok(Some(metadata)) = self.metadata(path).await
            && metadata.is_symlink
            && metadata.is_dir
        {
            self.remove_dir(
                path,
                RemoveOptions {
                    recursive: false,
                    ignore_if_not_exists: true,
                },
            )
            .await?;
            return Ok(());
        }

        match smol::fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound && options.ignore_if_not_exists => {
                Ok(())
            }
            Err(err) => Err(err)?,
        }
    }

    async fn open_sync(&self, path: &Path) -> Result<Box<dyn io::Read + Send + Sync>> {
        Ok(Box::new(std::fs::File::open(path)?))
    }

    async fn open_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(windows::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS.0);
        }
        Ok(Arc::new(options.open(path)?))
    }

    async fn load(&self, path: &Path) -> Result<String> {
        let path = path.to_path_buf();
        self.executor
            .spawn(async move {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read file {}", path.display()))
            })
            .await
    }

    async fn load_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let path = path.to_path_buf();
        let bytes = self
            .executor
            .spawn(async move { std::fs::read(path) })
            .await?;
        Ok(bytes)
    }

    #[cfg(not(target_os = "windows"))]
    async fn atomic_write(&self, path: PathBuf, data: String) -> Result<()> {
        smol::unblock(move || {
            // Use the directory of the destination as temp dir to avoid
            // invalid cross-device link error, and XDG_CACHE_DIR for fallback.
            // See https://github.com/zed-industries/zed/pull/8437 for more details.
            let mut tmp_file =
                tempfile::NamedTempFile::new_in(path.parent().unwrap_or(std::env::temp_dir()))?;
            tmp_file.write_all(data.as_bytes())?;
            tmp_file.persist(path)?;
            anyhow::Ok(())
        })
        .await?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn atomic_write(&self, path: PathBuf, data: String) -> Result<()> {
        smol::unblock(move || {
            // If temp dir is set to a different drive than the destination,
            // we receive error:
            //
            // failed to persist temporary file:
            // The system cannot move the file to a different disk drive. (os error 17)
            //
            // This is because `ReplaceFileW` does not support cross volume moves.
            // See the remark section: "The backup file, replaced file, and replacement file must all reside on the same volume."
            // https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew#remarks
            //
            // So we use the directory of the destination as a temp dir to avoid it.
            // https://github.com/zed-industries/zed/issues/16571
            let temp_dir = TempDir::new_in(path.parent().unwrap_or(&std::env::temp_dir()))?;
            let temp_file = {
                let temp_file_path = temp_dir.path().join("temp_file");
                let mut file = std::fs::File::create_new(&temp_file_path)?;
                file.write_all(data.as_bytes())?;
                temp_file_path
            };
            atomic_replace(path.as_path(), temp_file.as_path())?;
            anyhow::Ok(())
        })
        .await?;
        Ok(())
    }

    async fn save(&self, path: &Path, text: &str) -> Result<()> {
        let buffer_size = text.len().min(10 * 1024);
        if let Some(path) = path.parent() {
            self.create_dir(path)
                .await
                .with_context(|| format!("Failed to create directory at {:?}", path))?;
        }
        let file = smol::fs::File::create(path)
            .await
            .with_context(|| format!("Failed to create file at {:?}", path))?;
        let mut writer = smol::io::BufWriter::with_capacity(buffer_size, file);
        writer.write_all(text.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }

    async fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        if let Some(path) = path.parent() {
            self.create_dir(path)
                .await
                .with_context(|| format!("Failed to create directory at {:?}", path))?;
        }
        let path = path.to_owned();
        let contents = content.to_owned();
        self.executor
            .spawn(async move {
                std::fs::write(path, contents)?;
                Ok(())
            })
            .await
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let path = path.to_owned();
        self.executor
            .spawn(async move {
                #[cfg(target_os = "windows")]
                let result = Self::canonicalize(&path);

                #[cfg(not(target_os = "windows"))]
                let result = std::fs::canonicalize(&path);

                result.with_context(|| format!("canonicalizing {path:?}"))
            })
            .await
    }

    async fn is_file(&self, path: &Path) -> bool {
        let path = path.to_owned();
        self.executor
            .spawn(async move { std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) })
            .await
    }

    async fn is_dir(&self, path: &Path) -> bool {
        let path = path.to_owned();
        self.executor
            .spawn(async move { std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir()) })
            .await
    }

    async fn metadata(&self, path: &Path) -> Result<Option<Metadata>> {
        let path_buf = path.to_owned();
        let symlink_metadata = match self
            .executor
            .spawn(async move { std::fs::symlink_metadata(&path_buf) })
            .await
        {
            Ok(metadata) => metadata,
            Err(err) => {
                return match err.kind() {
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => Ok(None),
                    _ => Err(anyhow::Error::new(err)),
                };
            }
        };

        let is_symlink = symlink_metadata.file_type().is_symlink();
        let metadata = if is_symlink {
            let path_buf = path.to_path_buf();
            // Read target metadata, if the target exists
            match self
                .executor
                .spawn(async move { std::fs::metadata(path_buf) })
                .await
            {
                Ok(target_metadata) => target_metadata,
                Err(err) => {
                    if err.kind() != io::ErrorKind::NotFound {
                        // TODO: Also FilesystemLoop when that's stable
                        log::warn!(
                            "Failed to read symlink target metadata for path {path:?}: {err}"
                        );
                    }
                    // For a broken or recursive symlink, return the symlink metadata. (Or
                    // as edge cases, a symlink into a directory we can't read, which is hard
                    // to distinguish from just being broken.)
                    symlink_metadata
                }
            }
        } else {
            symlink_metadata
        };

        #[cfg(unix)]
        let inode = metadata.ino();

        #[cfg(windows)]
        let inode = file_id(path).await?;

        #[cfg(windows)]
        let is_fifo = false;

        #[cfg(unix)]
        let is_fifo = metadata.file_type().is_fifo();

        let path_buf = path.to_path_buf();
        let is_executable = self
            .executor
            .spawn(async move { path_buf.is_executable() })
            .await;

        Ok(Some(Metadata {
            inode,
            mtime: MTime(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)),
            len: metadata.len(),
            is_symlink,
            is_dir: metadata.file_type().is_dir(),
            is_fifo,
            is_executable,
            is_writable: !metadata.permissions().readonly(),
        }))
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf> {
        let path = path.to_owned();
        let path = self
            .executor
            .spawn(async move { std::fs::read_link(&path) })
            .await?;
        Ok(path)
    }

    async fn read_dir(
        &self,
        path: &Path,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PathBuf>>>>> {
        let path = path.to_owned();
        let result = iter(
            self.executor
                .spawn(async move { std::fs::read_dir(path) })
                .await?,
        )
        .map(|entry| match entry {
            Ok(entry) => Ok(entry.path()),
            Err(error) => Err(anyhow!("failed to read dir entry {error:?}")),
        });
        Ok(Box::pin(result))
    }

    async fn watch(
        &self,
        path: &Path,
        latency: Duration,
    ) -> (
        Pin<Box<dyn Send + Stream<Item = Vec<PathEvent>>>>,
        Arc<dyn Watcher>,
    ) {
        let executor = self.executor.clone();

        let (tx, rx) = async_channel::unbounded();
        let pending_paths: Arc<Mutex<Vec<PathEvent>>> = Default::default();

        let watcher: Arc<dyn Watcher> = Arc::new(fs_watcher::FsWatcher::new(
            executor.clone(),
            tx.clone(),
            pending_paths.clone(),
        ));

        if let Err(e) = watcher.add(path) {
            log::warn!("Failed to watch {}:\n{e}", path.display());
        }

        // Check if path is a symlink and follow the target parent
        if let Some(mut target) = self.read_link(path).await.ok() {
            log::trace!("watch symlink {path:?} -> {target:?}");
            // Check if symlink target is relative path, if so make it absolute
            if target.is_relative()
                && let Some(parent) = path.parent()
            {
                target = parent.join(target);
                if let Ok(canonical) = self.canonicalize(&target).await {
                    target = canonical;
                }
            }
            watcher.add(&target).ok();
            // Skipped for poll watchers: PollWatcher::watch() recursively scans
            // at registration, blocking on large virtual filesystem mounts
            if let Some(parent) = target.parent()
                && !fs_watcher::requires_poll_watcher(parent)
            {
                let _ = watcher.add(parent).inspect_err(|e| log::error!("{e}"));
            }
        }

        (
            Box::pin(rx.filter_map({
                let watcher = watcher.clone();
                let executor = executor.clone();
                move |_| {
                    let _ = watcher.clone();
                    let pending_paths = pending_paths.clone();
                    let executor = executor.clone();
                    async move {
                        executor.timer(latency).await;
                        let paths = std::mem::take(&mut *pending_paths.lock());
                        log::debug!("pending path events: {:?}", paths);
                        (!paths.is_empty()).then_some(paths)
                    }
                }
            })),
            watcher,
        )
    }

    /// Runs `git config` with the given arguments.
    /// Will return `Ok` if the commands exit status is `0`, with the stdout
    /// contents. Otherwise returns `Err` with the stderr contents.

    /// Checks whether the file system is case sensitive by attempting to create two files
    /// that have the same name except for the casing.
    ///
    /// It creates both files in a temporary directory it removes at the end.
    async fn is_case_sensitive(&self) -> bool {
        const UNINITIALIZED: u8 = 0;
        const CASE_SENSITIVE: u8 = 1;
        const NOT_CASE_SENSITIVE: u8 = 2;

        // Note we could CAS here, but really, if we race we do this work twice at worst which isn't a big deal.
        let load = self.is_case_sensitive.load(Ordering::Acquire);
        if load != UNINITIALIZED {
            return load == CASE_SENSITIVE;
        }
        let Ok(temp_dir) = self.executor.spawn(async { TempDir::new() }).await else {
            log::error!(
                "Failed to determine whether filesystem is case sensitive (falling back to true)"
            );
            self.is_case_sensitive
                .store(NOT_CASE_SENSITIVE, Ordering::Release);
            return true;
        };
        let test_file_1 = temp_dir.path().join("case_sensitivity_test.tmp");
        let test_file_2 = temp_dir.path().join("CASE_SENSITIVITY_TEST.TMP");

        let create_opts = CreateOptions {
            overwrite: false,
            ignore_if_exists: false,
        };

        let case_sensitive = match self.create_file(&test_file_1, create_opts).await {
            Ok(_) => match self.create_file(&test_file_2, create_opts).await {
                Ok(_) => true,
                Err(ref e)
                    if e.downcast_ref::<io::Error>()
                        .map_or(false, |ioe| ioe.kind() == io::ErrorKind::AlreadyExists) =>
                {
                    false
                }
                Err(e) => {
                    log::error!("Failed to check case sensitivity: {e:#}");
                    let _ = temp_dir.close();
                    return true;
                }
            },
            Err(e) => {
                log::error!("Failed to create test file for case sensitivity: {e:#}");
                let _ = temp_dir.close();
                return true;
            }
        };
        let _ = temp_dir.close();
        self.is_case_sensitive.store(
            if case_sensitive {
                CASE_SENSITIVE
            } else {
                NOT_CASE_SENSITIVE
            },
            Ordering::Release,
        );
        case_sensitive
    }
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
impl Watcher for RealWatcher {
    fn add(&self, _: &Path) -> Result<()> {
        Ok(())
    }

    fn remove(&self, _: &Path) -> Result<()> {
        Ok(())
    }
}

pub async fn copy_recursive<'a>(
    fs: &'a dyn Fs,
    source: &'a Path,
    target: &'a Path,
    options: CopyOptions,
) -> Result<()> {
    for (item, is_dir) in read_dir_items(fs, source).await? {
        let Ok(item_relative_path) = item.strip_prefix(source) else {
            continue;
        };
        let target_item = if item_relative_path == Path::new("") {
            target.to_path_buf()
        } else {
            target.join(item_relative_path)
        };
        if is_dir {
            if !options.overwrite && fs.metadata(&target_item).await.is_ok_and(|m| m.is_some()) {
                if options.ignore_if_exists {
                    continue;
                } else {
                    anyhow::bail!("{target_item:?} already exists");
                }
            }
            let _ = fs
                .remove_dir(
                    &target_item,
                    RemoveOptions {
                        recursive: true,
                        ignore_if_not_exists: true,
                    },
                )
                .await;
            fs.create_dir(&target_item).await?;
        } else {
            fs.copy_file(&item, &target_item, options).await?;
        }
    }
    Ok(())
}

/// Recursively reads all of the paths in the given directory.
///
/// Returns a vector of tuples of (path, is_dir).
pub async fn read_dir_items<'a>(fs: &'a dyn Fs, source: &'a Path) -> Result<Vec<(PathBuf, bool)>> {
    let mut items = Vec::new();
    read_recursive(fs, source, &mut items).await?;
    Ok(items)
}

fn read_recursive<'a>(
    fs: &'a dyn Fs,
    source: &'a Path,
    output: &'a mut Vec<(PathBuf, bool)>,
) -> BoxFuture<'a, Result<()>> {
    use futures::future::FutureExt;

    async move {
        let metadata = fs
            .metadata(source)
            .await?
            .with_context(|| format!("path does not exist: {source:?}"))?;

        if metadata.is_dir {
            output.push((source.to_path_buf(), true));
            let mut children = fs.read_dir(source).await?;
            while let Some(child_path) = children.next().await {
                if let Ok(child_path) = child_path {
                    read_recursive(fs, &child_path, output).await?;
                }
            }
        } else {
            output.push((source.to_path_buf(), false));
        }
        Ok(())
    }
    .boxed()
}

// todo(windows)
// can we get file id not open the file twice?
// https://github.com/rust-lang/rust/issues/63010
#[cfg(target_os = "windows")]
async fn file_id(path: impl AsRef<Path>) -> Result<u64> {
    use std::os::windows::io::AsRawHandle;

    use smol::fs::windows::OpenOptionsExt;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, GetFileInformationByHandle,
        },
    };

    let file = smol::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(path)
        .await?;

    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle
    // This function supports Windows XP+
    smol::unblock(move || {
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle() as _), &mut info)? };

        Ok(((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64))
    })
    .await
}

#[cfg(target_os = "windows")]
fn atomic_replace<P: AsRef<Path>>(
    replaced_file: P,
    replacement_file: P,
) -> windows::core::Result<()> {
    use windows::{
        Win32::Storage::FileSystem::{REPLACE_FILE_FLAGS, ReplaceFileW},
        core::HSTRING,
    };

    // If the file does not exist, create it.
    let _ = std::fs::File::create_new(replaced_file.as_ref());

    unsafe {
        ReplaceFileW(
            &HSTRING::from(replaced_file.as_ref().to_string_lossy().into_owned()),
            &HSTRING::from(replacement_file.as_ref().to_string_lossy().into_owned()),
            None,
            REPLACE_FILE_FLAGS::default(),
            None,
            None,
        )
    }
}
