//! Capability registry and resolver.
//!
//! The capability layer answers the question: "Can this operation be executed,
//! and by whom?" It sits between the Application layer and the Runtime layer.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bit7z_domain::archive::ArchiveFormat;

/// Identifier for a registered backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackendId(pub u64);

/// Identifier for a registered capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(pub u64);

/// Kind of capability a backend can provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    Read,
    Write,
    Extract,
    Test,
    Preview,
    Hash,
    Compress,
    Encrypt,
}

/// Availability of a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Always available at runtime.
    Always,
    /// Available only when a feature is enabled.
    FeatureFlag(&'static str),
    /// Available only when an external dependency is present.
    ExternalDependency(&'static str),
}

/// Metadata attached to a capability.
#[derive(Debug, Clone, Default)]
pub struct CapabilityMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// A registered capability.
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: CapabilityId,
    pub backend: BackendId,
    pub kind: CapabilityKind,
    pub formats: Vec<ArchiveFormat>,
    pub availability: Availability,
    pub metadata: CapabilityMetadata,
    pub supports_encryption: bool,
    pub supports_solid: bool,
    pub supports_streaming: bool,
    pub supports_incremental: bool,
}

impl Capability {
    /// Returns whether this capability is considered available at this time.
    ///
    /// The current implementation treats every capability as available; runtime
    /// checks for feature flags or external dependencies will be added later.
    pub fn is_available(&self) -> bool {
        true
    }
}

/// Request to resolve a capability.
#[derive(Debug, Clone)]
pub struct CapabilityRequest {
    pub kind: CapabilityKind,
    pub format: ArchiveFormat,
    pub constraints: Vec<Constraint>,
    pub session_id: Option<bit7z_domain::archive::SessionId>,
}

/// Constraint on a capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraint {
    /// Requires encryption support.
    RequireEncryption,
    /// Requires solid archive support.
    RequireSolid,
    /// Requires a specific backend.
    Backend(BackendId),
    /// Requires writing in a streaming manner.
    RequireStreaming,
    /// Requires incremental update support.
    RequireIncremental,
}

/// Resource claim required to execute an operation.
#[derive(Debug, Clone, Default)]
pub struct ResourceClaim {
    /// Sessions that must be locked.
    pub session_locks: Vec<SessionLock>,
    /// Whether this operation needs a global concurrency token.
    pub concurrency_token: bool,
    /// Memory budget in bytes.
    pub memory_budget: Option<usize>,
    /// Thread budget.
    pub thread_budget: Option<usize>,
    /// Temporary directory space in bytes.
    pub temp_storage: Option<usize>,
    /// FFI/native handle budget.
    pub handle_budget: Option<usize>,
}

/// Lock type for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLock {
    /// Shared lock: allows concurrent reads on the same session.
    Shared(bit7z_domain::archive::SessionId),
    /// Exclusive lock: forbids any other operation on the session.
    Exclusive(bit7z_domain::archive::SessionId),
}

/// Policy for executing an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPolicy {
    /// Run immediately if resources allow.
    Immediate,
    /// Queue and run when resources allow.
    Queued,
    /// Run in the background.
    Background,
}

/// Descriptor produced by the resolver that tells the runtime how to execute.
#[derive(Debug, Clone)]
pub struct ExecutionDescriptor {
    pub backend: BackendId,
    pub capabilities: Vec<CapabilityId>,
    pub resource_claim: ResourceClaim,
    pub policy: ExecutionPolicy,
}

/// Registry for backend capabilities.
#[derive(Debug)]
pub struct CapabilityRegistry {
    capabilities: RwLock<HashMap<CapabilityId, Capability>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, cap: Capability) {
        self.capabilities.write().unwrap().insert(cap.id, cap);
    }

    pub fn unregister(&self, id: CapabilityId) {
        self.capabilities.write().unwrap().remove(&id);
    }

    pub fn all(&self) -> Vec<Capability> {
        self.capabilities
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    pub fn by_backend(&self, backend: BackendId) -> Vec<Capability> {
        self.capabilities
            .read()
            .unwrap()
            .values()
            .filter(|c| c.backend == backend)
            .cloned()
            .collect()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
        }
    }
}

/// Error returned by capability resolution.
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("no backend satisfies the request: {0:?}")]
    NoMatchingBackend(CapabilityRequest),
    #[error("required constraint unsupported: {0:?}")]
    UnsupportedConstraint(Constraint),
    #[error("backend unavailable: {0:?}")]
    BackendUnavailable(BackendId),
}

/// Resolves capability requests into execution descriptors.
pub trait CapabilityResolver: Send + Sync {
    fn resolve(&self, request: CapabilityRequest) -> Result<ExecutionDescriptor, CapabilityError>;
}

/// Default resolver that matches against a registry.
pub struct DefaultCapabilityResolver {
    registry: Arc<CapabilityRegistry>,
}

impl DefaultCapabilityResolver {
    pub fn new(registry: Arc<CapabilityRegistry>) -> Self {
        Self { registry }
    }
}

impl CapabilityResolver for DefaultCapabilityResolver {
    fn resolve(&self, request: CapabilityRequest) -> Result<ExecutionDescriptor, CapabilityError> {
        let caps = self.registry.all();

        // 1. Basic filtering by kind, format, and availability.
        let mut candidates: Vec<&Capability> = caps
            .iter()
            .filter(|c| {
                c.kind == request.kind && c.formats.contains(&request.format) && c.is_available()
            })
            .collect();

        // 2. Apply explicit constraints.
        for constraint in &request.constraints {
            candidates.retain(|c| match constraint {
                Constraint::RequireEncryption => c.supports_encryption,
                Constraint::RequireSolid => c.supports_solid,
                Constraint::RequireStreaming => c.supports_streaming,
                Constraint::RequireIncremental => c.supports_incremental,
                Constraint::Backend(b) => c.backend == *b,
            });
        }

        // 3. Prefer backends that satisfy the most explicit constraints.
        candidates.sort_by(|a, b| {
            let a_score = score(&request.constraints, a);
            let b_score = score(&request.constraints, b);
            b_score.cmp(&a_score)
        });

        let chosen = (*candidates
            .first()
            .ok_or_else(|| CapabilityError::NoMatchingBackend(request.clone()))?)
        .clone();

        // 4. Build resource claim based on operation kind and session.
        let mut resource_claim = ResourceClaim::default();
        if let Some(session_id) = request.session_id {
            let lock = match request.kind {
                CapabilityKind::Read
                | CapabilityKind::Extract
                | CapabilityKind::Test
                | CapabilityKind::Preview
                | CapabilityKind::Hash => SessionLock::Shared(session_id),
                _ => SessionLock::Exclusive(session_id),
            };
            resource_claim.session_locks.push(lock);
        }
        // Memory-intensive operations require a budget hint.
        resource_claim.memory_budget = match request.kind {
            CapabilityKind::Compress | CapabilityKind::Encrypt | CapabilityKind::Extract => {
                Some(64 * 1024 * 1024)
            }
            _ => None,
        };

        // 5. Determine execution policy.
        let policy = match request.kind {
            CapabilityKind::Read | CapabilityKind::Test | CapabilityKind::Preview => {
                ExecutionPolicy::Immediate
            }
            CapabilityKind::Extract | CapabilityKind::Hash => ExecutionPolicy::Queued,
            CapabilityKind::Write | CapabilityKind::Compress | CapabilityKind::Encrypt => {
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

fn score(constraints: &[Constraint], cap: &Capability) -> usize {
    let mut score = 0;
    for c in constraints {
        let satisfied = match c {
            Constraint::RequireEncryption => cap.supports_encryption,
            Constraint::RequireSolid => cap.supports_solid,
            Constraint::RequireStreaming => cap.supports_streaming,
            Constraint::RequireIncremental => cap.supports_incremental,
            Constraint::Backend(b) => cap.backend == *b,
        };
        if satisfied {
            score += 1;
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::ArchiveFormat;

    fn make_cap(
        id: u64,
        backend: u64,
        kind: CapabilityKind,
        formats: &[ArchiveFormat],
    ) -> Capability {
        Capability {
            id: CapabilityId(id),
            backend: BackendId(backend),
            kind,
            formats: formats.to_vec(),
            availability: Availability::Always,
            metadata: CapabilityMetadata::default(),
            supports_encryption: false,
            supports_solid: false,
            supports_streaming: false,
            supports_incremental: false,
        }
    }

    #[test]
    fn resolve_selects_backend_by_kind_and_format() {
        let registry = Arc::new(CapabilityRegistry::new());
        registry.register(make_cap(
            1,
            1,
            CapabilityKind::Read,
            &[ArchiveFormat::SevenZip],
        ));
        registry.register(make_cap(2, 2, CapabilityKind::Write, &[ArchiveFormat::Zip]));

        let resolver = DefaultCapabilityResolver::new(registry);
        let request = CapabilityRequest {
            kind: CapabilityKind::Read,
            format: ArchiveFormat::SevenZip,
            constraints: vec![],
            session_id: None,
        };
        let descriptor = resolver.resolve(request).unwrap();
        assert_eq!(descriptor.backend, BackendId(1));
        assert_eq!(descriptor.policy, ExecutionPolicy::Immediate);
    }

    #[test]
    fn resolve_applies_constraints() {
        let registry = Arc::new(CapabilityRegistry::new());
        let plain = make_cap(1, 1, CapabilityKind::Read, &[ArchiveFormat::Zip]);
        let mut encrypted = make_cap(2, 2, CapabilityKind::Read, &[ArchiveFormat::Zip]);
        encrypted.supports_encryption = true;
        registry.register(plain);
        registry.register(encrypted);

        let resolver = DefaultCapabilityResolver::new(registry);
        let request = CapabilityRequest {
            kind: CapabilityKind::Read,
            format: ArchiveFormat::Zip,
            constraints: vec![Constraint::RequireEncryption],
            session_id: None,
        };
        let descriptor = resolver.resolve(request).unwrap();
        assert_eq!(descriptor.backend, BackendId(2));
    }

    #[test]
    fn resolve_adds_session_lock() {
        let registry = Arc::new(CapabilityRegistry::new());
        registry.register(make_cap(1, 1, CapabilityKind::Write, &[ArchiveFormat::Zip]));

        let resolver = DefaultCapabilityResolver::new(registry);
        let request = CapabilityRequest {
            kind: CapabilityKind::Write,
            format: ArchiveFormat::Zip,
            constraints: vec![],
            session_id: Some(42),
        };
        let descriptor = resolver.resolve(request).unwrap();
        assert_eq!(descriptor.resource_claim.session_locks.len(), 1);
        assert!(matches!(
            descriptor.resource_claim.session_locks[0],
            SessionLock::Exclusive(42)
        ));
    }

    #[test]
    fn resolve_fails_when_no_backend_matches() {
        let registry = Arc::new(CapabilityRegistry::new());
        let resolver = DefaultCapabilityResolver::new(registry);
        let request = CapabilityRequest {
            kind: CapabilityKind::Read,
            format: ArchiveFormat::Rar,
            constraints: vec![],
            session_id: None,
        };
        assert!(resolver.resolve(request).is_err());
    }
}
