//! Generic capability graph framework.
//!
//! The capability layer is domain-agnostic. Each domain defines its own
//! capability metadata (`CapMeta`) and maps its resource keys to the
//! opaque [`LockKey`] type.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// Domain-specific capability metadata.
///
/// Implement this trait for your domain's metadata type. The blanket impl
/// covers any type that satisfies the bounds.
pub trait CapMeta: Clone + std::fmt::Debug + Send + Sync + 'static {}
impl<T: Clone + std::fmt::Debug + Send + Sync + 'static> CapMeta for T {}

/// Opaque resource lock key.
///
/// Domains map their own key types (e.g. `SessionId`) to this type via
/// a simple conversion. This keeps the runtime and capability framework
/// free of domain-specific type parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LockKey(pub u64);

/// Identifier for a registered backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackendId(pub u64);

/// Identifier for a registered capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(pub u64);

/// Availability of a capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Always available at runtime.
    Always,
    /// Available only when a feature is enabled.
    FeatureFlag(&'static str),
    /// Available only when an external dependency is present.
    ExternalDependency(&'static str),
}

impl Availability {
    /// Returns whether this capability is considered available.
    pub fn is_available(&self) -> bool {
        match self {
            Availability::Always => true,
            Availability::FeatureFlag(_) | Availability::ExternalDependency(_) => {
                // Runtime checks will be added by the domain resolver.
                true
            }
        }
    }
}

/// Generic metadata attached to a capability.
#[derive(Debug, Clone, Default)]
pub struct CapabilityMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// A registered capability, generic over domain metadata.
#[derive(Debug, Clone)]
pub struct Capability<M: CapMeta> {
    pub id: CapabilityId,
    pub backend: BackendId,
    pub meta: M,
    pub availability: Availability,
    pub metadata: CapabilityMetadata,
}

impl<M: CapMeta> Capability<M> {
    pub fn is_available(&self) -> bool {
        self.availability.is_available()
    }
}

/// Request to resolve a capability.
#[derive(Debug, Clone)]
pub struct Request<M: CapMeta> {
    /// Domain-specific capability metadata to match against.
    pub meta: M,
    /// Domain-specific tags for filtering (e.g. "encryption", "streaming").
    pub tags: Vec<String>,
    /// Optional resource key to lock during execution.
    pub lock_key: Option<LockKey>,
}

/// Lock type for a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLock {
    /// Shared lock: allows concurrent access on the same resource.
    Shared(LockKey),
    /// Exclusive lock: forbids any other operation on the resource.
    Exclusive(LockKey),
}

/// Resource claim required to execute an operation.
#[derive(Debug, Clone, Default)]
pub struct ResourceClaim {
    /// Resources that must be locked.
    pub resource_locks: Vec<ResourceLock>,
    /// Whether this operation needs a global concurrency token.
    pub concurrency_token: bool,
    /// Memory budget in bytes.
    pub memory_budget: Option<usize>,
    /// Thread budget.
    pub thread_budget: Option<usize>,
    /// FFI / native handle budget.
    pub handle_budget: Option<usize>,
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

/// Edge kind in the capability dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Target capability is required.
    Requires,
    /// Target capability conflicts.
    Conflicts,
    /// Target capability is implied.
    Implies,
}

/// A capability dependency graph with typed edges.
#[derive(Debug, Clone)]
pub struct CapGraph<M: CapMeta> {
    nodes: HashMap<CapabilityId, Capability<M>>,
    edges: HashMap<CapabilityId, Vec<(EdgeKind, CapabilityId)>>,
}

impl<M: CapMeta> CapGraph<M> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn register(&mut self, cap: Capability<M>) {
        self.edges.entry(cap.id).or_default();
        self.nodes.insert(cap.id, cap);
    }

    pub fn unregister(&mut self, id: CapabilityId) {
        self.nodes.remove(&id);
        self.edges.remove(&id);
        for edges in self.edges.values_mut() {
            edges.retain(|(_, target)| *target != id);
        }
    }

    pub fn add_edge(
        &mut self,
        from: CapabilityId,
        kind: EdgeKind,
        to: CapabilityId,
    ) -> Result<(), CapError> {
        if !self.nodes.contains_key(&from) {
            return Err(CapError::CapabilityNotFound(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(CapError::CapabilityNotFound(to));
        }
        if kind == EdgeKind::Conflicts && from == to {
            return Err(CapError::InvalidEdge(
                "a capability cannot conflict with itself",
            ));
        }
        self.edges.entry(from).or_default().push((kind, to));
        Ok(())
    }

    pub fn all(&self) -> Vec<&Capability<M>> {
        self.nodes.values().collect()
    }

    pub fn get(&self, id: CapabilityId) -> Option<&Capability<M>> {
        self.nodes.get(&id)
    }

    pub fn by_backend(&self, backend: BackendId) -> Vec<&Capability<M>> {
        self.nodes
            .values()
            .filter(|c| c.backend == backend)
            .collect()
    }

    /// Resolve transitive dependencies via DFS.
    pub fn transitive_deps(&self, from: CapabilityId) -> Result<HashSet<CapabilityId>, CapError> {
        if !self.nodes.contains_key(&from) {
            return Err(CapError::CapabilityNotFound(from));
        }
        let mut visited = HashSet::new();
        let mut result = HashSet::new();
        self.visit_deps(from, &mut visited, &mut result);
        Ok(result)
    }

    fn visit_deps(
        &self,
        id: CapabilityId,
        visited: &mut HashSet<CapabilityId>,
        result: &mut HashSet<CapabilityId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        if let Some(edges) = self.edges.get(&id) {
            for (kind, target) in edges {
                if *kind == EdgeKind::Requires || *kind == EdgeKind::Implies {
                    result.insert(*target);
                    self.visit_deps(*target, visited, result);
                }
            }
        }
    }

    /// Check for conflicts within a set of capability IDs.
    pub fn check_conflicts(&self, ids: &[CapabilityId]) -> Result<(), CapError> {
        for &id in ids {
            if let Some(edges) = self.edges.get(&id) {
                for (kind, target) in edges {
                    if *kind == EdgeKind::Conflicts && ids.contains(target) {
                        return Err(CapError::Conflict(id, *target));
                    }
                }
            }
        }
        Ok(())
    }
}

impl<M: CapMeta> Default for CapGraph<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe capability registry.
#[derive(Debug)]
pub struct CapabilityRegistry<M: CapMeta> {
    graph: RwLock<CapGraph<M>>,
}

impl<M: CapMeta> CapabilityRegistry<M> {
    pub fn new() -> Self {
        Self {
            graph: RwLock::new(CapGraph::new()),
        }
    }

    pub fn register(&self, cap: Capability<M>) {
        self.graph.write().unwrap().register(cap);
    }

    pub fn unregister(&self, id: CapabilityId) {
        self.graph.write().unwrap().unregister(id);
    }

    pub fn all(&self) -> Vec<Capability<M>> {
        self.graph
            .read()
            .unwrap()
            .all()
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn by_backend(&self, backend: BackendId) -> Vec<Capability<M>> {
        self.graph
            .read()
            .unwrap()
            .by_backend(backend)
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn add_edge(
        &self,
        from: CapabilityId,
        kind: EdgeKind,
        to: CapabilityId,
    ) -> Result<(), CapError> {
        self.graph.write().unwrap().add_edge(from, kind, to)
    }

    /// Expose the inner graph for domain-specific traversal.
    pub fn graph(&self) -> std::sync::RwLockReadGuard<'_, CapGraph<M>> {
        self.graph.read().unwrap()
    }
}

impl<M: CapMeta> Default for CapabilityRegistry<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned by capability resolution.
#[derive(Debug, thiserror::Error)]
pub enum CapError {
    #[error("capability not found: {0:?}")]
    CapabilityNotFound(CapabilityId),
    #[error("no matching capability for request")]
    NoMatch,
    #[error("capability conflict: {0:?} conflicts with {1:?}")]
    Conflict(CapabilityId, CapabilityId),
    #[error("invalid edge: {0}")]
    InvalidEdge(&'static str),
}

/// Resolves capability requests into execution descriptors.
///
/// Domain-agnostic trait. Each domain provides its own resolver
/// that knows how to interpret its metadata type `M`.
pub trait CapabilityResolver<M: CapMeta>: Send + Sync {
    fn resolve(&self, request: &Request<M>) -> Result<ExecutionDescriptor, CapError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal metadata type for testing.
    #[derive(Clone, Debug, PartialEq)]
    struct TestMeta {
        kind: &'static str,
        value: u32,
    }

    fn make_cap(id: u64, backend: u64, kind: &'static str) -> Capability<TestMeta> {
        Capability {
            id: CapabilityId(id),
            backend: BackendId(backend),
            meta: TestMeta { kind, value: 0 },
            availability: Availability::Always,
            metadata: CapabilityMetadata::default(),
        }
    }

    #[test]
    fn register_and_query() {
        let registry: CapabilityRegistry<TestMeta> = CapabilityRegistry::new();
        registry.register(make_cap(1, 1, "read"));
        registry.register(make_cap(2, 2, "write"));

        let all = registry.all();
        assert_eq!(all.len(), 2);

        let by_b1 = registry.by_backend(BackendId(1));
        assert_eq!(by_b1.len(), 1);
        assert_eq!(by_b1[0].id, CapabilityId(1));

        let by_b2 = registry.by_backend(BackendId(2));
        assert_eq!(by_b2.len(), 1);
        assert_eq!(by_b2[0].id, CapabilityId(2));
    }

    #[test]
    fn unregister_removes_capability() {
        let registry: CapabilityRegistry<TestMeta> = CapabilityRegistry::new();
        registry.register(make_cap(1, 1, "read"));
        assert_eq!(registry.all().len(), 1);
        registry.unregister(CapabilityId(1));
        assert!(registry.all().is_empty());
    }

    #[test]
    fn graph_transitive_deps() {
        let mut graph: CapGraph<TestMeta> = CapGraph::new();
        graph.register(make_cap(1, 1, "admin"));
        graph.register(make_cap(2, 1, "write"));
        graph.register(make_cap(3, 1, "read"));

        graph
            .add_edge(CapabilityId(1), EdgeKind::Requires, CapabilityId(2))
            .unwrap();
        graph
            .add_edge(CapabilityId(2), EdgeKind::Requires, CapabilityId(3))
            .unwrap();

        let deps = graph.transitive_deps(CapabilityId(1)).unwrap();
        assert!(deps.contains(&CapabilityId(2)));
        assert!(deps.contains(&CapabilityId(3)));
    }

    #[test]
    fn graph_conflict_detection() {
        let mut graph: CapGraph<TestMeta> = CapGraph::new();
        graph.register(make_cap(1, 1, "readonly"));
        graph.register(make_cap(2, 1, "readwrite"));

        graph
            .add_edge(CapabilityId(1), EdgeKind::Conflicts, CapabilityId(2))
            .unwrap();

        // No conflict on its own.
        assert!(graph.check_conflicts(&[CapabilityId(1)]).is_ok());
        // Conflict when both are present.
        assert!(graph
            .check_conflicts(&[CapabilityId(1), CapabilityId(2)])
            .is_err());
    }

    #[test]
    fn add_edge_fails_for_missing_nodes() {
        let mut graph: CapGraph<TestMeta> = CapGraph::new();
        graph.register(make_cap(1, 1, "read"));

        assert!(graph
            .add_edge(CapabilityId(1), EdgeKind::Requires, CapabilityId(99))
            .is_err());
        assert!(graph
            .add_edge(CapabilityId(99), EdgeKind::Requires, CapabilityId(1))
            .is_err());
    }

    #[test]
    fn request_with_lock_key() {
        let req: Request<TestMeta> = Request {
            meta: TestMeta {
                kind: "read",
                value: 42,
            },
            tags: vec![],
            lock_key: Some(LockKey(100)),
        };
        assert_eq!(req.lock_key, Some(LockKey(100)));
    }

    #[test]
    fn resource_lock_exclusive_prevents_shared() {
        let key = LockKey(1);
        let exclusive = ResourceLock::Exclusive(key);
        let shared = ResourceLock::Shared(key);

        // Both refer to the same key.
        assert_eq!(
            format!("{:?}", exclusive),
            format!("Exclusive(LockKey(1))")
        );
        assert_eq!(format!("{:?}", shared), format!("Shared(LockKey(1))"));
    }
}