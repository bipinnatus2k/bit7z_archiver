# Large Archive Performance Benchmark

## Goal
Add automated benchmarks that measure how the bit7z FFI + safe API scale with archive size (entry count + directory depth), using cross-validation between 7z CLI and safe API to ensure correctness.

## Approach (A3)
Two directional cross-validation:

1. **CLI creates → safe API reads** (primary performance measurement)
2. **Safe API writes → CLI verifies** (correctness + reciprocal perf)

## Scale Matrix

| Entry count (N) | Directory depth | Structure | Cached fixture |
|---|---|---|---|
| 100 | 6 | fanout ~2 | `large_100.7z` |
| 500 | 8 | fanout ~2 | `large_500.7z` |
| 2,000 | 10 | fanout ~2 | `large_2000.7z` |
| 10,000 | 12 | fanout ~3 | `large_10000.7z` |
| 50,000 | 12 | fanout ~4 | `large_50000.7z` (optional skip) |

## Generate Tree Algorithm

```
create_tree(base, total_entries, max_depth, fanout):
    // Distribute entries across depth levels
    // Non-leaf dirs: `fanout` subdirectories + some files
    // Leaf dirs: remaining files
    // Each file = 1 byte content to minimize I/O
```

## Measurements

### Direction 1: CLI → safe API
- `open(N)` latency
- `item_count()` + enumerate all item path/name/size
- `list_directory("")` root listing
- `list_directory("d0/d1/.../")` deepest dir listing
- `extract_to_buffer(0)` single file
- `close(N)` latency

### Direction 2: Safe API → CLI
- `Writer::create` + `add_directory` + `compress_to` total time
- archive size on disk
- CLI `t` test time + result
- CLI `l` entry count match

## Report

Table per direction with columns N=100/500/2000/10000/50000, rows per operation,
final row estimating O(n) class and projected time for N=1,000,000.

## Files

- `tests/large_archive_bench.rs` — all benchmark code
- `tests/common/mod.rs` — add `find_7z_cli()`, `run_7z_command()`
- `tests/fixtures/large_*.7z` — cached archives (git-ignored if too large)

## Test Execution

```shell
cargo test --test large_archive_bench -- --nocapture
```

Skips gracefully if 7z.dll or 7z.exe not found. Cached fixtures are reused.
