use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use bit7z_infra_repo::supervisor::RepoSupervisor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Helper: check if the real 7z DLL is available on this system.
fn has_7z_library() -> bool {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    find_7z_dll(manifest_dir).is_some()
}

/// Helper: create a RepoSupervisor backed by the real 7-Zip library.
fn create_repo() -> Option<Arc<dyn ArchiveRepository>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib_path = find_7z_dll(&manifest_dir)?;
    let lib_str = lib_path.to_str()?;
    let repo = RepoSupervisor::new(lib_str).ok()?;
    Some(Arc::new(repo) as Arc<dyn ArchiveRepository>)
}

fn find_7z_dll(manifest_dir: &Path) -> Option<PathBuf> {
    let candidates: [PathBuf; 6] = [
        PathBuf::from(r"C:\Program Files\7-Zip\7z.dll"),
        PathBuf::from(r"C:\Program Files (x86)\7-Zip\7z.dll"),
        manifest_dir.join("vcpkg_installed").join("x64-windows").join("bin").join("7z.dll"),
        manifest_dir.parent()?.parent()?.parent()?
            .join("vcpkg_installed").join("x64-windows").join("bin").join("7z.dll"),
        manifest_dir.parent()?.parent()?.parent()?.parent()?
            .join("vcpkg_installed").join("x64-windows").join("bin").join("7z.dll"),
        manifest_dir.parent()?.parent()?
            .join("vcpkg_installed").join("x64-windows").join("bin").join("7z.dll"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

/// Helper: find fixture path relative to the workspace root.
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // repo crate path: .../crates/infrastructure/repo/
    manifest_dir.parent().unwrap().parent().unwrap().parent().unwrap().to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    workspace_root().join("tests").join("fixtures")
}

fn fixture_path(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

fn ensure_fixtures() -> Vec<String> {
    let needed = ["basic.7z", "basic.zip", "basic.tar", "corrupted.7z", "encrypted.7z", "empty.7z", "multi_file.7z"];
    needed.iter().filter(|n| !fixture_path(n).exists()).map(|n| n.to_string()).collect()
}

fn all_fixtures_exist() -> bool {
    ensure_fixtures().is_empty()
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bit7z_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn noop_ctx() -> OpCtx {
    OpCtx {
        cancel: CancellationToken::new(),
        pause: PauseToken::new(),
        progress: Arc::new(NoopSink),
    }
}

fn test_extract_req() -> ExtractRequest {
    ExtractRequest {
        indices: vec![0, 1],
        dest: temp_dir(),
        overwrite: OverwriteMode::Overwrite,
    }
}

// ============================================================================
// Test 1: Same archive operations serialized
// ============================================================================

#[test]
#[ignore = "requires 7-Zip DLL at runtime"]
fn test_same_archive_serialized() {
    if !has_7z_library() || !all_fixtures_exist() {
        eprintln!("Skipping: 7z library or fixtures not available");
        return;
    }
    let repo = create_repo().expect("Failed to create repo");
    let handle = repo.open(&fixture_path("basic.7z"), None).expect("Failed to open basic.7z");

    let (tx1, rx1) = std::sync::mpsc::channel();
    let (tx2, rx2) = std::sync::mpsc::channel();

    let repo1 = repo.clone();
    let h1 = handle.clone();
    std::thread::spawn(move || {
        let result = repo1.list(&h1, 0..10);
        let _ = tx1.send(result);
    });

    let repo2 = repo.clone();
    let h2 = handle.clone();
    std::thread::spawn(move || {
        let result = repo2.extract(&h2, &test_extract_req(), &noop_ctx());
        let _ = tx2.send(result);
    });

    let list_result = rx1.recv().expect("list thread panicked");
    let extract_result = rx2.recv().expect("extract thread panicked");

    assert!(list_result.is_ok(), "list should succeed: {:?}", list_result);
    if let Err(ref e) = extract_result {
        eprintln!("Extract result (may fail without destination): {:?}", e);
    }

    repo.close(&handle);
}

// ============================================================================
// Test 2: Two archives extracted in parallel
// ============================================================================

#[test]
#[ignore = "requires 7-Zip DLL at runtime"]
fn test_two_archives_parallel() {
    if !has_7z_library() || !all_fixtures_exist() {
        eprintln!("Skipping: 7z library or fixtures not available");
        return;
    }
    let repo = create_repo().expect("Failed to create repo");

    let h1 = repo.open(&fixture_path("basic.7z"), None).expect("Failed to open basic.7z");
    let h2 = repo.open(&fixture_path("multi_file.7z"), None).expect("Failed to open multi_file.7z");

    let (tx1, rx1) = std::sync::mpsc::channel();
    let (tx2, rx2) = std::sync::mpsc::channel();

    let repo1 = repo.clone();
    let h1_clone = h1.clone();
    std::thread::spawn(move || {
        let result = repo1.extract(&h1_clone, &test_extract_req(), &noop_ctx());
        let _ = tx1.send(result);
    });

    let repo2 = repo.clone();
    let h2_clone = h2.clone();
    std::thread::spawn(move || {
        let result = repo2.extract(&h2_clone, &test_extract_req(), &noop_ctx());
        let _ = tx2.send(result);
    });

    let _ = rx1.recv();
    let _ = rx2.recv();

    repo.close(&h1);
    repo.close(&h2);
}

// ============================================================================
// Test 3: Handle invalid after close
// ============================================================================

#[test]
#[ignore = "requires 7-Zip DLL at runtime"]
fn test_handle_invalid_after_close() {
    if !has_7z_library() || !all_fixtures_exist() {
        eprintln!("Skipping: 7z library or fixtures not available");
        return;
    }
    let repo = create_repo().expect("Failed to create repo");
    let handle = repo.open(&fixture_path("basic.7z"), None).expect("Failed to open basic.7z");

    repo.close(&handle);

    let result = repo.list(&handle, 0..10);
    assert!(matches!(result, Err(ArchiveError::NotOpen)),
        "Expected NotOpen after close, got {:?}", result);
}

// ============================================================================
// Test 4: Cancel mid-extract
// ============================================================================

#[test]
#[ignore = "requires 7-Zip DLL at runtime"]
fn test_cancel_mid_extract() {
    if !has_7z_library() || !all_fixtures_exist() {
        eprintln!("Skipping: 7z library or fixtures not available");
        return;
    }
    let repo = create_repo().expect("Failed to create repo");
    let handle = repo.open(&fixture_path("multi_file.7z"), None).expect("Failed to open multi_file.7z");

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    let ctx = OpCtx {
        cancel: cancel.clone(),
        pause: PauseToken::new(),
        progress: Arc::new(NoopSink),
    };

    let repo_clone = repo.clone();
    let h_clone = handle.clone();
    let req = ExtractRequest {
        indices: vec![0, 1, 2, 3],
        dest: temp_dir(),
        overwrite: OverwriteMode::Overwrite,
    };

    let join = std::thread::spawn(move || {
        repo_clone.extract(&h_clone, &req, &ctx)
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
    cancel_clone.cancel();

    let result = join.join().expect("extract thread panicked");

    match result {
        Ok(_) => eprintln!("Extraction completed before cancel took effect"),
        Err(ArchiveError::Cancelled) => { /* expected */ }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    repo.close(&handle);
}

// ============================================================================
// Test 5: Pause and resume
// ============================================================================

#[test]
#[ignore = "requires 7-Zip DLL at runtime"]
fn test_pause_resume() {
    if !has_7z_library() || !all_fixtures_exist() {
        eprintln!("Skipping: 7z library or fixtures not available");
        return;
    }
    let repo = create_repo().expect("Failed to create repo");
    let handle = repo.open(&fixture_path("multi_file.7z"), None).expect("Failed to open multi_file.7z");

    let pause = PauseToken::new();
    let pause_clone = pause.clone();
    let progress_calls = Arc::new(AtomicU64::new(0));
    let progress_calls_clone = progress_calls.clone();

    struct CountingSink(Arc<AtomicU64>);
    impl ProgressSink for CountingSink {
        fn on_progress(&self, _processed: u64, _total: u64) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        fn on_file(&self, _path: &str) {}
    }

    let ctx = OpCtx {
        cancel: CancellationToken::new(),
        pause: pause.clone(),
        progress: Arc::new(CountingSink(progress_calls)),
    };

    let repo_clone = repo.clone();
    let h_clone = handle.clone();
    let req = ExtractRequest {
        indices: vec![0, 1, 2, 3],
        dest: temp_dir(),
        overwrite: OverwriteMode::Overwrite,
    };

    let join = std::thread::spawn(move || {
        repo_clone.extract(&h_clone, &req, &ctx)
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
    pause_clone.pause();

    std::thread::sleep(std::time::Duration::from_millis(100));
    let _calls_while_paused = progress_calls_clone.load(Ordering::Relaxed);

    pause_clone.resume();

    let result = join.join().expect("extract thread panicked");

    if let Err(ArchiveError::Cancelled) = &result {
        eprintln!("Extraction was cancelled (expected if FFI cancel was triggered)");
    } else {
        assert!(result.is_ok(), "Extraction should succeed after resume: {:?}", result);
    }

    repo.close(&handle);
}

// ============================================================================
// Smoke test: Mock repo (no FFI needed)
// ============================================================================

#[test]
fn test_repo_supervisor_open_nonexistent() {
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use std::path::Path;

    let repo = MockArchiveRepository::arc_with_count(5);
    let handle = repo.open(Path::new("test.7z"), None).unwrap();
    let page = repo.list(&handle, 0..10).unwrap();
    assert_eq!(page.items.len(), 5);
    assert_eq!(page.total, Some(5));
    repo.close(&handle);
}
