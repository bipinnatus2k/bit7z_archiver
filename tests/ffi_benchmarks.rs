mod common;

use bit7z_archiver::adapters::bit7z::{self, WriterCompressionLevel, WriterFormat};
use std::time::{Duration, Instant};

fn has_7z_library() -> bool {
    bit7z_archiver::adapters::platform::find_7z_library().is_some()
}

fn open_lib() -> Option<bit7z::Library> {
    let lib_path = bit7z_archiver::adapters::platform::find_7z_library()?;
    bit7z::Library::open(lib_path.to_str()?).ok()
}

struct BenchResult {
    name: &'static str,
    iterations: u32,
    min: Duration,
    max: Duration,
    avg: Duration,
    total: Duration,
}

fn bench<F>(name: &'static str, iterations: u32, mut f: F) -> BenchResult
where
    F: FnMut(),
{
    let mut min = Duration::MAX;
    let mut max = Duration::ZERO;
    let mut total = Duration::ZERO;

    for _ in 0..iterations {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        if elapsed < min {
            min = elapsed;
        }
        if elapsed > max {
            max = elapsed;
        }
        total += elapsed;
    }

    BenchResult { name, iterations, min, max, avg: total / iterations, total }
}

fn fmt_dur(d: Duration) -> String {
    if d.as_secs() > 0 {
        format!("{}.{:03}s", d.as_secs(), d.subsec_millis())
    } else if d.as_micros() >= 1000 {
        format!("{}.{:03}ms", d.as_micros() / 1000, d.as_micros() % 1000)
    } else if d.as_nanos() >= 1000 {
        format!("{}µs", d.as_nanos() / 1000)
    } else {
        format!("{}ns", d.as_nanos())
    }
}

fn print_report(results: &[BenchResult]) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                      FFI Operation Benchmarks                          ║");
    println!("╠══════════════════════════════╦════════╦═══════════╦═════════════════════╣");
    println!("║ Operation                    ║ Iter   ║ Avg       ║ Min / Max           ║");
    println!("╠══════════════════════════════╬════════╬═══════════╬═════════════════════╣");
    for r in results {
        println!(
            "║ {:<28} ║ {:>6} ║ {:>9} ║ {:>9} / {:<9} ║",
            r.name,
            r.iterations,
            fmt_dur(r.avg),
            fmt_dur(r.min),
            fmt_dur(r.max),
        );
    }
    println!("╚══════════════════════════════╩════════╩═══════════╩═════════════════════╝");
    println!();
}

fn group(title: &str) {
    println!();
    println!("─── {} ───", title);
}

fn unique_path(dir: &std::path::Path, ext: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    dir.join(format!("bench_{}.{}", nanos, ext))
}

#[test]
fn safe_api_benchmarks() {
    if !has_7z_library() {
        eprintln!("SKIP: 7z.dll not found");
        return;
    }

    let basic_path = common::fixture_path("basic.7z");
    let multi_path = common::fixture_path("multi_file.7z");
    if !basic_path.exists() || !multi_path.exists() {
        eprintln!("SKIP: fixture files not found");
        return;
    }
    let basic_str = basic_path.to_str().unwrap();
    let multi_str = multi_path.to_str().unwrap();
    let temp_dir = common::temp_dir();
    let mut all_results: Vec<BenchResult> = Vec::new();

    // ───────────────────────────────────────────────────────────
    // Library
    // ───────────────────────────────────────────────────────────
    group("Library");

    all_results.push(bench("Library::open", 50, || {
        let lib_path = bit7z_archiver::adapters::platform::find_7z_library().unwrap();
        let _ = bit7z::Library::open(lib_path.to_str().unwrap()).ok();
    }));

    // ───────────────────────────────────────────────────────────
    // Reader open / close
    // ───────────────────────────────────────────────────────────
    group("Reader open / close");

    all_results.push(bench("open basic.7z", 50, || {
        let lib = open_lib().unwrap();
        let _ = bit7z::ArchiveReader::open(&lib, basic_str, None).ok();
    }));

    all_results.push(bench("open multi_file.7z", 50, || {
        let lib = open_lib().unwrap();
        let _ = bit7z::ArchiveReader::open(&lib, multi_str, None).ok();
    }));

    // ───────────────────────────────────────────────────────────
    // Item count
    // ───────────────────────────────────────────────────────────
    group("Item count");
    let lib = open_lib().unwrap();
    let reader = bit7z::ArchiveReader::open(&lib, basic_str, None).unwrap();

    all_results.push(bench("item_count", 500, || {
        let _ = reader.item_count();
    }));

    // ───────────────────────────────────────────────────────────
    // Item accessors (via safe Item wrapper)
    // ───────────────────────────────────────────────────────────
    group("Item accessors (4 items each iteration)");

    all_results.push(bench("Item::path", 200, || {
        for i in 0..4 { let _ = reader.item(i).path(); }
    }));

    all_results.push(bench("Item::name", 200, || {
        for i in 0..4 { let _ = reader.item(i).name(); }
    }));

    all_results.push(bench("Item::size", 200, || {
        for i in 0..4 { let _ = reader.item(i).size(); }
    }));

    all_results.push(bench("Item::packed_size", 200, || {
        for i in 0..4 { let _ = reader.item(i).packed_size(); }
    }));

    all_results.push(bench("Item::is_directory", 200, || {
        for i in 0..4 { let _ = reader.item(i).is_directory(); }
    }));

    all_results.push(bench("Item::is_encrypted", 200, || {
        for i in 0..4 { let _ = reader.item(i).is_encrypted(); }
    }));

    // Property accessors via BitArchiveItem* (Item methods)
    group("Item property accessors (Item methods)");

    let item0 = reader.item(0);
    all_results.push(bench("Item::mtime", 200, || {
        let _ = item0.mtime();
    }));
    all_results.push(bench("Item::ctime", 200, || {
        let _ = item0.ctime();
    }));
    all_results.push(bench("Item::atime", 200, || {
        let _ = item0.atime();
    }));
    all_results.push(bench("Item::attributes", 200, || {
        let _ = item0.attributes();
    }));
    all_results.push(bench("Item::host_os", 200, || {
        let _ = item0.host_os();
    }));
    all_results.push(bench("Item::is_symlink", 200, || {
        let _ = item0.is_symlink();
    }));
    all_results.push(bench("Item::posix_attrib", 200, || {
        let _ = item0.posix_attrib();
    }));
    all_results.push(bench("Item::compression_method", 200, || {
        let _ = item0.compression_method();
    }));
    all_results.push(bench("Item::comment", 200, || {
        let _ = item0.comment();
    }));
    all_results.push(bench("Item::extension", 200, || {
        let _ = item0.extension();
    }));

    // ───────────────────────────────────────────────────────────
    // Extraction (via safe API)
    // ───────────────────────────────────────────────────────────
    group("Extraction");

    all_results.push(bench("extract_to (4 items)", 10, || {
        let dest = common::temp_dir();
        let _ = reader.extract_to(&[0, 1, 2, 3], dest.to_str().unwrap());
    }));

    all_results.push(bench("extract_to_buffer (1 item)", 30, || {
        let _ = reader.extract_to_buffer(0);
    }));

    // ───────────────────────────────────────────────────────────
    // Integrity test (via safe API)
    // ───────────────────────────────────────────────────────────
    group("Integrity test");

    all_results.push(bench("reader.test()", 30, || {
        let _ = reader.test();
    }));

    // ───────────────────────────────────────────────────────────
    // Encryption detection (safe API)
    // ───────────────────────────────────────────────────────────
    group("Encryption detection");

    let encrypted_path = common::fixture_path("encrypted.7z");
    let encrypted_str = encrypted_path.to_str().unwrap();

    all_results.push(bench("lib.is_header_encrypted (plain)", 100, || {
        let _ = lib.is_header_encrypted(basic_str);
    }));

    all_results.push(bench("lib.is_header_encrypted (enc)", 100, || {
        let _ = lib.is_header_encrypted(encrypted_str);
    }));

    all_results.push(bench("lib.is_encrypted (plain)", 100, || {
        let _ = lib.is_encrypted(basic_str);
    }));

    all_results.push(bench("reader.has_encrypted_items", 100, || {
        let _ = reader.has_encrypted_items();
    }));

    drop(reader);

    // ───────────────────────────────────────────────────────────
    // Writer (safe API)
    // ───────────────────────────────────────────────────────────
    group("Writer (safe API)");

    let bench_file = temp_dir.join("bench_src.txt");
    std::fs::write(&bench_file, b"hello world").unwrap();
    let bench_file_str = bench_file.to_str().unwrap();

    all_results.push(bench("Writer::create", 50, || {
        let _ = bit7z::Writer::create(&lib, WriterFormat::SevenZip).ok();
    }));

    all_results.push(bench("Writer::set_threads(4)", 200, || {
        let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
        w.set_threads(4);
    }));

    all_results.push(bench("Writer::set_compression_level", 200, || {
        let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
        w.set_compression_level(WriterCompressionLevel::Ultra);
    }));

    all_results.push(bench("Writer::set_password", 200, || {
        let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
        w.set_password("test123");
    }));

    all_results.push(bench("Writer::add_file (no compress)", 50, || {
        let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
        let _ = w.add_file(bench_file_str);
    }));

    all_results.push(bench("Writer::add_file + compress", 10, || {
        let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
        w.set_compression_level(WriterCompressionLevel::Fastest);
        let _ = w.add_file(bench_file_str);
        let out = unique_path(&temp_dir, "7z");
        let _ = w.compress_to(out.to_str().unwrap());
    }));

    all_results.push(bench("Writer::add_directory", 30, || {
        let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
        let _ = w.add_directory(temp_dir.to_str().unwrap());
    }));

    // ───────────────────────────────────────────────────────────
    // Editor (safe API)
    // ───────────────────────────────────────────────────────────
    group("Editor (safe API)");

    let editor_archive = temp_dir.join("editor_bench.7z");
    let editor_archive_str = editor_archive.to_str().unwrap();
    {
        let w = bit7z::Writer::create(&lib, WriterFormat::SevenZip).unwrap();
        w.add_file(bench_file_str).unwrap();
        w.compress_to(editor_archive_str).unwrap();
    }

    all_results.push(bench("Editor::open", 30, || {
        let _ = bit7z::Editor::open(&lib, editor_archive_str, WriterFormat::SevenZip, None).ok();
    }));

    let editor = bit7z::Editor::open(&lib, editor_archive_str, WriterFormat::SevenZip, None).unwrap();

    all_results.push(bench("Editor::rename", 30, || {
        let _ = editor.rename(0, "renamed.txt");
    }));

    all_results.push(bench("Editor::delete", 30, || {
        let _ = editor.delete(0);
    }));

    all_results.push(bench("Editor::apply", 10, || {
        let _ = editor.apply();
    }));

    drop(editor);
    drop(lib);

    // ───────────────────────────────────────────────────────────
    // Report
    // ───────────────────────────────────────────────────────────
    print_report(&all_results);

    println!("NOTE: extract_to, compress_to, and add_directory include real I/O;");
    println!("      other benchmarks measure FFI + safe wrapper overhead only.");
}
