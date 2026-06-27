mod common;

use bit7z_archiver::adapters::bit7z;
use bit7z_archiver::adapters::repository::Bit7zRepository;
use bit7z_archiver::domain::repository::ArchiveRepository;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Scale {
    label: &'static str,
    target_entries: u64,
}

const SCALES: &[Scale] = &[
    Scale { label: "100", target_entries: 100 },
    Scale { label: "500", target_entries: 500 },
    Scale { label: "2000", target_entries: 2000 },
];

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

fn fixture_archive(label: &str) -> PathBuf {
    common::fixtures_dir().join(&format!("large_{}.7z", label))
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

struct BenchResult {
    scale: String,
    operation: &'static str,
    val: String,
}

#[test]
fn open_list_bench() {
    if !has_7z_library() {
        eprintln!("SKIP: 7z.dll not found");
        return;
    }

    let repo = create_repo().expect("Failed to create repository");
    let mut results: Vec<BenchResult> = Vec::new();

    for scale in SCALES {
        let label = scale.label;
        let path = fixture_archive(label);
        if !path.exists() {
            eprintln!("  SKIP {}: no cached fixture", label);
            continue;
        }

        // ── Step 1: repo.open() ──
        let mut open_times = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            if let Ok(handle) = repo.open(&path, None) {
                open_times.push(start.elapsed());
                repo.close(handle);
            }
        }
        if !open_times.is_empty() {
            results.push(BenchResult {
                scale: label.to_string(),
                operation: "repo.open()",
                val: fmt_dur(avg_dur(&open_times)),
            });
        }

        // ── Step 2: repo.get_properties() ──
        let handle = repo.open(&path, None).expect("open failed");
        let mut props_times = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            if let Ok(props) = repo.get_properties(&handle) {
                let elapsed = start.elapsed();
                props_times.push(elapsed);
                if props_times.len() == 1 {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "items_count",
                        val: props.items_count.to_string(),
                    });
                }
            }
        }
        if !props_times.is_empty() {
            results.push(BenchResult {
                scale: label.to_string(),
                operation: "repo.get_properties()",
                val: fmt_dur(avg_dur(&props_times)),
            });
        }

        // ── Step 3: repo.list_directory("") ──
        let mut list_times = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            if let Ok(entries) = repo.list_directory(&handle, "") {
                list_times.push(start.elapsed());
                if list_times.len() == 1 {
                    results.push(BenchResult {
                        scale: label.to_string(),
                        operation: "list_dir entries",
                        val: entries.len().to_string(),
                    });
                }
            }
        }
        if !list_times.is_empty() {
            results.push(BenchResult {
                scale: label.to_string(),
                operation: "repo.list_directory(\"\")",
                val: fmt_dur(avg_dur(&list_times)),
            });
        }

        repo.close(handle);
    }

    print_report(&results);
}

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
        "Operation",
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
                .find(|r| r.scale == *label && r.operation == *op)
                .map(|r| r.val.as_str())
                .unwrap_or("-");
            print!("{:>18} ", val);
        }
        println!("║");
    }

    println!("╚{}╝", "═".repeat(14 + labels.len() * 20));
}
