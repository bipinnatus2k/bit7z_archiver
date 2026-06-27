mod common;

use bit7z_archiver::adapters::bit7z::{self, WriterCompressionLevel, WriterFormat};
use bit7z_archiver::adapters::repository::Bit7zRepository;
use bit7z_archiver::domain::repository::ArchiveRepository;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ── Configuration ──────────────────────────────────────────────────────────

struct Scale {
    label: &'static str,
    target_entries: u64,
    max_depth: u32,
}

const SCALES: &[Scale] = &[
    Scale { label: "100", target_entries: 100, max_depth: 6 },
    Scale { label: "500", target_entries: 500, max_depth: 8 },
    Scale { label: "2000", target_entries: 2000, max_depth: 10 },
    Scale { label: "10000", target_entries: 10000, max_depth: 12 },
    Scale { label: "50000", target_entries: 50000, max_depth: 14 },
];

// ── Helpers ────────────────────────────────────────────────────────────────

fn has_7z_library() -> bool {
    bit7z_archiver::adapters::platform::find_7z_library().is_some()
}

fn open_lib() -> Option<bit7z::Library> {
    let p = bit7z_archiver::adapters::platform::find_7z_library()?;
    bit7z::Library::open(p.to_str()?).ok()
}

fn create_repo() -> Option<Arc<dyn ArchiveRepository>> {
    let p = bit7z_archiver::adapters::platform::find_7z_library()?;
    let lib = bit7z::Library::open(p.to_str()?).ok()?;
    Some(Arc::new(Bit7zRepository::new(lib)))
}

fn cached_archive(scale: &Scale) -> PathBuf {
    common::fixtures_dir().join(&format!("large_{}.7z", scale.label))
}

// ── Tree generator ────────────────────────────────────────────────────────
// Creates a binary tree with max_depth levels. Each node gets 1 file,
// then creates 2 child directories (unless at max depth).
// Total entries ≈ 3 * 2^(depth-1) - 1 for depth > 0.

fn generate_tree(base: &Path, target: u64, max_depth: u32) -> u64 {
    let mut total = 0u64;
    let mut dirs: Vec<PathBuf> = vec![base.to_path_buf()];

    while total < target && !dirs.is_empty() {
        let d = dirs.remove(0);

        if total >= target {
            break;
        }
        let fpath = d.join(format!("f{}.dat", total));
        let _ = std::fs::write(&fpath, b"x");
        total += 1;

        let depth = d
            .strip_prefix(base)
            .map(|p| p.components().count())
            .unwrap_or(0);

        if depth < max_depth as usize && total < target {
            for child_idx in 0..2u64 {
                if total >= target {
                    break;
                }
                let sub = d.join(format!("d{}", child_idx));
                let _ = std::fs::create_dir(&sub);
                total += 1;
                dirs.push(sub);
            }
        }
    }

    total
}

// ── Bench result ──────────────────────────────────────────────────────────

struct BenchResult {
    scale: String,
    operation: &'static str,
    val: String,
}

fn fmt_dur(d: Duration) -> String {
    if d.as_secs() > 0 {
        format!("{:.2}s", d.as_secs_f64())
    } else if d.as_micros() >= 1000 {
        format!("{:.1}ms", d.as_secs_f64() * 1000.0)
    } else if d.as_nanos() >= 1000 {
        format!("{}µs", d.as_nanos() / 1000)
    } else {
        format!("{}ns", d.as_nanos())
    }
}

fn avg_dur(results: &[Duration]) -> Duration {
    let sum: Duration = results.iter().sum();
    sum / results.len() as u32
}

// ── Main benchmark ────────────────────────────────────────────────────────

#[test]
fn large_archive_bench() {
    if !has_7z_library() {
        eprintln!("SKIP: 7z.dll not found");
        return;
    }
    let has_cli = common::find_7z_cli().is_some();
    if !has_cli {
        eprintln!("NOTE: 7z CLI not found — skip CLI create/verify");
    }

    let lib = open_lib().expect("Failed to load 7z library");
    let repo = create_repo().expect("Failed to create repository");
    let mut results: Vec<BenchResult> = Vec::new();

    for scale in SCALES {
        let label = scale.label;
        let cached = cached_archive(scale);
        let temp_dir = common::temp_dir();

        // ── STEP 1: Generate directory tree ──
        let tree_dir = temp_dir.join(&format!("tree_{}", label));
        std::fs::create_dir_all(&tree_dir).unwrap();

        let gen_start = Instant::now();
        let actual_entries = generate_tree(&tree_dir, scale.target_entries, scale.max_depth);
        let gen_time = gen_start.elapsed();

        results.push(BenchResult {
            scale: label.to_string(),
            operation: "Tree gen (entries)",
            val: actual_entries.to_string(),
        });
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "Tree gen (time)",
            val: fmt_dur(gen_time),
        });

        let tree_size = dir_size(&tree_dir);
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "Tree size",
            val: format_bytes(tree_size),
        });

        // ── STEP 2: Create archive via CLI (if available), read via safe API ──
        if has_cli {
            let cli_archive = if cached.exists() {
                eprintln!("  Using cached {} CLI archive", label);
                cached.clone()
            } else {
                eprintln!("  Creating {} CLI archive...", label);
                let start = Instant::now();
                let out = cached.to_str().unwrap();
                let src = tree_dir.to_str().unwrap();
                match common::run_7z(&["a", "-mx=1", "-bsp0", "-bb0", out, src]) {
                    Ok(_) => {
                        results.push(BenchResult {
                            scale: label.to_string(),
                            operation: "CLI create",
                            val: fmt_dur(start.elapsed()),
                        });
                        cached.clone()
                    }
                    Err(e) => {
                        eprintln!("  CLI create failed: {}", e);
                        results.push(BenchResult {
                            scale: label.to_string(),
                            operation: "CLI create",
                            val: format!("FAIL: {}", e),
                        });
                        continue;
                    }
                }
            };

            let size = file_size(&cli_archive);
            results.push(BenchResult {
                scale: label.to_string(),
                operation: "CLI archive size",
                val: format_bytes(size),
            });

            // Read CLI archive via safe API
            bench_open_enum_extract(&lib, &cli_archive, label, &mut results);
            bench_list_directory(repo.as_ref(), &cli_archive, label, &mut results);
        }

        // ── STEP 3: Create archive via Safe API ──
        let writer_archive = temp_dir.join(&format!("writer_{}.7z", label));
        {
            let start = Instant::now();
            let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
            w.set_compression_level(WriterCompressionLevel::Fastest);
            let _ = w.add_directory(tree_dir.to_str().unwrap());
            match w.compress_to(writer_archive.to_str().unwrap()) {
                Ok(()) => {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "Writer create",
                        val: fmt_dur(start.elapsed()),
                    });
                }
                Err(e) => {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "Writer create",
                        val: format!("FAIL: {}", e),
                    });
                    continue;
                }
            }
        }

        let size = file_size(&writer_archive);
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "Writer archive size",
            val: format_bytes(size),
        });

        // Read writer archive via safe API
        bench_open_enum_extract(&lib, &writer_archive, label, &mut results);
        bench_list_directory(repo.as_ref(), &writer_archive, label, &mut results);

        // Verify writer archive via CLI
        if has_cli {
            let start = Instant::now();
            match common::verify_7z(&writer_archive) {
                Ok(ok) => {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "CLI verify writer",
                        val: if ok {
                            format!("OK ({})", fmt_dur(start.elapsed()))
                        } else {
                            format!("CORRUPT ({})", fmt_dur(start.elapsed()))
                        },
                    });
                    if let Ok(count) = common::count_entries_7z(&writer_archive) {
                        results.push(BenchResult {
                            scale: label.to_string(),
                            operation: "CLI count writer",
                            val: count.to_string(),
                        });
                    }
                }
                Err(e) => {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "CLI verify writer",
                        val: format!("FAIL: {}", e),
                    });
                }
            }
        }
    }

    print_report(&results);
}

// ── Benchmark: open, enumerate all items, extract first file ──────────────

fn bench_open_enum_extract(
    lib: &bit7z::Library,
    path: &Path,
    label: &str,
    results: &mut Vec<BenchResult>,
) {
    let path_str = path.to_str().unwrap();

    // Open
    let mut open_times = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        if let Ok(r) = bit7z::ArchiveReader::open(lib, path_str, None) {
            open_times.push(start.elapsed());
            drop(r);
        }
    }
    if !open_times.is_empty() {
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "Open archive",
            val: fmt_dur(avg_dur(&open_times)),
        });
    }

    // Full enumeration
    let reader = match bit7z::ArchiveReader::open(lib, path_str, None) {
        Ok(r) => r,
        Err(_) => {
            results.push(BenchResult {
                scale: label.to_string(),
                operation: "Enumerate all",
                val: "FAIL".into(),
            });
            return;
        }
    };

    let count = reader.item_count();
    results.push(BenchResult {
        scale: label.to_string(),
        operation: "Item count",
        val: count.to_string(),
    });

    let mut enum_times = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        for i in 0..count {
            let item = reader.item(i);
            let _ = item.path();
            let _ = item.size();
        }
        enum_times.push(start.elapsed());
    }
    results.push(BenchResult {
        scale: label.to_string(),
        operation: "Enumerate all",
        val: fmt_dur(avg_dur(&enum_times)),
    });

    // Extract first actual file (skip directories)
    let first_file = (0..count).find(|&i| !reader.item(i).is_directory());
    if let Some(idx) = first_file {
        let mut xtract_times = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            let _ = reader.extract_to_buffer(idx);
            xtract_times.push(start.elapsed());
        }
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "Extract first file",
            val: fmt_dur(avg_dur(&xtract_times)),
        });
    } else {
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "Extract first file",
            val: "NO_FILE".into(),
        });
    }

    drop(reader);
}

// ── Benchmark: list_directory root + deep path ────────────────────────────

fn bench_list_directory(
    repo: &dyn ArchiveRepository,
    path: &Path,
    label: &str,
    results: &mut Vec<BenchResult>,
) {
    let handle = match repo.open(path, None) {
        Ok(h) => h,
        Err(_) => {
            results.push(BenchResult {
                scale: label.to_string(),
                operation: "list_directory(root)",
                val: "FAIL".into(),
            });
            return;
        }
    };

    // Root listing
    let mut root_times = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        if let Ok(entries) = repo.list_directory(&handle, "") {
            root_times.push(start.elapsed());
            results.push(BenchResult {
                scale: label.to_string(),
                operation: "list_root count",
                val: entries.len().to_string(),
            });
        }
    }
    if !root_times.is_empty() {
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "list_directory(root)",
            val: fmt_dur(avg_dur(&root_times)),
        });
    }

    // Detect root prefix from archive root entries.
    let root_prefix = repo.list_directory(&handle, "").ok()
        .and_then(|entries| entries.first().cloned())
        .filter(|e| e.is_directory)
        .map(|e| {
            let cleaned = e.path.trim_end_matches(&['/', '\\'][..]).to_string();
            if cleaned.is_empty() { String::new() } else { format!("{}/", cleaned) }
        })
        .unwrap_or_default();

    // Find deepest directory with children for the deep path benchmark.
    // Leaf directories (at max_depth) are empty because the generator stops before filling them.
    let mut best_depth = 0usize;
    for depth in 1..=10 {
        let mut p = root_prefix.clone();
        for _ in 0..depth { p.push_str("d0/"); }
        if let Ok(kids) = repo.list_directory(&handle, &p) {
            if !kids.is_empty() { best_depth = depth; }
        }
    }

    let mut deep_path = root_prefix.clone();
    for _ in 0..best_depth {
        deep_path.push_str("d0/");
    }
    let mut deep_times = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        match repo.list_directory(&handle, &deep_path) {
            Ok(entries) => {
                deep_times.push(start.elapsed());
                if deep_times.len() == 1 {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "list_deep count",
                        val: entries.len().to_string(),
                    });
                }
            }
            Err(_) => {
                if deep_times.is_empty() {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "list_directory(deep)",
                        val: "NOT_FOUND".into(),
                    });
                }
                break;
            }
        }
    }
    if !deep_times.is_empty() {
        results.push(BenchResult {
            scale: label.to_string(),
            operation: "list_directory(deep)",
            val: fmt_dur(avg_dur(&deep_times)),
        });
    }

    repo.close(handle);
}

// ── Report ────────────────────────────────────────────────────────────────

fn print_report(results: &[BenchResult]) {
    let labels: Vec<&str> = SCALES.iter().map(|s| s.label).collect();
    let ops: Vec<&str> = {
        let mut v: Vec<&str> = Vec::new();
        for r in results {
            if !v.contains(&r.operation) {
                v.push(r.operation);
            }
        }
        v
    };

    println!();
    println!("╔{}╗", "═".repeat(14 + labels.len() * 20));
    println!(
        "║{:>14}║{}║",
        "Scale",
        labels
            .iter()
            .map(|l| format!("{:>18} ", l))
            .collect::<Vec<_>>()
            .join("║")
    );
    println!("╠{}╣", "═".repeat(14 + labels.len() * 20));

    for op in &ops {
        print!("║{:>14}║", op);
        for label in &labels {
            let val = results
                .iter()
                .find(|r| r.scale.as_str() == *label && r.operation == *op)
                .map(|r| r.val.as_str())
                .unwrap_or("-");
            print!("{:>18} ", val);
        }
        println!("║");
    }

    println!("╚{}╝", "═".repeat(14 + labels.len() * 20));
    println!();
    println!("NOTE: CLI operations skipped if 7z.exe not in PATH.");
    println!("      Cached fixtures (*.7z) are reused if they exist.");
}

// ── Utility ───────────────────────────────────────────────────────────────

fn dir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path);
            } else if let Ok(meta) = std::fs::metadata(&path) {
                total += meta.len();
            }
        }
    }
    total
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
