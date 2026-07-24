//! Archive-specific capability types and resolver.
//!
//! This module bridges the generic capability framework to the archive domain.
//! It defines:
//!
//! - [`ArchiveOpKind`]: the set of operations an archive backend can perform.
//! - [`ArchiveCapMeta`]: metadata carried by each archive capability.
//! - [`ArchiveCapabilityResolver`]: domain-specific resolver logic (filtering,
//!   constraint scoring, resource-claim construction, execution-policy selection).

use std::sync::Arc;

use bit7z_capability::{
    Availability, BackendId, CapError, Capability, CapabilityId, CapabilityMetadata,
    CapabilityRegistry, CapabilityResolver, ExecutionDescriptor, ExecutionPolicy,
    Request, ResourceClaim, ResourceLock,
};
use bit7z_domain::archive::ArchiveFormat;

// ---------------------------------------------------------------------------
// Archive capability kinds
// ---------------------------------------------------------------------------

/// Archive-specific operation kinds that a backend can provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveOpKind {
    Read,
    Write,
    Extract,
    Test,
    Preview,
    Hash,
    Compress,
    Encrypt,
    /// Magic-bytes-based format detection.
    DetectFormat,
}

// ---------------------------------------------------------------------------
// Archive capability metadata
// ---------------------------------------------------------------------------

/// Metadata carried by each archive capability.
///
/// This replaces the old flat boolean flags (`supports_encryption`, …) with a
/// struct that both identifies the operation and describes its feature set.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveCapMeta {
    pub kind: ArchiveOpKind,
    /// Formats this capability supports (registration) or the target format (request).
    pub formats: Vec<ArchiveFormat>,
    pub supports_encryption: bool,
    pub supports_solid: bool,
    pub supports_streaming: bool,
    pub supports_incremental: bool,
}

// The blanket impl in `capability` covers this type automatically:
//   impl CapMeta for ArchiveCapMeta {}

// ---------------------------------------------------------------------------
// Archive capability registration helpers
// ---------------------------------------------------------------------------

/// Register the standard set of bit7z archive capabilities.
///
/// All capabilities are registered under the same backend id with the same
/// set of supported formats. Callers that need a different set of formats
/// or feature flags should register capabilities individually.
pub fn register_archive_capabilities(
    registry: &CapabilityRegistry<ArchiveCapMeta>,
    backend_id: BackendId,
    formats: &[ArchiveFormat],
) {
    for (id, kind) in [
        (1, ArchiveOpKind::Read),
        (2, ArchiveOpKind::Write),
        (3, ArchiveOpKind::Extract),
        (4, ArchiveOpKind::Test),
        (5, ArchiveOpKind::Preview),
        (6, ArchiveOpKind::Hash),
        (7, ArchiveOpKind::Compress),
        (8, ArchiveOpKind::Encrypt),
    ] {
        registry.register(Capability {
            id: CapabilityId(id),
            backend: backend_id,
            meta: ArchiveCapMeta {
                kind,
                formats: formats.to_vec(),
                supports_encryption: kind == ArchiveOpKind::Encrypt
                    || kind == ArchiveOpKind::Write,
                supports_solid: kind == ArchiveOpKind::Write
                    || kind == ArchiveOpKind::Compress,
                supports_streaming: kind == ArchiveOpKind::Extract,
                supports_incremental: kind == ArchiveOpKind::Write,
            },
            availability: Availability::Always,
            metadata: CapabilityMetadata {
                name: format!("{:?}", kind),
                description: String::new(),
                version: "1.0".into(),
            },
        });
    }

    // Format detection capability — supports all formats the backend can detect.
    registry.register(Capability {
        id: CapabilityId(9),
        backend: backend_id,
        meta: ArchiveCapMeta {
            kind: ArchiveOpKind::DetectFormat,
            formats: formats.to_vec(),
            supports_encryption: false,
            supports_solid: false,
            supports_streaming: false,
            supports_incremental: false,
        },
        availability: Availability::Always,
        metadata: CapabilityMetadata {
            name: "DetectFormat".into(),
            description: "Magic-bytes archive format detection".into(),
            version: "1.0".into(),
        },
    });
}

/// Default set of formats that the bit7z backend supports.
pub fn bit7z_formats() -> Vec<ArchiveFormat> {
    vec![
        ArchiveFormat::SevenZip,
        ArchiveFormat::Zip,
        ArchiveFormat::Tar,
        ArchiveFormat::GZip,
        ArchiveFormat::BZip2,
        ArchiveFormat::Xz,
        ArchiveFormat::Wim,
    ]
}

// ---------------------------------------------------------------------------
// Archive capability resolver
// ---------------------------------------------------------------------------

/// Resolver that matches archive capability requests against registered
/// capabilities.
///
/// This is the direct replacement for the old `DefaultCapabilityResolver`.
/// It knows how to:
///
/// 1. Filter candidates by kind, format, and availability.
/// 2. Apply explicit constraints (encryption, solid, streaming, …).
/// 3. Score and sort candidates by constraint satisfaction.
/// 4. Build a `ResourceClaim` with session locks and memory budgets.
/// 5. Determine the `ExecutionPolicy`.
pub struct ArchiveCapabilityResolver {
    registry: Arc<CapabilityRegistry<ArchiveCapMeta>>,
}

impl ArchiveCapabilityResolver {
    pub fn new(registry: Arc<CapabilityRegistry<ArchiveCapMeta>>) -> Self {
        Self { registry }
    }
}

impl CapabilityResolver<ArchiveCapMeta> for ArchiveCapabilityResolver {
    fn resolve(&self, request: &Request<ArchiveCapMeta>) -> Result<ExecutionDescriptor, CapError> {
        let caps = self.registry.all();

        // 1. Basic filtering by kind, format overlap, and availability.
        let mut candidates: Vec<&Capability<ArchiveCapMeta>> = caps
            .iter()
            .filter(|c| {
                if c.meta.kind != request.meta.kind {
                    return false;
                }
                // For format detection any detector is acceptable;
                // the caller will invoke the detector to learn the format.
                if request.meta.kind == ArchiveOpKind::DetectFormat {
                    return c.is_available();
                }
                c.meta.formats.iter().any(|f| request.meta.formats.contains(f))
                    && c.is_available()
            })
            .collect();

        // 2. Apply explicit tags as constraints.
        for tag in &request.tags {
            candidates.retain(|c| match tag.as_str() {
                "encryption" => c.meta.supports_encryption,
                "solid" => c.meta.supports_solid,
                "streaming" => c.meta.supports_streaming,
                "incremental" => c.meta.supports_incremental,
                _ => true,
            });
        }

        // 3. Score by tag satisfaction and pick the best match.
        candidates.sort_by(|a, b| {
            let a_score = score_tags(&request.tags, &a.meta);
            let b_score = score_tags(&request.tags, &b.meta);
            b_score.cmp(&a_score)
        });

        let chosen = *candidates
            .first()
            .ok_or(CapError::NoMatch)?;

        // 4. Build resource claim.
        let mut resource_claim = ResourceClaim::default();
        if let Some(lock_key) = request.lock_key {
            let lock = match request.meta.kind {
                ArchiveOpKind::Read
                | ArchiveOpKind::Extract
                | ArchiveOpKind::Test
                | ArchiveOpKind::Preview
                | ArchiveOpKind::Hash => ResourceLock::Shared(lock_key),
                _ => ResourceLock::Exclusive(lock_key),
            };
            resource_claim.resource_locks.push(lock);
        }
        resource_claim.memory_budget = match request.meta.kind {
            ArchiveOpKind::Compress | ArchiveOpKind::Encrypt | ArchiveOpKind::Extract => {
                Some(64 * 1024 * 1024)
            }
            _ => None,
        };

        // 5. Determine execution policy.
        let policy = match request.meta.kind {
            ArchiveOpKind::Read
            | ArchiveOpKind::Test
            | ArchiveOpKind::Preview
            | ArchiveOpKind::DetectFormat => ExecutionPolicy::Immediate,
            ArchiveOpKind::Extract | ArchiveOpKind::Hash => ExecutionPolicy::Queued,
            ArchiveOpKind::Write | ArchiveOpKind::Compress | ArchiveOpKind::Encrypt => {
                ExecutionPolicy::Queued
            }
        };

        Ok(ExecutionDescriptor {
            backend: chosen.backend,
            capabilities: vec![chosen.id],
            resource_claim,
            policy,
        })
    }
}

fn score_tags(tags: &[String], meta: &ArchiveCapMeta) -> usize {
    let mut score = 0;
    for tag in tags {
        let satisfied = match tag.as_str() {
            "encryption" => meta.supports_encryption,
            "solid" => meta.supports_solid,
            "streaming" => meta.supports_streaming,
            "incremental" => meta.supports_incremental,
            _ => true,
        };
        if satisfied {
            score += 1;
        }
    }
    score
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_capability::LockKey;

    fn make_cap(
        id: u64,
        backend: u64,
        kind: ArchiveOpKind,
        formats: Vec<ArchiveFormat>,
    ) -> Capability<ArchiveCapMeta> {
        Capability {
            id: CapabilityId(id),
            backend: BackendId(backend),
            meta: ArchiveCapMeta {
                kind,
                formats,
                supports_encryption: false,
                supports_solid: false,
                supports_streaming: false,
                supports_incremental: false,
            },
            availability: Availability::Always,
            metadata: CapabilityMetadata::default(),
        }
    }

    #[test]
    fn resolve_selects_backend_by_kind_and_format() {
        let registry = Arc::new(CapabilityRegistry::new());
        registry.register(make_cap(
            1,
            1,
            ArchiveOpKind::Read,
            vec![ArchiveFormat::SevenZip],
        ));
        registry.register(make_cap(
            2,
            2,
            ArchiveOpKind::Write,
            vec![ArchiveFormat::Zip],
        ));

        let resolver = ArchiveCapabilityResolver::new(registry);
        let request = Request {
            meta: ArchiveCapMeta {
                kind: ArchiveOpKind::Read,
                formats: vec![ArchiveFormat::SevenZip],
                supports_encryption: false,
                supports_solid: false,
                supports_streaming: false,
                supports_incremental: false,
            },
            tags: vec![],
            lock_key: None,
        };
        let descriptor = resolver.resolve(&request).unwrap();
        assert_eq!(descriptor.backend, BackendId(1));
        assert_eq!(descriptor.policy, ExecutionPolicy::Immediate);
    }

    #[test]
    fn resolve_applies_encryption_constraint() {
        let registry = Arc::new(CapabilityRegistry::new());
        let mut encrypted = make_cap(
            2,
            2,
            ArchiveOpKind::Read,
            vec![ArchiveFormat::Zip],
        );
        encrypted.meta.supports_encryption = true;
        registry.register(make_cap(
            1,
            1,
            ArchiveOpKind::Read,
            vec![ArchiveFormat::Zip],
        ));
        registry.register(encrypted);

        let resolver = ArchiveCapabilityResolver::new(registry);
        let request = Request {
            meta: ArchiveCapMeta {
                kind: ArchiveOpKind::Read,
                formats: vec![ArchiveFormat::Zip],
                supports_encryption: false,
                supports_solid: false,
                supports_streaming: false,
                supports_incremental: false,
            },
            tags: vec!["encryption".into()],
            lock_key: None,
        };
        let descriptor = resolver.resolve(&request).unwrap();
        assert_eq!(descriptor.backend, BackendId(2));
    }

    #[test]
    fn resolve_adds_session_lock() {
        let registry = Arc::new(CapabilityRegistry::new());
        registry.register(make_cap(
            1,
            1,
            ArchiveOpKind::Write,
            vec![ArchiveFormat::Zip],
        ));

        let resolver = ArchiveCapabilityResolver::new(registry);
        let request = Request {
            meta: ArchiveCapMeta {
                kind: ArchiveOpKind::Write,
                formats: vec![ArchiveFormat::Zip],
                supports_encryption: false,
                supports_solid: false,
                supports_streaming: false,
                supports_incremental: false,
            },
            tags: vec![],
            lock_key: Some(LockKey(42)),
        };
        let descriptor = resolver.resolve(&request).unwrap();
        assert_eq!(descriptor.resource_claim.resource_locks.len(), 1);
        assert!(matches!(
            descriptor.resource_claim.resource_locks[0],
            ResourceLock::Exclusive(LockKey(42))
        ));
    }

    #[test]
    fn resolve_fails_when_no_backend_matches() {
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = ArchiveCapabilityResolver::new(registry);
        let request = Request {
            meta: ArchiveCapMeta {
                kind: ArchiveOpKind::Read,
                formats: vec![ArchiveFormat::Rar],
                supports_encryption: false,
                supports_solid: false,
                supports_streaming: false,
                supports_incremental: false,
            },
            tags: vec![],
            lock_key: None,
        };
        assert!(resolver.resolve(&request).is_err());
    }
}