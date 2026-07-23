# Format Detection & Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a safe Rust pre-flight detection + validation layer before handing files to the unsafe bit7z C++ FFI.

**Architecture:** Four new components: expanded `ArchiveFormat` enum (all bit7z physical formats + logical aliases), `AutoFormat` (magic + extension detection via `file_format`), `ValidatorRegistry` (plug-in structural validators), `Detection` result passed through `ArchiveService`. No changes to the runtime/executor layer.

**Tech Stack:** `file_format` crate for magic detection, `nom` crate for Zip structural validation, pure Rust, no unsafe.

## Global Constraints

- All new formats in `ArchiveFormat` must have a `serde` rename tag
- `TarGz`/`TarBz2`/`TarXz` must keep their existing serde names for backward compatibility
- `file_format` crate must be added to `bit7z-app-archive` dependencies, NOT `bit7z-domain`
- `nom` crate must be added to `bit7z-app-archive` dependencies only
- Every `unsafe` block requires a `// SAFETY:` comment
- Format variants use `CamelCase` matching the bit7z SDK convention

---

### Task 1: Expand ArchiveFormat Enum

**Files:**
- Modify: `crates/domain/src/archive.rs:111-181`

**Interfaces:**
- Consumes: existing `ArchiveFormat` enum with 7 variants
- Produces: `ArchiveFormat` with 30+ variants, new methods `physical()`, `inner_format()`, `extensions()`, `sdk_value()`, updated `extension()`/`display_name()`/`supports_*`/`is_writable()`/`all_support_compress()`

- [ ] **Step 1: Add new format variants to the enum**

Add these variants after `Rar`:

```rust
#[serde(rename = "rar5")]
Rar5,
#[serde(rename = "gz")]
GZip,
#[serde(rename = "bz2")]
BZip2,
#[serde(rename = "xz")]
Xz,
#[serde(rename = "wim")]
Wim,
#[serde(rename = "arj")]
Arj,
#[serde(rename = "cab")]
Cab,
#[serde(rename = "lzh")]
Lzh,
#[serde(rename = "lzma")]
Lzma,
#[serde(rename = "lzma86")]
Lzma86,
#[serde(rename = "zstd")]
Zstd,
#[serde(rename = "ppmd")]
Ppmd,
#[serde(rename = "nsis")]
Nsis,
#[serde(rename = "iso")]
Iso,
#[serde(rename = "chm")]
Chm,
#[serde(rename = "cpio")]
Cpio,
#[serde(rename = "deb")]
Deb,
#[serde(rename = "rpm")]
Rpm,
#[serde(rename = "dmg")]
Dmg,
#[serde(rename = "vhd")]
Vhd,
#[serde(rename = "vmdk")]
Vmdk,
#[serde(rename = "vdi")]
Vdi,
#[serde(rename = "squashfs")]
SquashFS,
#[serde(rename = "compound")]
Compound,
#[serde(rename = "other")]
Other(u32),
```

- [ ] **Step 2: Add new methods to `impl ArchiveFormat`**

```rust
pub fn physical(&self) -> ArchiveFormat {
    match self {
        ArchiveFormat::TarGz => ArchiveFormat::GZip,
        ArchiveFormat::TarBz2 => ArchiveFormat::BZip2,
        ArchiveFormat::TarXz => ArchiveFormat::Xz,
        other => *other,
    }
}

pub fn inner_format(&self) -> Option<ArchiveFormat> {
    match self {
        ArchiveFormat::TarGz | ArchiveFormat::GZip => Some(ArchiveFormat::Tar),
        ArchiveFormat::TarBz2 | ArchiveFormat::BZip2 => Some(ArchiveFormat::Tar),
        ArchiveFormat::TarXz | ArchiveFormat::Xz => Some(ArchiveFormat::Tar),
        _ => None,
    }
}

pub fn extensions(&self) -> &[&str] {
    match self {
        ArchiveFormat::SevenZip => &["7z"],
        ArchiveFormat::Zip => &["zip"],
        ArchiveFormat::Tar => &["tar"],
        ArchiveFormat::TarGz => &["tar.gz", "tgz"],
        ArchiveFormat::TarXz => &["tar.xz", "txz"],
        ArchiveFormat::TarBz2 => &["tar.bz2", "tbz2", "tbz"],
        ArchiveFormat::Rar => &["rar"],
        ArchiveFormat::Rar5 => &["rar"],
        ArchiveFormat::GZip => &["gz", "tgz", "tar.gz"],
        ArchiveFormat::BZip2 => &["bz2", "tbz2", "tbz", "tar.bz2"],
        ArchiveFormat::Xz => &["xz", "txz", "tar.xz"],
        ArchiveFormat::Wim => &["wim"],
        ArchiveFormat::Arj => &["arj"],
        ArchiveFormat::Cab => &["cab"],
        ArchiveFormat::Lzh => &["lzh", "lha"],
        ArchiveFormat::Lzma => &["lzma"],
        ArchiveFormat::Lzma86 => &["lzma86"],
        ArchiveFormat::Zstd => &["zst", "zstd"],
        ArchiveFormat::Ppmd => &["ppmd"],
        ArchiveFormat::Nsis => &["exe"],
        ArchiveFormat::Iso => &["iso"],
        ArchiveFormat::Chm => &["chm"],
        ArchiveFormat::Cpio => &["cpio"],
        ArchiveFormat::Deb => &["deb"],
        ArchiveFormat::Rpm => &["rpm"],
        ArchiveFormat::Dmg => &["dmg"],
        ArchiveFormat::Vhd => &["vhd", "vhdx"],
        ArchiveFormat::Vmdk => &["vmdk"],
        ArchiveFormat::Vdi => &["vdi"],
        ArchiveFormat::SquashFS => &["squashfs"],
        ArchiveFormat::Compound => &["compound"],
        ArchiveFormat::Other(_) => &[],
    }
}

pub fn sdk_value(&self) -> Option<u32> {
    match self {
        ArchiveFormat::Other(v) => Some(*v),
        _ => None,
    }
}
```

- [ ] **Step 3: Update existing methods**

`extension()` — keep the existing compound variants returning the same strings, add new formats:

```rust
pub fn extension(&self) -> &str {
    match self {
        ArchiveFormat::SevenZip => "7z",
        ArchiveFormat::Zip => "zip",
        ArchiveFormat::Tar => "tar",
        ArchiveFormat::TarGz => "tar.gz",
        ArchiveFormat::TarXz => "tar.xz",
        ArchiveFormat::TarBz2 => "tar.bz2",
        ArchiveFormat::Rar | ArchiveFormat::Rar5 => "rar",
        ArchiveFormat::GZip => "gz",
        ArchiveFormat::BZip2 => "bz2",
        ArchiveFormat::Xz => "xz",
        ArchiveFormat::Wim => "wim",
        ArchiveFormat::Arj => "arj",
        ArchiveFormat::Cab => "cab",
        ArchiveFormat::Lzh => "lzh",
        ArchiveFormat::Lzma => "lzma",
        ArchiveFormat::Lzma86 => "lzma86",
        ArchiveFormat::Zstd => "zst",
        ArchiveFormat::Ppmd => "ppmd",
        ArchiveFormat::Nsis => "exe",
        ArchiveFormat::Iso => "iso",
        ArchiveFormat::Chm => "chm",
        ArchiveFormat::Cpio => "cpio",
        ArchiveFormat::Deb => "deb",
        ArchiveFormat::Rpm => "rpm",
        ArchiveFormat::Dmg => "dmg",
        ArchiveFormat::Vhd => "vhd",
        ArchiveFormat::Vmdk => "vmdk",
        ArchiveFormat::Vdi => "vdi",
        ArchiveFormat::SquashFS => "squashfs",
        ArchiveFormat::Compound => "compound",
        ArchiveFormat::Other(v) => return &v.to_string(),
    }
}
```

`supports_encryption()` — add new writable formats; most read-only formats return false:

```rust
pub fn supports_encryption(&self) -> bool {
    matches!(self, ArchiveFormat::SevenZip | ArchiveFormat::Zip)
}
```

`supports_encrypted_filenames()`:

```rust
pub fn supports_encrypted_filenames(&self) -> bool {
    matches!(self, ArchiveFormat::SevenZip)
}
```

`is_writable()` — align with bit7z's `BitInOutFormat` types:

```rust
pub fn is_writable(&self) -> bool {
    matches!(self,
        ArchiveFormat::SevenZip
        | ArchiveFormat::Zip
        | ArchiveFormat::Tar
        | ArchiveFormat::GZip
        | ArchiveFormat::BZip2
        | ArchiveFormat::Xz
        | ArchiveFormat::Wim
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarBz2
        | ArchiveFormat::TarXz
    )
}
```

`supports_compression_level()`:

```rust
pub fn supports_compression_level(&self) -> bool {
    !matches!(self, ArchiveFormat::Tar | ArchiveFormat::GZip | ArchiveFormat::BZip2 | ArchiveFormat::Xz)
}
```

`display_name()` — add all new formats:

```rust
pub fn display_name(&self) -> &str {
    match self {
        ArchiveFormat::SevenZip => "7z",
        ArchiveFormat::Zip => "Zip",
        ArchiveFormat::Tar => "Tar",
        ArchiveFormat::TarGz => "Tar.gz",
        ArchiveFormat::TarXz => "Tar.xz",
        ArchiveFormat::TarBz2 => "Tar.bz2",
        ArchiveFormat::Rar => "Rar",
        ArchiveFormat::Rar5 => "Rar5",
        ArchiveFormat::GZip => "GZip",
        ArchiveFormat::BZip2 => "BZip2",
        ArchiveFormat::Xz => "Xz",
        ArchiveFormat::Wim => "WIM",
        ArchiveFormat::Arj => "ARJ",
        ArchiveFormat::Cab => "CAB",
        ArchiveFormat::Lzh => "LZH",
        ArchiveFormat::Lzma => "LZMA",
        ArchiveFormat::Lzma86 => "LZMA86",
        ArchiveFormat::Zstd => "Zstd",
        ArchiveFormat::Ppmd => "PPMd",
        ArchiveFormat::Nsis => "NSIS",
        ArchiveFormat::Iso => "ISO",
        ArchiveFormat::Chm => "CHM",
        ArchiveFormat::Cpio => "CPIO",
        ArchiveFormat::Deb => "DEB",
        ArchiveFormat::Rpm => "RPM",
        ArchiveFormat::Dmg => "DMG",
        ArchiveFormat::Vhd => "VHD",
        ArchiveFormat::Vmdk => "VMDK",
        ArchiveFormat::Vdi => "VDI",
        ArchiveFormat::SquashFS => "SquashFS",
        ArchiveFormat::Compound => "Compound",
        ArchiveFormat::Other(_) => "Other",
    }
}
```

`all_support_compress()` — use writable formats:

```rust
pub fn all_support_compress() -> Vec<ArchiveFormat> {
    vec![
        ArchiveFormat::SevenZip,
        ArchiveFormat::Zip,
        ArchiveFormat::Tar,
        ArchiveFormat::GZip,
        ArchiveFormat::BZip2,
        ArchiveFormat::Xz,
        ArchiveFormat::Wim,
        ArchiveFormat::TarGz,
        ArchiveFormat::TarBz2,
        ArchiveFormat::TarXz,
    ]
}
```

Add `EncryptionMethod::available_for` for GZip and physical formats:

```rust
pub fn available_for(format: ArchiveFormat) -> Vec<Self> {
    match format {
        ArchiveFormat::SevenZip => vec![EncryptionMethod::Aes256],
        ArchiveFormat::Zip => vec![EncryptionMethod::ZipCrypto, EncryptionMethod::Aes256],
        _ => vec![],
    }
}
```

- [ ] **Step 4: Update existing match statements across all files**

These files have match statements on `ArchiveFormat` that now need exhaustive handling (or `_` catch-all for read-only formats):

1. `crates/domain/src/archive.rs:285` — `EncryptionMethod::available_for()` (already updated above with catch-all)
2. `crates/domain/src/vfs/overlay.rs:460` — CRC skip check (already uses `matches!` with catch-all)
3. `crates/testing/src/fixture.rs:40-63` — extension mapping + WriterFormat mapping (add GZip/BZip2/Xz/Wim)
4. `crates/testing/src/cli_referee.rs:138-166` — `detect_format()`
5. `crates/infrastructure/persistence/src/adapters/ffi_util.rs:11-38` — `detect_writer_format()` + `writer_format_to_archive_format()`
6. `crates/infrastructure/persistence/src/adapters/writer.rs:57-69` — `ArchiveFormat → WriterFormat`
7. `crates/application/archive/src/runtime_service.rs:74-82` — capability registration (add GZip/BZip2/Xz/Wim)
8. `crates/application/archive/src/runtime_service.rs:159-171` — `resolve_format()` (will be replaced by AutoFormat in a later task)
9. `crates/runtime/cli/src/lib.rs:555-566` — `parse_format()`
10. `crates/presentation/settings/src/pages.rs:85-88` — format selection
11. `crates/presentation/settings/src/panels.rs:77-88` — format selection
12. `crates/presentation/dialogs/src/create.rs:112-120` — create dialog format list

For files 9-12, add a catch-all `_ =>` or unknown entry. For files 3-8, add the new physical format mappings.

- [ ] **Step 5: Run `cargo test -p bit7z-domain` to verify enum changes compile**

Run: `cargo test -p bit7z-domain 2>&1`
Expected: all tests pass

- [ ] **Step 6: Run full cargo check**

Run: `cargo check 2>&1`
Expected: no errors (warnings about unused variants in existing match arms are OK for now)

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(domain): expand ArchiveFormat with all bit7z physical format variants"
```

---

### Task 2: AutoFormat Detection Layer

**Files:**
- Create: `crates/application/archive/src/auto_format.rs`
- Modify: `crates/application/archive/Cargo.toml` (add `file_format` dep)
- Test: via `cargo test -p bit7z-app-archive`

**Interfaces:**
- Consumes: `ArchiveFormat` from domain, `file_format::FileFormat`
- Produces: `AutoFormat { detect() → Detection }`, `Detection { format, logical, inner, extensions, is_multi_volume, total_volumes, validated }`, `DetectionError`

- [ ] **Step 1: Add `file_format` dependency**

In `crates/application/archive/Cargo.toml`, add:

```toml
file_format = "0.29"
```

- [ ] **Step 2: Create Detection and DetectionError types**

In `crates/application/archive/src/auto_format.rs`:

```rust
use std::path::{Path, PathBuf};
use bit7z_domain::archive::ArchiveFormat;

/// Result of detecting and validating an archive file.
#[derive(Debug, Clone)]
pub struct Detection {
    pub format: ArchiveFormat,
    pub logical: Option<ArchiveFormat>,
    pub inner: Option<ArchiveFormat>,
    pub extensions: Vec<String>,
    pub is_multi_volume: bool,
    pub total_volumes: Option<u32>,
    pub validated: bool,
}

/// Errors from the detection layer.
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown or unsupported archive format: {path}")]
    UnknownFormat { path: PathBuf },
    #[error(
        "format mismatch: magic bytes indicate {magic:?} \
         but extension suggests {extension:?}"
    )]
    FormatMismatch {
        magic: ArchiveFormat,
        extension: ArchiveFormat,
    },
    #[error("validation failed for {format:?}: {detail}")]
    ValidationFailed {
        format: ArchiveFormat,
        detail: String,
    },
}
```

- [ ] **Step 3: Create `file_format → ArchiveFormat` mapper**

```rust
fn file_format_to_archive_format(ff: file_format::FileFormat) -> Option<ArchiveFormat> {
    use file_format::FileFormat;
    match ff {
        FileFormat::SevenZip => Some(ArchiveFormat::SevenZip),
        FileFormat::Zip => Some(ArchiveFormat::Zip),
        FileFormat::Tar => Some(ArchiveFormat::Tar),
        FileFormat::GZip => Some(ArchiveFormat::GZip),
        FileFormat::BZip2 => Some(ArchiveFormat::BZip2),
        FileFormat::Xz => Some(ArchiveFormat::Xz),
        FileFormat::Wim => Some(ArchiveFormat::Wim),
        FileFormat::Rar => Some(ArchiveFormat::Rar),
        FileFormat::Rar5 => Some(ArchiveFormat::Rar5),
        FileFormat::Arj => Some(ArchiveFormat::Arj),
        FileFormat::Cab => Some(ArchiveFormat::Cab),
        FileFormat::Lzh => Some(ArchiveFormat::Lzh),
        FileFormat::Lzma => Some(ArchiveFormat::Lzma),
        FileFormat::Iso => Some(ArchiveFormat::Iso),
        FileFormat::Chm => Some(ArchiveFormat::Chm),
        FileFormat::Cpio => Some(ArchiveFormat::Cpio),
        FileFormat::Deb => Some(ArchiveFormat::Deb),
        FileFormat::Rpm => Some(ArchiveFormat::Rpm),
        FileFormat::Dmg => Some(ArchiveFormat::Dmg),
        FileFormat::Vhd => Some(ArchiveFormat::Vhd),
        FileFormat::Squashfs => Some(ArchiveFormat::SquashFS),
        _ => None,
    }
}
```

- [ ] **Step 4: Create AutoFormat struct + detect()**

```rust
use std::path::Path;
use std::sync::Arc;
use crate::validator::ValidatorRegistry;

pub struct AutoFormat {
    validators: Option<Arc<ValidatorRegistry>>,
}

impl AutoFormat {
    pub fn new() -> Self {
        Self { validators: None }
    }

    pub fn with_validators(validators: Arc<ValidatorRegistry>) -> Self {
        Self { validators: Some(validators) }
    }

    pub fn detect(&self, path: &Path) -> Result<Detection, DetectionError> {
        let header = std::fs::read(path)?;
        let data = &header[..header.len().min(1024)];

        let ff = file_format::FileFormat::from_bytes(data);
        let physical = file_format_to_archive_format(ff)
            .ok_or_else(|| DetectionError::UnknownFormat { path: path.to_path_buf() })?;

        let ext_str = path.to_string_lossy().to_lowercase();
        let extensions = parse_extensions(&ext_str);

        let logical = resolve_logical_format(&extensions);
        let inner = logical.and_then(|l| l.inner_format())
            .or_else(|| physical.inner_format());

        let is_multi_volume = extensions.iter().any(|e| {
            e.ends_with(".001") || e.ends_with(".002") || e.starts_with('r') && e.len() == 3
        });

        if let Some(logical) = logical {
            if physical != logical.physical() {
                return Err(DetectionError::FormatMismatch {
                    magic: physical,
                    extension: logical,
                });
            }
        }

        let validated = if let Some(registry) = &self.validators {
            registry.validate(physical, path, data).is_ok()
        } else {
            false
        };

        Ok(Detection {
            format: physical,
            logical,
            inner,
            extensions,
            is_multi_volume,
            total_volumes: None,
            validated,
        })
    }
}

impl Default for AutoFormat {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_extensions(path: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = path.to_string();
    loop {
        if let Some(idx) = current.rfind('.') {
            let ext = current[idx..].to_string();
            parts.push(ext.trim_start_matches('.').to_string());
            current = current[..idx].to_string();
        } else {
            break;
        }
    }
    parts
}

fn resolve_logical_format(extensions: &[String]) -> Option<ArchiveFormat> {
    let joined = extensions.join(".");
    // Compound format resolution checks longest match first
    let known: &[(&str, ArchiveFormat)] = &[
        ("tar.gz", ArchiveFormat::TarGz),
        ("tar.xz", ArchiveFormat::TarXz),
        ("tar.bz2", ArchiveFormat::TarBz2),
        ("tgz", ArchiveFormat::TarGz),
        ("txz", ArchiveFormat::TarXz),
        ("tbz2", ArchiveFormat::TarBz2),
        ("tbz", ArchiveFormat::TarBz2),
    ];
    for (pat, fmt) in known {
        if joined == *pat || joined.ends_with(pat) {
            return Some(*fmt);
        }
    }
    // Otherwise try single extension match
    if let Some(last) = extensions.last() {
        for fmt in crate::archive::ALL_FORMATS {
            if fmt.extensions().contains(&last.as_str()) {
                return Some(*fmt);
            }
        }
    }
    None
}
```

Note: `ALL_FORMATS` needs to be defined. Add a constant to `crates/domain/src/archive.rs`:

```rust
pub const ALL_FORMATS: &[ArchiveFormat] = &[
    ArchiveFormat::SevenZip,
    ArchiveFormat::Zip,
    ArchiveFormat::Tar,
    ArchiveFormat::TarGz,
    ArchiveFormat::TarBz2,
    ArchiveFormat::TarXz,
    ArchiveFormat::GZip,
    ArchiveFormat::BZip2,
    ArchiveFormat::Xz,
    ArchiveFormat::Wim,
    ArchiveFormat::Rar,
    ArchiveFormat::Arj,
    ArchiveFormat::Cab,
    ArchiveFormat::Lzh,
    ArchiveFormat::Lzma,
    ArchiveFormat::Iso,
    ArchiveFormat::Chm,
    ArchiveFormat::Cpio,
    ArchiveFormat::Deb,
    ArchiveFormat::Rpm,
    ArchiveFormat::Dmg,
    ArchiveFormat::Vhd,
    ArchiveFormat::Vmdk,
    ArchiveFormat::Vdi,
    ArchiveFormat::SquashFS,
];
```

- [ ] **Step 5: Write detection tests**

In `crates/application/archive/tests/auto_format_test.rs`:

```rust
use bit7z_app_archive::auto_format::AutoFormat;
use std::path::Path;

#[test]
fn test_detect_zip() {
    // Create a minimal valid Zip file in temp, then test detection
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("test.zip");
    let data = [0x50, 0x4B, 0x03, 0x04, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00];
    std::fs::write(&zip_path, &data).unwrap();
    let detector = AutoFormat::new();
    let result = detector.detect(&zip_path).unwrap();
    assert_eq!(result.format, ArchiveFormat::Zip);
}

#[test]
fn test_detect_7z() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.7z");
    let data = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];
    std::fs::write(&path, &data).unwrap();
    let detector = AutoFormat::new();
    let result = detector.detect(&path).unwrap();
    assert_eq!(result.format, ArchiveFormat::SevenZip);
}

#[test]
fn test_detect_gzip_tar() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("archive.tar.gz");
    let data = [0x1F, 0x8B];
    std::fs::write(&path, &data).unwrap();
    let detector = AutoFormat::new();
    let result = detector.detect(&path).unwrap();
    assert_eq!(result.format, ArchiveFormat::GZip);
    assert_eq!(result.logical, Some(ArchiveFormat::TarGz));
    assert_eq!(result.inner, Some(ArchiveFormat::Tar));
}

#[test]
fn test_format_mismatch_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("document.zip");
    let data = [0x1F, 0x8B]; // GZip magic, but .zip extension
    std::fs::write(&path, &data).unwrap();
    let detector = AutoFormat::new();
    assert!(detector.detect(&path).is_err());
}
```

Also add the tempfile dev-dependency to `crates/application/archive/Cargo.toml` if not already present.

- [ ] **Step 6: Run tests**

Run: `cargo test -p bit7z-app-archive 2>&1`
Expected: all tests pass (including new detection tests)

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(application): add AutoFormat magic + extension detection layer"
```

---

### Task 3: Validator Infrastructure

**Files:**
- Create: `crates/application/archive/src/validator.rs`
- Modify: `crates/application/archive/src/lib.rs` (add `pub mod validator;`)

**Interfaces:**
- Consumes: `ArchiveFormat`
- Produces: `FormatValidator` trait, `ValidatorRegistry`

- [ ] **Step 1: Create trait and registry**

In `crates/application/archive/src/validator.rs`:

```rust
use std::collections::HashMap;
use std::path::Path;
use bit7z_domain::archive::ArchiveFormat;

/// Pure-Rust structural validator for a single archive format.
#[async_trait::async_trait]
pub trait FormatValidator: Send + Sync {
    fn format(&self) -> ArchiveFormat;
    async fn validate(&self, path: &Path, data: &[u8]) -> Result<(), Vec<String>>;
}

/// Registry of registered format validators.
pub struct ValidatorRegistry {
    validators: HashMap<ArchiveFormat, Box<dyn FormatValidator>>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }

    pub fn register(&mut self, v: Box<dyn FormatValidator>) {
        self.validators.insert(v.format(), v);
    }

    pub fn get(&self, fmt: ArchiveFormat) -> Option<&dyn FormatValidator> {
        self.validators.get(&fmt).map(|b| b.as_ref())
    }

    pub fn validate(
        &self,
        fmt: ArchiveFormat,
        path: &Path,
        data: &[u8],
    ) -> Result<(), Vec<String>> {
        match self.validators.get(&fmt) {
            Some(v) => {
                // async in sync context — validators should be lightweight
                // In practice validate() does no I/O (data is already read)
                futures::executor::block_on(v.validate(path, data))
            }
            None => Ok(()),
        }
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Add `async-trait` and `futures` to Cargo.toml**

In `crates/application/archive/Cargo.toml`:

```toml
async-trait = "0.1"
futures = "0.3"
```

Or if they're already present, just verify.

- [ ] **Step 3: Register the module**

In `crates/application/archive/src/lib.rs`, add:

```rust
pub mod auto_format;
pub mod validator;
```

- [ ] **Step 4: Write validator tests**

In the same file, add `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use bit7z_domain::archive::ArchiveFormat;

    struct MockZipValidator;

    #[async_trait::async_trait]
    impl FormatValidator for MockZipValidator {
        fn format(&self) -> ArchiveFormat {
            ArchiveFormat::Zip
        }
        async fn validate(&self, _path: &Path, data: &[u8]) -> Result<(), Vec<String>> {
            if data.len() < 4 {
                return Err(vec!["too short".to_string()]);
            }
            Ok(())
        }
    }

    #[test]
    fn test_registry_validate_passes() {
        let mut registry = ValidatorRegistry::new();
        registry.register(Box::new(MockZipValidator));
        assert!(registry.validate(ArchiveFormat::Zip, Path::new(""), &[0x50, 0x4B, 0x03, 0x04]).is_ok());
    }

    #[test]
    fn test_registry_validate_fails() {
        let mut registry = ValidatorRegistry::new();
        registry.register(Box::new(MockZipValidator));
        assert!(registry.validate(ArchiveFormat::Zip, Path::new(""), &[0x00]).is_err());
    }

    #[test]
    fn test_registry_no_validator_is_ok() {
        let registry = ValidatorRegistry::new();
        assert!(registry.validate(ArchiveFormat::SevenZip, Path::new(""), &[]).is_ok());
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p bit7z-app-archive 2>&1`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(application): add FormatValidator trait and ValidatorRegistry"
```

---

### Task 4: Zip Structural Validator

**Files:**
- Create: `crates/application/archive/src/validators/mod.rs`
- Create: `crates/application/archive/src/validators/zip.rs`
- Modify: `crates/application/archive/Cargo.toml` (add `nom` dep)

**Interfaces:**
- Consumes: `FormatValidator` trait
- Produces: `ZipValidator` implementing `FormatValidator`

- [ ] **Step 1: Add `nom` dependency**

In `crates/application/archive/Cargo.toml`:

```toml
nom = "7"
```

- [ ] **Step 2: Create validators module**

In `crates/application/archive/src/validators/mod.rs`:

```rust
pub mod zip;
```

- [ ] **Step 3: Create ZipValidator**

The Zip validator validates:
1. EOCD signature exists at expected offset
2. Central Directory entries have valid signatures
3. Local File Headers have valid signatures
4. No path traversal in filenames

```rust
use std::path::Path;
use bit7z_domain::archive::ArchiveFormat;
use crate::validator::FormatValidator;

pub struct ZipValidator;

#[async_trait::async_trait]
impl FormatValidator for ZipValidator {
    fn format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }

    async fn validate(&self, _path: &Path, data: &[u8]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if data.len() < 22 {
            errors.push("file too small for a Zip archive".to_string());
            return Err(errors);
        }

        // Search for EOCD signature (0x06054b50) from end of file
        let eocd_pos = data
            .windows(4)
            .enumerate()
            .rev()
            .find(|(_, w)| w == [0x50, 0x4B, 0x05, 0x06])
            .map(|(i, _)| i);

        let eocd_pos = match eocd_pos {
            Some(p) => p,
            None => {
                errors.push("EOCD signature not found".to_string());
                return Err(errors);
            }
        };

        // Parse EOCD fields
        if eocd_pos + 22 > data.len() {
            errors.push("EOCD extends beyond file".to_string());
            return Err(errors);
        }

        let cd_offset = u32::from_le_bytes(
            data[eocd_pos + 16..eocd_pos + 20].try_into().unwrap(),
        ) as usize;
        let cd_entries = u16::from_le_bytes(
            data[eocd_pos + 10..eocd_pos + 12].try_into().unwrap(),
        ) as usize;
        let cd_size = u32::from_le_bytes(
            data[eocd_pos + 12..eocd_pos + 16].try_into().unwrap(),
        ) as usize;

        // Validate Central Directory bounds
        if cd_offset + cd_size > data.len() {
            errors.push("Central Directory extends beyond file".to_string());
        }

        // Iterate central directory entries
        let mut pos = cd_offset;
        for _ in 0..cd_entries {
            if pos + 46 > data.len() {
                errors.push("CD entry truncated".to_string());
                break;
            }
            if data[pos..pos + 4] != [0x50, 0x4B, 0x01, 0x02] {
                errors.push("invalid CD entry signature".to_string());
                break;
            }

            let name_len = u16::from_le_bytes(
                data[pos + 28..pos + 30].try_into().unwrap(),
            ) as usize;
            let extra_len = u16::from_le_bytes(
                data[pos + 30..pos + 32].try_into().unwrap(),
            ) as usize;
            let comment_len = u16::from_le_bytes(
                data[pos + 32..pos + 34].try_into().unwrap(),
            ) as usize;
            let local_offset = u32::from_le_bytes(
                data[pos + 42..pos + 46].try_into().unwrap(),
            ) as usize;

            // Check filename for path traversal
            if name_len > 0 && pos + 46 + name_len <= data.len() {
                let name = &data[pos + 46..pos + 46 + name_len];
                let name_str = String::from_utf8_lossy(name);
                if name_str.contains("..") {
                    errors.push(format!(
                        "path traversal detected in entry: {}",
                        name_str
                    ));
                }
            }

            // Validate Local File Header
            if local_offset + 30 > data.len() {
                errors.push(format!(
                    "LFH at offset {} extends beyond file",
                    local_offset
                ));
            } else if data[local_offset..local_offset + 4] != [0x50, 0x4B, 0x03, 0x04] {
                errors.push(format!(
                    "invalid LFH signature at offset {}",
                    local_offset
                ));
            } else {
                let lfh_name_len = u16::from_le_bytes(
                    data[local_offset + 26..local_offset + 28].try_into().unwrap(),
                ) as usize;
                let lfh_extra_len = u16::from_le_bytes(
                    data[local_offset + 28..local_offset + 30].try_into().unwrap(),
                ) as usize;
                let lfh_total = 30 + lfh_name_len + lfh_extra_len;
                if local_offset + lfh_total > data.len() {
                    errors.push(format!(
                        "LFH data at offset {} extends beyond file",
                        local_offset
                    ));
                }
            }

            let entry_total = 46 + name_len + extra_len + comment_len;
            pos += entry_total;
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

- [ ] **Step 4: Register module in lib.rs**

Add to `crates/application/archive/src/lib.rs`:

```rust
pub mod validators;
```

- [ ] **Step 5: Write Zip validator tests**

In `crates/application/archive/tests/zip_validator_test.rs` (or as inline `#[cfg(test)]`):

```rust
use bit7z_app_archive::validators::zip::ZipValidator;
use bit7z_app_archive::validator::FormatValidator;

#[tokio::test]
async fn test_zip_validator_empty_data_fails() {
    let v = ZipValidator;
    let result = v.validate(std::path::Path::new(""), &[]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_zip_validator_minimal_header_passes() {
    // A minimal valid Zip with one file entry
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("test.zip");

    // Create a real Zip file using the bit7z writer or a simple construction
    // For now, use a known-good minimal zip byte sequence
    let mut data = Vec::new();

    // Local file header for "hello.txt" containing "Hello"
    data.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]); // LFH sig
    data.extend_from_slice(&[0x0A, 0x00]);              // version needed
    data.extend_from_slice(&[0x00, 0x00]);              // flags
    data.extend_from_slice(&[0x00, 0x00]);              // compression: store
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);  // mtime
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);  // crc32 (placeholder)
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]);     // compressed size
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]);     // uncompressed size
    data.extend_from_slice(&[10, 0x00]);                 // filename length
    data.extend_from_slice(&[0x00, 0x00]);              // extra field length
    data.extend_from_slice(b"hello.txt");                // filename
    data.extend_from_slice(b"Hello");                    // file data

    let cd_offset = data.len() as u32;

    // Central directory entry
    data.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]); // CD sig
    data.extend_from_slice(&[0x0A, 0x00]);              // version made by
    data.extend_from_slice(&[0x0A, 0x00]);              // version needed
    data.extend_from_slice(&[0x00, 0x00]);              // flags
    data.extend_from_slice(&[0x00, 0x00]);              // compression
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);  // mtime
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);  // crc32
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]);     // compressed
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]);     // uncompressed
    data.extend_from_slice(&[10, 0x00]);                 // filename length
    data.extend_from_slice(&[0x00, 0x00]);              // extra
    data.extend_from_slice(&[0x00, 0x00]);              // comment
    data.extend_from_slice(&[0x00, 0x00]);              // disk start
    data.extend_from_slice(&[0x00, 0x00]);              // internal attrs
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);  // external attrs
    data.extend_from_slice(&cd_offset.to_le_bytes());    // local header offset
    data.extend_from_slice(b"hello.txt");                // filename

    let cd_size = (data.len() - cd_offset as usize) as u32;

    // EOCD
    data.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]); // EOCD sig
    data.extend_from_slice(&[0x00, 0x00]);              // disk
    data.extend_from_slice(&[0x00, 0x00]);              // cd disk
    data.extend_from_slice(&[1, 0x00]);                 // entries on disk
    data.extend_from_slice(&[1, 0x00]);                 // total entries
    data.extend_from_slice(&cd_size.to_le_bytes());      // cd size
    data.extend_from_slice(&cd_offset.to_le_bytes());    // cd offset
    data.extend_from_slice(&[0x00, 0x00]);              // comment length

    let v = ZipValidator;
    let result = v.validate(std::path::Path::new(""), &data).await;
    assert!(result.is_ok(), "expected OK, got: {:?}", result);
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p bit7z-app-archive 2>&1`
Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(application): add nom-based Zip structural validator"
```

---

### Task 5: Integration into ArchiveService

**Files:**
- Modify: `crates/application/archive/src/runtime_service.rs`
- Modify: `crates/application/archive/src/lib.rs` (if not already)
- Modify: `crates/infrastructure/persistence/src/adapters/ffi_util.rs`
- Modify: `crates/infrastructure/persistence/src/adapters/writer.rs`
- Modify: `crates/testing/src/fixture.rs`
- Modify: `crates/testing/src/cli_referee.rs`
- Modify: `crates/runtime/cli/src/lib.rs`
- Modify: `crates/presentation/settings/src/pages.rs`
- Modify: `crates/presentation/settings/src/panels.rs`
- Modify: `crates/presentation/dialogs/src/create.rs`

- [ ] **Step 1: Wire AutoFormat into ArchiveService::open()**

In `ArchiveService`, add an `auto_format: AutoFormat` field. In `open()`:

```rust
pub fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
    // 1. Safe pre-flight: magic + extension check
    let detection = self.auto_format.detect(path)
        .map_err(|e| ArchiveError::Internal(e.to_string()))?;

    // 2. Resolve capability using detected physical format
    let descriptor = self.resolver.resolve(CapabilityRequest {
        kind: CapabilityKind::Read,
        format: detection.format,
        constraints: vec![],
        session_id: None,
    }).map_err(|e| ArchiveError::Internal(e.to_string()))?;

    // 3. Submit to runtime (existing logic)
    let handle = self.runtime.submit(OperationRequest {
        kind: OperationKind::OpenArchive {
            path: path.to_path_buf(),
            password: password.cloned(),
        },
        descriptor,
        priority: Priority::User,
    });

    match self.wait(handle)? {
        JobResult::Ok => {
            let session = find_session_by_path(&self.runtime, path)
                .ok_or_else(|| ArchiveError::Internal("session not stored".into()))?;
            let archive_handle = ArchiveHandle::new_reader()
                .with_path(path.to_path_buf());
            self.sessions.lock().unwrap().insert(archive_handle.id, session);
            Ok(archive_handle)
        }
        // ... rest unchanged
    }
}
```

- [ ] **Step 2: Update `build_bit7z_runtime()` to create AutoFormat with validators**

After creating the registry, also create the `ValidatorRegistry` and wire it:

```rust
pub fn build_bit7z_runtime(...) -> (...) {
    // ... existing code ...

    // Register format validators
    let mut validator_registry = ValidatorRegistry::new();
    validator_registry.register(Box::new(validators::zip::ZipValidator));

    // Create the runtime and service
    let runtime = Arc::new(RuntimeBuilder::new().build(ports, context));
    let service = ArchiveService::new(
        runtime.clone(),
        resolver.clone(),
        AutoFormat::with_validators(Arc::new(validator_registry)),
    );

    // ... return ...
}
```

- [ ] **Step 3: Update adapter format mappings**

In `ffi_util.rs`, update `detect_writer_format()` and `writer_format_to_archive_format()`
to use the new physical formats. The old mapping from `WriterFormat::GZip → ArchiveFormat::TarGz`
should be updated: the reader should detect the physical format, and the logical
(TarGz) should come from AutoFormat's extension mapping.

`writer_format_to_archive_format` returns physical format:

```rust
pub fn writer_format_to_archive_format(wf: bit7z::WriterFormat) -> Option<ArchiveFormat> {
    match wf {
        bit7z::WriterFormat::SevenZip => Some(ArchiveFormat::SevenZip),
        bit7z::WriterFormat::Zip => Some(ArchiveFormat::Zip),
        bit7z::WriterFormat::Tar => Some(ArchiveFormat::Tar),
        bit7z::WriterFormat::GZip => Some(ArchiveFormat::GZip),
        bit7z::WriterFormat::BZip2 => Some(ArchiveFormat::BZip2),
        bit7z::WriterFormat::Xz => Some(ArchiveFormat::Xz),
        bit7z::WriterFormat::Wim => Some(ArchiveFormat::Wim),
        _ => None,
    }
}
```

In `writer.rs`, the `ArchiveFormat → WriterFormat` mapping now needs GZip/BZip2/Xz/Wim:

```rust
fn archive_format_to_writer_format(f: ArchiveFormat) -> Result<bit7z::WriterFormat, ArchiveError> {
    match f.physical() {
        ArchiveFormat::SevenZip => Ok(bit7z::WriterFormat::SevenZip),
        ArchiveFormat::Zip => Ok(bit7z::WriterFormat::Zip),
        ArchiveFormat::Tar => Ok(bit7z::WriterFormat::Tar),
        ArchiveFormat::GZip => Ok(bit7z::WriterFormat::GZip),
        ArchiveFormat::BZip2 => Ok(bit7z::WriterFormat::BZip2),
        ArchiveFormat::Xz => Ok(bit7z::WriterFormat::Xz),
        ArchiveFormat::Wim => Ok(bit7z::WriterFormat::Wim),
        _ => Err(ArchiveError::UnsupportedOperation("format not writable".into())),
    }
}
```

- [ ] **Step 4: Update fixture and CLI format detection**

In `fixture.rs`: update the format → extension mapping to use `physical()` for
WriterFormat resolution:

```rust
let writer_format = match format.physical() {
    ArchiveFormat::SevenZip => bit7z_infra_bit7z::WriterFormat::SevenZip,
    ArchiveFormat::Zip => bit7z_infra_bit7z::WriterFormat::Zip,
    ArchiveFormat::Tar => bit7z_infra_bit7z::WriterFormat::Tar,
    ArchiveFormat::GZip => bit7z_infra_bit7z::WriterFormat::GZip,
    ArchiveFormat::BZip2 => bit7z_infra_bit7z::WriterFormat::BZip2,
    ArchiveFormat::Xz => bit7z_infra_bit7z::WriterFormat::Xz,
    ArchiveFormat::Wim => bit7z_infra_bit7z::WriterFormat::Wim,
    _ => unreachable!(),
};
```

In `cli_referee.rs`: update `detect_format()` to handle compound extensions:

```rust
fn detect_format(path: &Path) -> ArchiveFormat {
    let name = path.to_string_lossy().to_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return ArchiveFormat::TarGz;
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return ArchiveFormat::TarXz;
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") || name.ends_with(".tbz") {
        return ArchiveFormat::TarBz2;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some("7z") => ArchiveFormat::SevenZip,
        Some("zip") => ArchiveFormat::Zip,
        Some("tar") => ArchiveFormat::Tar,
        Some("gz") => ArchiveFormat::GZip,
        Some("bz2") => ArchiveFormat::BZip2,
        Some("xz") => ArchiveFormat::Xz,
        Some("wim") => ArchiveFormat::Wim,
        Some("rar") => ArchiveFormat::Rar,
        _ => ArchiveFormat::Zip,
    }
}
```

- [ ] **Step 5: Update capability registration**

In `runtime_service.rs`, use physical formats for capability registration:

```rust
fn register_bit7z_capabilities(registry: &CapabilityRegistry) {
    let formats = vec![
        ArchiveFormat::SevenZip,
        ArchiveFormat::Zip,
        ArchiveFormat::Tar,
        ArchiveFormat::GZip,
        ArchiveFormat::BZip2,
        ArchiveFormat::Xz,
        ArchiveFormat::Wim,
    ];
    // ... rest unchanged (for loop)
}
```

- [ ] **Step 6: Update CLI and UI format references**

In `crates/runtime/cli/src/lib.rs` — add all new formats to `parse_format()`:

```rust
pub fn parse_format(s: &str) -> Option<ArchiveFormat> {
    match s.to_lowercase().as_str() {
        "7z" | "sevenzip" => Some(ArchiveFormat::SevenZip),
        "zip" => Some(ArchiveFormat::Zip),
        "tar" => Some(ArchiveFormat::Tar),
        "tar.gz" | "tgz" | "targz" => Some(ArchiveFormat::TarGz),
        "tar.xz" | "txz" | "tarxz" => Some(ArchiveFormat::TarXz),
        "tar.bz2" | "tbz2" | "tarbz2" => Some(ArchiveFormat::TarBz2),
        "gz" | "gzip" => Some(ArchiveFormat::GZip),
        "bz2" | "bzip2" => Some(ArchiveFormat::BZip2),
        "xz" => Some(ArchiveFormat::Xz),
        "wim" => Some(ArchiveFormat::Wim),
        "rar" => Some(ArchiveFormat::Rar),
        _ => None,
    }
}
```

In `crates/presentation/settings/src/pages.rs` and `panels.rs` — add GZip/BZip2/Xz/Wim
to the format selection dropdown lists.

In `crates/presentation/dialogs/src/create.rs` — update create dialog format list
to include GZip/BZip2/Xz.

- [ ] **Step 7: Add capability constraint checks**

In `ArchiveService`, before creating an archive, check format features:

```rust
pub fn create(
    &self,
    path: &Path,
    format: ArchiveFormat,
    encryption: Option<&EncryptionConfig>,
) -> Result<ArchiveHandle, ArchiveError> {
    if encryption.is_some() && !format.supports_encryption() {
        return Err(ArchiveError::UnsupportedOperation(
            format!("{} does not support encryption", format.display_name()),
        ));
    }
    // ... rest of create ...
}
```

- [ ] **Step 8: Run full test suite**

Run: `cargo test 2>&1`
Expected: all tests pass (existing + new)

- [ ] **Step 9: Run clippy**

Run: `cargo clippy 2>&1`
Expected: no new warnings

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "feat: integrate AutoFormat detection + validation into ArchiveService"
```
