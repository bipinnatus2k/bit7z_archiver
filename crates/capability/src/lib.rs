//! Capability registry and resolver.
//!
//! The capability layer answers the question: "Can this operation be executed,
//! and by whom?" It sits between the Application layer and the Runtime layer.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bit7z_domain::archive::ArchiveFormat;
use bit7z_ports::fs::FileSystem;
use bit7z_ports::progress::ProgressReporter;

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
}

/// Request to resolve a capability.
#[derive(Debug, Clone)]
pub struct CapabilityRequest {
    pub kind: CapabilityKind,
    pub format: ArchiveFormat,
    pub constraints: Vec<Constraint>,
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
        self.capabilities.read().unwrap().values().cloned().collect()
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
        let candidates: Vec<&Capability> = caps
            .iter()
            .filter(|c| c.kind == request.kind && c.formats.contains(&request.format))
            .collect();

        let chosen = candidates
            .first()
            .ok_or_else(|| CapabilityError::NoMatchingBackend(request.clone()))?;

        Ok(ExecutionDescriptor {
            backend: chosen.backend,
            capabilities: vec![chosen.id],
            resource_claim: ResourceClaim::default(),
            policy: ExecutionPolicy::Queued,
        })
    }
}
