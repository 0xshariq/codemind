//! Identity helpers for stable repository keys, captured source manifests, and
//! graph projections. These are deliberately separate identity domains:
//! repository identity comes from a trusted stable key (remote normalization
//! belongs to `repository::identity`), snapshot identity comes from the source
//! manifest, and graph version identifies the projection contract for a
//! snapshot. A Git commit ID must never replace the source snapshot ID or graph version.

use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::Path};
use uuid::Uuid;

/// Bump this revision when the canonical graph projection contract changes in
/// a way that can alter graph meaning for identical captured source bytes. The
/// v4 added the explicit Test Graph node/relationship contract. v5 switched TIE
/// identity generation. v6 standardizes random SHA-256 IDs across CoIE,
/// History, TIE and ChIE graph records. Repository, workspace, snapshot and
/// operation identities remain stable because they are lifecycle contracts.
pub const GRAPH_PROJECTION_VERSION: &str = "codemind-canonical-graph-v6";

/// Mints opaque per-process graph-record IDs from fresh UUIDv4 entropy hashed
/// with SHA-256. A generator is scoped to one projection build so it can
/// explicitly reject any repeated digest before publishing a graph.
#[derive(Debug, Default)]
pub struct RandomIdGenerator {
    issued: HashSet<String>,
}

impl RandomIdGenerator {
    /// Generate a collision-checked SHA-256 ID in the supplied namespace.
    /// `domain` separates different logical record types that share a prefix.
    pub fn next(&mut self, prefix: &str, domain: &str) -> String {
        loop {
            let nonce = Uuid::new_v4();
            let mut material = Vec::with_capacity(prefix.len() + domain.len() + 48);
            append_component(&mut material, b"codemind-random-graph-id-v1");
            append_component(&mut material, prefix.as_bytes());
            append_component(&mut material, domain.as_bytes());
            append_component(&mut material, nonce.as_bytes());

            let id = format!("{prefix}:{}", digest_bytes(&material));
            if self.issued.insert(id.clone()) {
                return id;
            }
        }
    }

    /// Number of IDs minted by this generator. Useful for graph-build checks.
    pub fn issued_count(&self) -> usize {
        self.issued.len()
    }
}

/// Derive a namespaced SHA-256 identifier from an already-random parent ID.
/// This is used for related persisted records whose identity must remain
/// reproducible within one operation (for example impact/risk nodes).
pub fn derive_sha256_id(prefix: &str, domain: &str, random_parent_id: &str) -> String {
    let mut material =
        Vec::with_capacity(prefix.len() + domain.len() + random_parent_id.len() + 32);
    append_component(&mut material, b"codemind-derived-random-id-v1");
    append_component(&mut material, prefix.as_bytes());
    append_component(&mut material, domain.as_bytes());
    append_component(&mut material, random_parent_id.as_bytes());
    format!("{prefix}:{}", digest_bytes(&material))
}

fn append_component(material: &mut Vec<u8>, component: &[u8]) {
    material.extend_from_slice(&(component.len() as u64).to_be_bytes());
    material.extend_from_slice(component);
}

/// Stable lowercase SHA-256 hex digest. Keep all persisted digest generation
/// routed through this helper rather than adding module-specific hash logic.
pub fn digest_bytes(value: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(value);
    let mut output = String::with_capacity(64);
    for byte in digest.iter() {
        // Writing to a String is infallible; avoid unwrap/expect in this path.
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

pub fn digest_text(value: &str) -> String {
    digest_bytes(value.as_bytes())
}

/// Compute a deterministic graph-projection identity for a captured snapshot.
/// This identifies the graph representation expected for the input and schema
/// revision; it is not, by itself, proof that a graph write has succeeded.
pub fn graph_version_for_snapshot(source_snapshot_id: &str) -> String {
    let material = format!("{GRAPH_PROJECTION_VERSION}\0{source_snapshot_id}");
    format!("graph-{}", digest_text(&material))
}

/// Derive a repository ID from an explicitly supplied stable repository key,
/// such as a trusted application repository identifier or normalized remote.
pub fn repository_id_for_stable_identity(identity: &str) -> String {
    let identity = identity.trim();
    if identity.len() == 69
        && identity.starts_with("repo-")
        && identity[5..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return identity.to_owned();
    }
    format!(
        "repo-{}",
        digest_text(&format!("codemind-repository-identity-v1\0{identity}"))
    )
}

/// Fallback identity for an unregistered local directory. It is intentionally
/// path-bound because the caller has not supplied a stable logical repository
/// key; callers that need identity across checkouts should use the explicit
/// identity API on `SourceProvider`.
pub fn repository_id_for_local_path(canonical_root: &Path) -> String {
    let mut material = b"codemind-local-path-identity-v1\0".to_vec();
    // `as_encoded_bytes` avoids lossy UTF-8 conversion collisions for local
    // paths; this fallback is intentionally host/path-specific.
    material.extend_from_slice(canonical_root.as_os_str().as_encoded_bytes());
    format!("repo-{}", digest_bytes(&material))
}

#[cfg(test)]
mod tests {
    use super::{
        digest_bytes, digest_text, graph_version_for_snapshot, repository_id_for_stable_identity,
    };

    #[test]
    fn digest_is_stable_and_fixed_width() {
        let first = digest_text("codemind");
        assert_eq!(first, digest_text("codemind"));
        assert_eq!(first.len(), 64);
        assert_ne!(first, digest_text("codemind-change"));
        assert_eq!(first, digest_bytes(b"codemind"));
        assert_eq!(
            digest_text("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn random_graph_ids_are_sha256_namespaced_and_unique_within_a_projection() {
        let mut generator = super::RandomIdGenerator::default();
        let ids = (0..128)
            .map(|_| generator.next("engine-entity", "code-entity"))
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(ids.len(), 128);
        assert_eq!(generator.issued_count(), 128);
        assert!(ids.iter().all(|id| {
            id.strip_prefix("engine-entity:").is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        }));
    }

    #[test]
    fn graph_projection_and_repository_identity_are_domain_separated() {
        assert_eq!(
            graph_version_for_snapshot("snapshot-a"),
            graph_version_for_snapshot("snapshot-a")
        );
        assert_ne!(
            graph_version_for_snapshot("snapshot-a"),
            graph_version_for_snapshot("snapshot-b")
        );
        assert_eq!(
            repository_id_for_stable_identity("https://example.invalid/repo"),
            repository_id_for_stable_identity("https://example.invalid/repo")
        );
        assert_ne!(
            repository_id_for_stable_identity("repository-a"),
            repository_id_for_stable_identity("repository-b")
        );
        let canonical = repository_id_for_stable_identity("repository-a");
        assert_eq!(repository_id_for_stable_identity(&canonical), canonical);
    }
}
