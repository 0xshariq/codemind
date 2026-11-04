//! Remote ref inspection without retaining a local clone.
//!
//! Remote inspection distinguishes unchanged, changed, and unknown state. An
//! inaccessible remote is never treated as proof that stored knowledge is
//! current.

use super::{
    GitCommandConfig, GitError, classify_remote_failure,
    command::{output_text, run_git},
    validate_commit_id, validate_ref,
};
use crate::repository::RepositoryIdentity;
use chrono::{DateTime, Utc};
use std::ffi::OsString;

/// A resolved remote reference. `commit_id` is the commit/object hash resolved
/// for this reference; for an annotated tag, a peeled `^{}` target is preferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteHead {
    pub commit_id: String,
    pub reference: String,
    pub symbolic_ref: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteFreshness {
    Unchanged,
    Changed,
    Unknown,
}

/// Result of a remote freshness observation. A missing remote observation is
/// deliberately distinct from an unchanged remote: callers must never treat
/// an inaccessible remote as proof that stored knowledge is current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFreshnessObservation {
    pub freshness: RemoteFreshness,
    pub remote_head: Option<RemoteHead>,
}

/// Resolve the remote's advertised HEAD and symbolic default branch.
pub async fn resolve_remote_head(
    remote_locator: &str,
    config: &GitCommandConfig,
) -> Result<RemoteHead, GitError> {
    let identity = RepositoryIdentity::from_remote(remote_locator)?;
    let remote = identity
        .remote_locator()
        .ok_or(GitError::InvalidRemoteLocator)?;
    let args = vec![
        OsString::from("ls-remote"),
        OsString::from("--symref"),
        OsString::from("--"),
        OsString::from(remote),
        OsString::from("HEAD"),
    ];
    let output = run_git("remote-head", args, None, config).await?;
    if !output.status.success() {
        return Err(classify_remote_failure("remote-head", &output));
    }
    let text = output_text(&output, "remote-head")?;
    let mut commit_id = None;
    let mut symbolic_ref = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("ref: ") {
            if let Some((reference, name)) = rest.split_once('\t')
                && name == "HEAD"
            {
                validate_ref(reference)?;
                if let Some(previous) = symbolic_ref.as_deref()
                    && previous != reference
                {
                    return Err(GitError::AmbiguousRemoteRef);
                }
                symbolic_ref = Some(reference.to_owned());
            }
            continue;
        }
        if let Some((oid, name)) = line.split_once('\t')
            && name == "HEAD"
        {
            record_unique_commit(&mut commit_id, oid)?;
        }
    }
    let commit_id = commit_id.ok_or_else(|| GitError::RemoteRefNotFound {
        reference: "HEAD".to_owned(),
    })?;
    Ok(RemoteHead {
        commit_id,
        reference: "HEAD".to_owned(),
        symbolic_ref,
        checked_at: Utc::now(),
    })
}

/// Resolve a named remote branch or tag. A short name prefers a branch
/// (`refs/heads/<name>`) and falls back to a tag; fully-qualified `refs/heads/`
/// and `refs/tags/` names are accepted. Inputs are validated before process start.
pub async fn resolve_remote_ref(
    remote_locator: &str,
    requested_ref: &str,
    config: &GitCommandConfig,
) -> Result<RemoteHead, GitError> {
    validate_ref(requested_ref)?;
    let identity = RepositoryIdentity::from_remote(remote_locator)?;
    let remote = identity
        .remote_locator()
        .ok_or(GitError::InvalidRemoteLocator)?;
    let (branch_reference, tag_reference) = if requested_ref.starts_with("refs/heads/") {
        (Some(requested_ref.to_owned()), None)
    } else if requested_ref.starts_with("refs/tags/") {
        (None, Some(requested_ref.to_owned()))
    } else if requested_ref.starts_with("refs/") {
        return Err(GitError::InvalidRef);
    } else {
        (
            Some(format!("refs/heads/{requested_ref}")),
            Some(format!("refs/tags/{requested_ref}")),
        )
    };

    let mut args = vec![
        OsString::from("ls-remote"),
        OsString::from("--symref"),
        OsString::from("--"),
        OsString::from(remote),
    ];
    if let Some(reference) = branch_reference.as_deref() {
        validate_ref(reference)?;
        args.push(OsString::from(reference));
    }
    if let Some(reference) = tag_reference.as_deref() {
        validate_ref(reference)?;
        args.push(OsString::from(reference));
        // `ls-remote` only advertises peeled annotated-tag targets when the
        // peeled ref pattern is requested explicitly. Keep the query exact;
        // do not use a wildcard that may match an unbounded set of tags.
        args.push(OsString::from(format!("{reference}^{{}}")));
    }
    let output = run_git("remote-ref", args, None, config).await?;
    if !output.status.success() {
        return Err(classify_remote_failure("remote-ref", &output));
    }
    let text = output_text(&output, "remote-ref")?;
    let (reference, commit_id) = parse_requested_ref_output(
        text,
        requested_ref,
        branch_reference.as_deref(),
        tag_reference.as_deref(),
    )?;

    Ok(RemoteHead {
        commit_id,
        reference,
        symbolic_ref: None,
        checked_at: Utc::now(),
    })
}

pub(super) fn parse_requested_ref_output(
    text: &str,
    requested_ref: &str,
    branch_reference: Option<&str>,
    tag_reference: Option<&str>,
) -> Result<(String, String), GitError> {
    let mut branch_oid = None;
    let mut tag_oid = None;
    let mut peeled_tag_oid = None;
    let peeled_tag_reference = tag_reference.map(|reference| format!("{reference}^{{}}"));
    for line in text.lines() {
        if let Some((oid, name)) = line.split_once('\t') {
            if let Some(reference) = branch_reference
                && name == reference
            {
                record_unique_commit(&mut branch_oid, oid)?;
            }
            if let Some(reference) = tag_reference {
                if name == reference {
                    record_unique_commit(&mut tag_oid, oid)?;
                } else if peeled_tag_reference.as_deref() == Some(name) {
                    record_unique_commit(&mut peeled_tag_oid, oid)?;
                }
            }
        }
    }
    // A short name follows Git's branch-first convention. If no branch with
    // that name exists, allow a tag; annotated tags use the peeled commit hash.
    match (branch_oid, peeled_tag_oid, tag_oid) {
        (Some(commit), _, _) => Ok((
            branch_reference.ok_or(GitError::InvalidRef)?.to_owned(),
            commit,
        )),
        (None, Some(commit), _) => Ok((
            tag_reference.ok_or(GitError::InvalidRef)?.to_owned(),
            commit,
        )),
        (None, None, Some(commit)) => Ok((
            tag_reference.ok_or(GitError::InvalidRef)?.to_owned(),
            commit,
        )),
        (None, None, None) => Err(GitError::RemoteRefNotFound {
            reference: requested_ref.to_owned(),
        }),
    }
}

/// Compare an advertised remote reference with the last successfully analysed
/// commit. Errors mean freshness is unknown; they must never be interpreted as
/// an unchanged repository.
pub async fn observe_remote_freshness(
    remote_locator: &str,
    requested_ref: Option<&str>,
    last_verified_commit: &str,
    config: &GitCommandConfig,
) -> Result<RemoteFreshnessObservation, GitError> {
    let last_verified_commit = validate_commit_id(last_verified_commit)?;
    let remote = match requested_ref {
        Some(reference) => resolve_remote_ref(remote_locator, reference, config).await,
        None => resolve_remote_head(remote_locator, config).await,
    };

    let remote = match remote {
        Ok(remote) => remote,
        Err(error) if is_unknown_remote_state(&error) => {
            return Ok(RemoteFreshnessObservation {
                freshness: RemoteFreshness::Unknown,
                remote_head: None,
            });
        }
        Err(error) => return Err(error),
    };

    let freshness = if remote.commit_id == last_verified_commit {
        RemoteFreshness::Unchanged
    } else {
        RemoteFreshness::Changed
    };
    Ok(RemoteFreshnessObservation {
        freshness,
        remote_head: Some(remote),
    })
}

/// Compatibility helper for callers that only need the three-state freshness
/// decision. Detailed callers should prefer `observe_remote_freshness` so the
/// successful remote HEAD can be recorded in durable repository state.
pub async fn check_remote_freshness(
    remote_locator: &str,
    requested_ref: Option<&str>,
    last_verified_commit: &str,
    config: &GitCommandConfig,
) -> Result<RemoteFreshness, GitError> {
    Ok(
        observe_remote_freshness(remote_locator, requested_ref, last_verified_commit, config)
            .await?
            .freshness,
    )
}

fn record_unique_commit(slot: &mut Option<String>, value: &str) -> Result<(), GitError> {
    let commit_id = validate_commit_id(value)?;
    if let Some(previous) = slot.as_deref() {
        if !previous.eq_ignore_ascii_case(&commit_id) {
            return Err(GitError::AmbiguousRemoteRef);
        }
    } else {
        *slot = Some(commit_id);
    }
    Ok(())
}

pub(super) fn is_unknown_remote_state(error: &GitError) -> bool {
    matches!(
        error,
        GitError::InvalidRemoteLocator
            | GitError::RepositoryIdentity(_)
            | GitError::RemoteAuthentication { .. }
            | GitError::RemoteNetwork { .. }
            | GitError::RemoteRepositoryUnavailable { .. }
            | GitError::RemoteRefNotFound { .. }
            | GitError::InvalidRef
            | GitError::InvalidCommitId
            | GitError::AmbiguousRemoteRef
            | GitError::MalformedOutput { .. }
            | GitError::OutputLimitExceeded { .. }
            // Any Git process/transport failure means the remote state was not
            // proven. Do not infer an empty diff from an unclassified failure.
            | GitError::Command(_)
    )
}
