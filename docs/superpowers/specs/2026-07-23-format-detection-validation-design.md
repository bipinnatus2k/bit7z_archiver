# Format Detection & Validation Layer

## Motivation

bit7z/7-zip is a C++ library with an unsafe FFI boundary and a history of CVEs.
Before handing a file to the C++ layer, we need a safe(Rust) pre-flight check
that can reject obviously malformed or mismatched archives without touching
unsafe code.

## ArchiveFormat: Align with bit7z

bit7z declares 7 writable (`BitInOutFormat`) and ~45 read-only (`BitInFormat`)
formats (see `bitformat.hpp`). The `ArchiveFormat` enum should cover:

1. All writable formats (needed for creation capability + UI)
2. Common read-only formats (needed for detection/display)
3. A catch-all for the long tail

```rust
pub enum ArchiveFormat {
    // --- Writable (BitInOutFormat) ---
    SevenZip,
    Zip,
    Tar,
    GZip,
    BZip2,
    Xz,
    Wim,

    // --- Compound aliases (extensions only, no magic bytes) ---
    TarGz,   // GZip wrapping Tar — extension .tar.gz
    TarBz2,  // BZip2 wrapping Tar — extension .tar.bz2
    TarXz,   // Xz wrapping Tar     — extension .tar.xz

    // --- Common read-only (BitInFormat) ---
    Rar,
    Rar5,
    Arj,
    Cab,
    Lzh,
    Lzma,
    Lzma86,
    Zstd,
    Ppmd,
    Nsis,
    Iso,
    Chm,
    Cpio,
    Deb,
    Rpm,
    Dmg,
    Vhd,
    Vmdk,
    Vdi,
    SquashFS,
    Compound,

    /// Catch-all for any bit7z format not explicitly listed (stores the
    /// 7z SDK format ID so the reader can open it by value).
    Other(u32),
}
```

New methods:

| Method | Returns | Purpose |
|---|---|---|
| `physical()` | `ArchiveFormat` | Compound → outer (TarGz → GZip); identity for others |
| `inner_format()` | `Option<ArchiveFormat>` | Compression-only → expected inner (GZip → Tar); `None` for others |
| `extensions()` | `&[&str]` | All known extensions for the format |
| `sdk_value()` | `Option<u32>` | 7z SDK format ID for runtime use; `None` for compound aliases |

Compound variants (TarGz/TarBz2/TarXz) delegate:
- `physical() → GZip / BZip2 / Xz`
- `inner_format() → Some(Tar)`
- `extensions() → ["tar.gz", "tgz"]` etc.
- `sdk_value() → None` (use `physical()` to get the SDK value)

## file_format Integration

Add `file_format` crate as a dependency in the application crate.

`file_format::FileFormat` identifies the file from magic bytes. Mapping to
`ArchiveFormat` covers all formats bit7z can read, translating to the
appropriate variant (including `Other(id)` for the long tail of disk-image
and filesystem formats).

```rust
fn file_format_to_archive_format(ff: file_format::FileFormat) -> ArchiveFormat {
    match ff {
        // Writable
        FileFormat::SevenZip => ArchiveFormat::SevenZip,
        FileFormat::Zip | FileFormat::JavaArchive | FileFormat::AndroidPackage
            | FileFormat::OfficeOpenXmlDocument // etc.
            => ArchiveFormat::Zip,
        FileFormat::Tar => ArchiveFormat::Tar,
        FileFormat::GZip => ArchiveFormat::GZip,
        FileFormat::BZip2 => ArchiveFormat::BZip2,
        FileFormat::Xz => ArchiveFormat::Xz,
        FileFormat::Wim => ArchiveFormat::Wim,
        // Common read-only
        FileFormat::Rar => ArchiveFormat::Rar,
        FileFormat::Rar5 => ArchiveFormat::Rar5,
        FileFormat::Arj => ArchiveFormat::Arj,
        FileFormat::Cab => ArchiveFormat::Cab,
        FileFormat::Lzh => ArchiveFormat::Lzh,
        FileFormat::Lzma => ArchiveFormat::Lzma,
        FileFormat::Iso => ArchiveFormat::Iso,
        FileFormat::Chm => ArchiveFormat::Chm,
        FileFormat::Cpio => ArchiveFormat::Cpio,
        FileFormat::Deb => ArchiveFormat::Deb,
        FileFormat::Rpm => ArchiveFormat::Rpm,
        FileFormat::Dmg => ArchiveFormat::Dmg,
        FileFormat::Vhd => ArchiveFormat::Vhd,
        FileFormat::Squashfs => ArchiveFormat::SquashFS,
        // Everything else detectable by file_format → catch-all
        _ => ArchiveFormat::Other(0), // falls through to bit7z Auto
    }
}
```

## Detection

```
AutoFormat::detect(path) → Result<Detection, DetectionError>
               │
               ├──① read first bytes (file_format::FileFormat::from_bytes)
               │
               ├──② map to ArchiveFormat (physical_format)
               │     GZip magic → ArchiveFormat::GZip
               │     Zip  magic → ArchiveFormat::Zip
               │     etc.
               │
               ├──③ parse extension chain → derive extensions Vec
               │     ".tar.gz"   → ["tar.gz", "gz"]
               │     ".7z.001"   → ["001", "7z"]
               │     ".zip"      → ["zip"]
               │
               ├──④ resolve logical format from extension chain
               │     ".tar.gz" → known compound → format=TarGz, inner=Tar
               │     ".gz"     → GZip, no known inner
               │     ".7z.001" → SevenZip, multi-volume
               │
               ├──⑤ consistency: physical × logical × extension
               │     GZip + TarGz + ".tar.gz" → ✓
               │     Zip  + TarGz + ".tar.gz" → ✗ FormatMismatch
               │
               ├──⑥ if validator registered for physical_format → run it
               │
               └──⑦ return Detection
```

```rust
pub struct Detection {
    pub format: ArchiveFormat,              // 物理格式（如 GZip）
    pub logical: Option<ArchiveFormat>,     // 逻辑格式（如 TarGz），已知复合时设置
    pub inner: Option<ArchiveFormat>,       // 内层预期格式（如 Tar）
    pub extensions: Vec<String>,            // 原始扩展名链
    pub is_multi_volume: bool,
    pub total_volumes: Option<u32>,
    pub validated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown or unsupported archive format")]
    UnknownFormat,
    #[error("format mismatch: magic bytes indicate {magic:?} but extension suggests {extension:?}")]
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

## Format Validator — Independent Capability

Structural validation is a separate concern from format detection. The
`ValidatorRegistry` is an optional service that `AutoFormat` can query.

```rust
/// Pure-Rust structural validator for a single archive format.
#[async_trait]
pub trait FormatValidator: Send + Sync {
    fn format(&self) -> ArchiveFormat;
    /// Returns Ok(()) if the data is well-formed, or a list of errors.
    async fn validate(&self, path: &Path, data: &[u8]) -> Result<(), Vec<String>>;
}

/// Registry of registered validators.
pub struct ValidatorRegistry {
    validators: HashMap<ArchiveFormat, Box<dyn FormatValidator>>,
}

impl ValidatorRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, v: Box<dyn FormatValidator>);
    pub fn get(&self, fmt: ArchiveFormat) -> Option<&dyn FormatValidator>;
    /// Convenience: validate if a validator exists for this format.
    pub fn validate(
        &self,
        fmt: ArchiveFormat,
        path: &Path,
        data: &[u8],
    ) -> Result<(), Vec<String>>;
}
```

`AutoFormat` takes an optional `Arc<ValidatorRegistry>`:

```rust
pub struct AutoFormat {
    validators: Option<Arc<ValidatorRegistry>>,
}
```

In `detect()`, after magic + extension checks pass:

```rust
if let Some(registry) = &self.validators {
    registry.validate(physical_format, path, &header)?;
}
// validated = true only if a validator actually ran and passed
```

This separation means:
- **Validators work standalone** — CLI tool, background batch scan, CI checks
- **AutoFormat stays focused** — magic detection + extension consistency only
- **Registry is composable** — tests can inject mock validators

### Architectural note: parallel registries

`ValidatorRegistry` is a minimal registry that exists because
`CapabilityRegistry` was originally intended as a general-purpose DI container
(Runtime Capability Graph) but has since become archive-specific
(`CapabilityKind` only has Read/Write/Extract/Test/Preview/Hash/Compress/Encrypt).

In the future, `CapabilityRegistry` should be generalized to accept arbitrary
capability kinds (including `Validate`), at which point `ValidatorRegistry`
can be folded into it. For now, keeping them separate is the pragmatic choice:
it avoids refactoring the entire capability system while achieving the
immediate safety goal.

### Phase 1 validators

| Format | Approach | Depth |
|---|---|---|
| Zip | `nom`-based EOCD → CD → LFH chain | Full structural + boundary check + path traversal |
| Tar | Header block structure check | Basic |
| GZip | Magic + header flags | Minimal (full validation requires decompression) |
| 7z | Magic + signature block | Minimal |
| Others | Magic only (validated = false) | None |

Zip is the priority because it's the most common format with the most CVEs.

## Capability Constraint Integration

Wire the existing but unused `Constraint` system into the application layer:

```rust
// In ArchiveService::create()
let constraints = if encryption.is_some() {
    vec![Constraint::RequireEncryption]
} else {
    vec![]
};

// Check format supports the requested features
if !format.supports_encryption() && encryption.is_some() {
    return Err(ArchiveError::UnsupportedOperation(
        "format does not support encryption"
    ));
}
```

This catches errors early, before the request reaches the runtime:

| Operation | Check | Error |
|---|---|---|
| Tar archive with encryption | `Tar.supports_encryption()` → false | Reject at application layer |
| 7z with ZipCrypto (AES only) | `SevenZip.default_encryption()` | Reject or fallback |

## Integration Points

### ArchiveService::open()

```rust
pub fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
    // 1. Safe pre-flight
    let detection = self.auto_format.detect(path)?;

    // 2. Capability resolution uses detected format
    let descriptor = self.resolver.resolve(CapabilityRequest {
        kind: CapabilityKind::Read,
        format: detection.format,
        constraints: vec![],
        session_id: None,
    })?;

    // 3. Submit to runtime
    let handle = self.runtime.submit(OperationRequest {
        kind: OperationKind::OpenArchive {
            path: path.to_path_buf(),
            password: password.cloned(),
        },
        descriptor,
        priority: Priority::User,
    });

    // ... existing wait/session logic ...
}
```

### Compound / nested archive flow

bit7z's `Auto` mode handles format chaining internally (e.g. `.tar.gz` opens
transparently). The detection layer does NOT attempt to decompress or parse
inner layers — it only validates the outer layer on disk.

When the user opens a nested inner archive (e.g. extracts a `.tar` entry from
a `.zip` and then opens that `.tar`), the same `detect()` runs on the inner
file, giving it its own validation pass.

```rust
// Outer: .tar.gz
let d = AutoFormat::detect("archive.tar.gz")?;
// d = Detection { format: GZip, logical: Some(TarGz), inner: Some(Tar), ... }

// bit7z Auto opens it transparently — user sees files, not a .tar blob
// When user later opens inner file "data.tar" extracted from archive:
let d2 = AutoFormat::detect("data.tar")?;
// d2 = Detection { format: Tar, inner: None, ... }
```

### Where the detection result is used

| Consumer | Field used |
|---|---|
| Capability resolution | `detection.logical.unwrap_or(detection.format)` |
| Session metadata | `detection.format` + `detection.inner` |
| UI display | `detection.logical.map_or_else(\|\| detection.format.display_name(), \|f\| f.display_name())` |
| Nested archive open | `detection.inner` hints at what to expect |

## Multi-Volume Handling

bit7z FFI provides:
- `bit7z_reader_is_multi_volume(raw_ptr)`
- `bit7z_reader_volumes_count(raw_ptr)`

Detection layer identifies multi-volume from filename pattern:
- `.7z.001`, `.7z.002` (or `.7z.001`, `.7z.002` on some tools)
- `.zip.001`, `.zip.002`
- `.rar`, `.r00`, `.r01`

The reader-side multi-volume info is queried AFTER opening (via the FFI);
the detection layer only flags the filename pattern.

## Out of Scope (Phase 1)

- Content decompression for inner-format validation (e.g. decompressing GZip
  to validate contained Tar)
- Full 7z structural parsing (complex solid-block model)
- Cross-volume structural validation
- Wim format support beyond magic detection
