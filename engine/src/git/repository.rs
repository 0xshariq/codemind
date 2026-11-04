//! Opening and cloning Git repositories.
//!
//! Remote clones must be placed in a `WorkspaceManager`-created workspace.
//! This module never deletes workspaces; persistence/verification and cleanup
//! ordering remain the repository lifecycle manager's responsibility.

use super::{
    GitError, classify_remote_failure,
    command::{GitCommandConfig, output_line, run_git, run_git_checked},
    validate_commit_id,
};
use crate::{
    repository::{RepositoryIdentity, WorkspaceManager, WorkspacePhase},
    source::identity::digest_bytes,
};
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

/// A read-only observation of caller-owned Git state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalGitStateObservation {
    /// Stable digest of HEAD, status and staged-index diff.
    pub digest: String,
    /// Whether the checkout has no staged, unstaged, untracked or rename/delete
    /// status entries at the time this observation was collected.
    pub working_tree_clean: bool,
}

/// A validated Git checkout. `root` is the working-tree root and `head_commit`
/// is the commit currently checked out; neither is a CodeMind source snapshot
/// identifier. A cloned checkout records the owning operation but local inputs
/// are never considered owned by CodeMind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepository {
    root: PathBuf,
    head_commit: String,
    branch: Option<String>,
    remote_locator: Option<String>,
    owning_operation_id: Option<String>,
}

impl GitRepository {
    /// Clone a remote repository into an operation workspace created by
    /// `WorkspaceManager::create`, resolve HEAD, then advance the workspace to
    /// `Analyzing`. On error, the marker is left in place for the coordinator to
    /// report and clean through the safe workspace API.
    pub async fn clone_remote(
        workspaces: &WorkspaceManager,
        operation_id: &str,
        config: &GitCommandConfig,
    ) -> Result<Self, GitError> {
        let marker = workspaces.load_marker(operation_id)?;
        if marker.phase != WorkspacePhase::Preparing {
            return Err(GitError::WorkspacePhaseMismatch {
                expected: WorkspacePhase::Preparing,
                actual: marker.phase,
            });
        }
        let marker = workspaces.transition(operation_id, WorkspacePhase::Cloning)?;
        let result =
            Self::clone_prepared_workspace(workspaces, operation_id, &marker, config).await;
        if let Err(error) = &result {
            let code = match error {
                GitError::RemoteAuthentication { .. } => "git_authentication_failed",
                GitError::RemoteNetwork { .. } => "git_network_failed",
                GitError::RemoteRepositoryUnavailable { .. } => "git_remote_unavailable",
                GitError::Command(_) => "git_command_failed",
                GitError::Workspace(_) => "workspace_state_failed",
                _ => "git_prepare_failed",
            };
            let _ = workspaces.record_failure_code(operation_id, code);
        }
        result
    }

    /// Reopen a complete clone retained after a retryable indexing failure.
    ///
    /// The workspace marker, canonical parent, `.git` directory, remote origin
    /// and recorded HEAD are all revalidated before the marker advances to
    /// `Analyzing`. This does not reclone or mutate the existing source tree.
    pub async fn resume_retryable_workspace(
        workspaces: &WorkspaceManager,
        operation_id: &str,
        config: &GitCommandConfig,
    ) -> Result<Self, GitError> {
        let marker = workspaces.load_marker(operation_id)?;
        if marker.phase != WorkspacePhase::RetryableFailure {
            return Err(GitError::WorkspacePhaseMismatch {
                expected: WorkspacePhase::RetryableFailure,
                actual: marker.phase,
            });
        }
        let expected_head = marker
            .head_commit
            .as_deref()
            .ok_or_else(|| GitError::InvalidClone(marker.repository_root.clone()))?;
        let identity = RepositoryIdentity::from_remote(&marker.remote_locator)?;
        if identity.repository_id() != marker.repository_id {
            return Err(GitError::RepositoryIdentityMismatch);
        }

        let checkout_metadata =
            fs::symlink_metadata(&marker.repository_root).map_err(|source| {
                GitError::Filesystem {
                    path: marker.repository_root.clone(),
                    source,
                }
            })?;
        if checkout_metadata.file_type().is_symlink() || !checkout_metadata.is_dir() {
            return Err(GitError::InvalidClone(marker.repository_root.clone()));
        }
        let canonical_root =
            fs::canonicalize(&marker.repository_root).map_err(|source| GitError::Filesystem {
                path: marker.repository_root.clone(),
                source,
            })?;
        let canonical_workspace =
            fs::canonicalize(&marker.workspace_root).map_err(|source| GitError::Filesystem {
                path: marker.workspace_root.clone(),
                source,
            })?;
        if canonical_root.parent() != Some(canonical_workspace.as_path()) {
            return Err(GitError::InvalidClone(canonical_root));
        }

        let dot_git = canonical_root.join(".git");
        let dot_git_metadata =
            fs::symlink_metadata(&dot_git).map_err(|source| GitError::Filesystem {
                path: dot_git.clone(),
                source,
            })?;
        if dot_git_metadata.file_type().is_symlink() || !dot_git_metadata.is_dir() {
            return Err(GitError::InvalidClone(canonical_root));
        }

        let verified_root = repository_top_level(&canonical_root, config).await?;
        if verified_root != canonical_root {
            return Err(GitError::RepositoryRootMismatch {
                expected: canonical_root,
                actual: verified_root,
            });
        }

        let remote_output = run_git_checked(
            "resume-remote-origin",
            vec![
                OsString::from("remote"),
                OsString::from("get-url"),
                OsString::from("origin"),
            ],
            Some(&canonical_root),
            config,
        )
        .await?;
        let remote_origin = output_line(&remote_output, "resume-remote-origin")?;
        let observed_identity = RepositoryIdentity::from_remote(remote_origin)?;
        if observed_identity.repository_id() != marker.repository_id {
            return Err(GitError::RepositoryIdentityMismatch);
        }

        let head_commit = resolve_head_commit(&canonical_root, config).await?;
        if !head_commit.eq_ignore_ascii_case(expected_head) {
            return Err(GitError::InvalidClone(canonical_root));
        }
        let branch = resolve_current_branch(&canonical_root, config).await?;
        workspaces.resume_analysis(operation_id)?;

        Ok(Self {
            root: canonical_root,
            head_commit,
            branch,
            remote_locator: Some(marker.remote_locator),
            owning_operation_id: Some(operation_id.to_owned()),
        })
    }

    async fn clone_prepared_workspace(
        workspaces: &WorkspaceManager,
        operation_id: &str,
        marker: &crate::repository::WorkspaceMarker,
        config: &GitCommandConfig,
    ) -> Result<Self, GitError> {
        let identity = RepositoryIdentity::from_remote(&marker.remote_locator)?;
        if identity.repository_id() != marker.repository_id {
            return Err(GitError::RepositoryIdentityMismatch);
        }

        match fs::symlink_metadata(&marker.repository_root) {
            Ok(_) => {
                return Err(GitError::CloneDestinationAlreadyExists(
                    marker.repository_root.clone(),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(GitError::Filesystem {
                    path: marker.repository_root.clone(),
                    source,
                });
            }
        }

        let mut args = vec![OsString::from("clone")];
        let requested_ref = marker.requested_ref.as_deref();
        let explicit_tag =
            requested_ref.is_some_and(|reference| reference.starts_with("refs/tags/"));
        let skip_tags = requested_ref.is_none()
            || requested_ref.is_some_and(|reference| reference.starts_with("refs/heads/"));
        if skip_tags {
            // Avoid fetching all tags for the default-branch and explicit-branch
            // paths. Short refs may resolve to tags, so leave tags enabled there.
            args.push(OsString::from("--no-tags"));
        }
        args.push(OsString::from("--no-recurse-submodules"));
        if let Some(requested_ref) = requested_ref
            && !explicit_tag
        {
            let branch = clone_branch_name(requested_ref)?;
            args.push(OsString::from("--branch"));
            args.push(OsString::from(branch));
        }
        args.push(OsString::from("--"));
        args.push(OsString::from(
            identity
                .remote_locator()
                .ok_or(GitError::InvalidRemoteLocator)?
                .to_owned(),
        ));
        args.push(marker.repository_root.as_os_str().to_os_string());

        let clone_output = run_git("clone", args, None, config).await?;
        if !clone_output.status.success() {
            if !explicit_tag && requested_ref.is_some() && {
                let diagnostic = String::from_utf8_lossy(&clone_output.stderr).to_ascii_lowercase();
                diagnostic.contains("remote branch") && diagnostic.contains("not found")
            } {
                return Err(GitError::RemoteRefNotFound {
                    reference: requested_ref.unwrap_or_default().to_owned(),
                });
            }
            return Err(classify_remote_failure("clone", &clone_output));
        }
        if clone_output.stdout_truncated {
            return Err(GitError::OutputLimitExceeded {
                operation: "clone",
                stream: "stdout",
            });
        }
        // Inspect the configured destination before canonicalizing it: checking
        // metadata only after canonicalization would hide a final-component
        // symlink by returning the symlink target's path.
        let checkout_metadata =
            fs::symlink_metadata(&marker.repository_root).map_err(|source| {
                GitError::Filesystem {
                    path: marker.repository_root.clone(),
                    source,
                }
            })?;
        if checkout_metadata.file_type().is_symlink() || !checkout_metadata.is_dir() {
            return Err(GitError::InvalidClone(marker.repository_root.clone()));
        }

        let canonical_root =
            fs::canonicalize(&marker.repository_root).map_err(|source| GitError::Filesystem {
                path: marker.repository_root.clone(),
                source,
            })?;
        let canonical_workspace =
            fs::canonicalize(&marker.workspace_root).map_err(|source| GitError::Filesystem {
                path: marker.workspace_root.clone(),
                source,
            })?;
        if canonical_root.parent() != Some(canonical_workspace.as_path()) {
            return Err(GitError::InvalidClone(canonical_root));
        }
        let dot_git = canonical_root.join(".git");
        let dot_git_metadata =
            fs::symlink_metadata(&dot_git).map_err(|source| GitError::Filesystem {
                path: dot_git.clone(),
                source,
            })?;
        if dot_git_metadata.file_type().is_symlink() || !dot_git_metadata.is_dir() {
            return Err(GitError::InvalidClone(canonical_root));
        }

        if explicit_tag {
            let reference = requested_ref.ok_or(GitError::InvalidRef)?;
            // Fetch the exact tag ref before checking it out. Automatic tag
            // following may omit tags that point outside the default branch's
            // reachable history, and a fully qualified tag must not resolve to
            // a same-named branch.
            let fetch_refspec = format!("+{reference}:{reference}");
            let fetch_args = vec![
                OsString::from("fetch"),
                OsString::from("--no-tags"),
                OsString::from("origin"),
                OsString::from(fetch_refspec),
            ];
            let fetch_output = run_git(
                "fetch-requested-tag",
                fetch_args,
                Some(&canonical_root),
                config,
            )
            .await?;
            if !fetch_output.status.success() {
                let diagnostic = String::from_utf8_lossy(&fetch_output.stderr).to_ascii_lowercase();
                if diagnostic.contains("couldn't find remote ref")
                    || diagnostic.contains("could not find remote ref")
                {
                    return Err(GitError::RemoteRefNotFound {
                        reference: reference.to_owned(),
                    });
                }
                return Err(classify_remote_failure(
                    "fetch-requested-tag",
                    &fetch_output,
                ));
            }
            let checkout_args = vec![
                OsString::from("checkout"),
                OsString::from("--detach"),
                OsString::from(reference),
            ];
            run_git_checked(
                "checkout-requested-tag",
                checkout_args,
                Some(&canonical_root),
                config,
            )
            .await?;
        }

        let verified_root = repository_top_level(&canonical_root, config).await?;
        if verified_root != canonical_root {
            return Err(GitError::RepositoryRootMismatch {
                expected: canonical_root,
                actual: verified_root,
            });
        }
        let head_commit = resolve_head_commit(&canonical_root, config).await?;
        let branch = resolve_current_branch(&canonical_root, config).await?;
        workspaces.record_head_commit(operation_id, &head_commit)?;
        workspaces.transition(operation_id, WorkspacePhase::Analyzing)?;

        Ok(Self {
            root: canonical_root,
            head_commit,
            branch,
            remote_locator: Some(identity.remote_locator().unwrap_or_default().to_owned()),
            owning_operation_id: Some(operation_id.to_owned()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn head_commit(&self) -> &str {
        &self.head_commit
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// Returns only a credential-free normalized locator.
    pub fn remote_locator(&self) -> Option<&str> {
        self.remote_locator.as_deref()
    }

    /// Some only for a clone that CodeMind created in an owned workspace.
    pub fn owning_operation_id(&self) -> Option<&str> {
        self.owning_operation_id.as_deref()
    }

    pub fn is_owned_temporary_clone(&self) -> bool {
        self.owning_operation_id.is_some()
    }

    /// Fingerprint the caller-owned Git state which is not represented by the
    /// source manifest alone. The source snapshot captures working-tree bytes;
    /// this fingerprint additionally covers HEAD, index contents/status,
    /// unstaged status, untracked paths, renames and deletions. It is read-only
    /// and never writes a tree/object or modifies the caller's checkout.
    pub async fn local_state_observation(
        &self,
        config: &GitCommandConfig,
    ) -> Result<LocalGitStateObservation, GitError> {
        if self.is_owned_temporary_clone() {
            return Err(GitError::LocalFingerprintRequiresCallerOwnedRepository);
        }

        let head_output = run_git_checked(
            "local-state-head",
            vec![
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                OsString::from("HEAD^{commit}"),
            ],
            Some(&self.root),
            config,
        )
        .await?;
        if head_output.stdout_truncated {
            return Err(GitError::OutputLimitExceeded {
                operation: "local-state-head",
                stream: "stdout",
            });
        }
        let head = validate_commit_id(output_line(&head_output, "local-state-head")?)?;
        if !head.eq_ignore_ascii_case(&self.head_commit) {
            return Err(GitError::LocalRepositoryHeadChanged);
        }

        // `--no-optional-locks` prevents status from refreshing the index as a
        // side effect. `-z` keeps arbitrary path bytes unambiguous.
        let status = run_git(
            "local-state-status",
            vec![
                OsString::from("--no-optional-locks"),
                OsString::from("status"),
                OsString::from("--porcelain=v2"),
                OsString::from("-z"),
                OsString::from("--untracked-files=all"),
                OsString::from("--renames"),
                OsString::from("--ignore-submodules=none"),
            ],
            Some(&self.root),
            config,
        )
        .await?;
        status.ensure_success("local-state-status")?;
        if status.stdout_truncated {
            return Err(GitError::OutputLimitExceeded {
                operation: "local-state-status",
                stream: "stdout",
            });
        }

        // Porcelain status reports which paths changed, while the cached raw
        // diff includes staged blob identities. Together with the source
        // manifest, this catches a staged index that differs even when the
        // working-tree bytes have been restored to the old snapshot.
        let staged = run_git(
            "local-state-index-diff",
            vec![
                OsString::from("--no-optional-locks"),
                OsString::from("diff"),
                OsString::from("--cached"),
                OsString::from("--raw"),
                OsString::from("-z"),
                OsString::from("--no-ext-diff"),
                OsString::from("--no-textconv"),
                OsString::from("--no-abbrev"),
                OsString::from("HEAD"),
                OsString::from("--"),
            ],
            Some(&self.root),
            config,
        )
        .await?;
        staged.ensure_success("local-state-index-diff")?;
        if staged.stdout_truncated {
            return Err(GitError::OutputLimitExceeded {
                operation: "local-state-index-diff",
                stream: "stdout",
            });
        }

        // HEAD can move while status/index are being inspected (for example,
        // another process may complete a commit). Read it again so the digest
        // never combines an old commit identity with a newer index/status view.
        let final_head_output = run_git_checked(
            "local-state-head-final",
            vec![
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                OsString::from("HEAD^{commit}"),
            ],
            Some(&self.root),
            config,
        )
        .await?;
        if final_head_output.stdout_truncated {
            return Err(GitError::OutputLimitExceeded {
                operation: "local-state-head-final",
                stream: "stdout",
            });
        }
        let final_head =
            validate_commit_id(output_line(&final_head_output, "local-state-head-final")?)?;
        if !final_head.eq_ignore_ascii_case(&head) {
            return Err(GitError::LocalRepositoryHeadChanged);
        }

        let working_tree_clean = status.stdout.is_empty();
        let mut material = b"codemind-local-git-state-v1\0".to_vec();
        for part in [
            head.as_bytes(),
            status.stdout.as_slice(),
            staged.stdout.as_slice(),
        ] {
            material.extend_from_slice(&(part.len() as u64).to_be_bytes());
            material.extend_from_slice(part);
        }
        Ok(LocalGitStateObservation {
            digest: digest_bytes(&material),
            working_tree_clean,
        })
    }

    /// Compatibility helper for callers that need only the stable digest.
    pub async fn local_state_fingerprint(
        &self,
        config: &GitCommandConfig,
    ) -> Result<String, GitError> {
        Ok(self.local_state_observation(config).await?.digest)
    }
}

/// Open an existing local Git working-tree root without taking ownership of it.
/// A subdirectory inside a repository is rejected to keep source-root identity
/// explicit. This function never modifies or deletes the caller's directory.
pub async fn open_local_repository(
    path: impl AsRef<Path>,
    config: &GitCommandConfig,
) -> Result<GitRepository, GitError> {
    let input = path.as_ref();
    let metadata = fs::metadata(input).map_err(|source| GitError::Filesystem {
        path: input.to_path_buf(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(GitError::RepositoryPathNotDirectory(input.to_path_buf()));
    }
    let canonical_input = fs::canonicalize(input).map_err(|source| GitError::Filesystem {
        path: input.to_path_buf(),
        source,
    })?;
    let root = repository_top_level(&canonical_input, config).await?;
    if root != canonical_input {
        return Err(GitError::NotRepositoryRoot(canonical_input));
    }
    let head_commit = resolve_head_commit(&root, config).await?;
    let branch = resolve_current_branch(&root, config).await?;
    let remote_locator = resolve_origin_locator(&root, config).await?;

    Ok(GitRepository {
        root,
        head_commit,
        branch,
        remote_locator,
        owning_operation_id: None,
    })
}

async fn repository_top_level(root: &Path, config: &GitCommandConfig) -> Result<PathBuf, GitError> {
    let args = vec![
        OsString::from("rev-parse"),
        OsString::from("--show-toplevel"),
    ];
    let output = super::command::run_git("repository-root", args, Some(root), config).await?;
    if !output.status.success() {
        return Err(GitError::NotGitRepository(root.to_path_buf()));
    }
    if output.stdout_truncated {
        return Err(GitError::OutputLimitExceeded {
            operation: "repository-root",
            stream: "stdout",
        });
    }
    let output_path = path_from_git_output(output.stdout.as_slice())?;
    fs::canonicalize(&output_path).map_err(|source| GitError::Filesystem {
        path: output_path,
        source,
    })
}

async fn resolve_head_commit(root: &Path, config: &GitCommandConfig) -> Result<String, GitError> {
    let args = vec![
        OsString::from("rev-parse"),
        OsString::from("--verify"),
        OsString::from("HEAD^{commit}"),
    ];
    let output = run_git_checked("head-commit", args, Some(root), config).await?;
    let commit = output_line(&output, "head-commit")?;
    validate_commit_id(commit)
}

async fn resolve_current_branch(
    root: &Path,
    config: &GitCommandConfig,
) -> Result<Option<String>, GitError> {
    let args = vec![OsString::from("branch"), OsString::from("--show-current")];
    let output = run_git_checked("current-branch", args, Some(root), config).await?;
    let branch = output_line(&output, "current-branch")?;
    Ok(if branch.is_empty() {
        None
    } else {
        Some(branch.to_owned())
    })
}

async fn resolve_origin_locator(
    root: &Path,
    config: &GitCommandConfig,
) -> Result<Option<String>, GitError> {
    let args = vec![
        OsString::from("remote"),
        OsString::from("get-url"),
        OsString::from("origin"),
    ];
    let output = super::command::run_git("origin-url", args, Some(root), config).await?;
    if !output.status.success() || output.stdout_truncated {
        return Ok(None);
    }
    let raw = output_line(&output, "origin-url")?;
    let Ok(identity) = RepositoryIdentity::from_remote(raw) else {
        // A local origin containing credentials or an unsupported locator is
        // deliberately ignored rather than persisted or returned to callers.
        return Ok(None);
    };
    Ok(identity.remote_locator().map(str::to_owned))
}

fn clone_branch_name(reference: &str) -> Result<&str, GitError> {
    let branch = if let Some(branch) = reference.strip_prefix("refs/heads/") {
        branch
    } else if let Some(tag) = reference.strip_prefix("refs/tags/") {
        tag
    } else if reference.starts_with("refs/") {
        // `git clone --branch` accepts a short branch/tag name, not arbitrary
        // fully qualified refs such as refs/remotes/origin/... .
        return Err(GitError::InvalidRef);
    } else {
        reference
    };
    super::validate_ref(branch)?;
    Ok(branch)
}

fn path_from_git_output(bytes: &[u8]) -> Result<PathBuf, GitError> {
    // `rev-parse --show-toplevel` terminates its path with one LF. Remove
    // exactly that terminator; filenames may legally contain trailing newlines.
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
    }
    #[cfg(not(unix))]
    {
        let text = std::str::from_utf8(bytes).map_err(|_| GitError::MalformedOutput {
            operation: "repository-root",
        })?;
        Ok(PathBuf::from(text))
    }
}

#[cfg(test)]
mod tests {
    use super::clone_branch_name;

    #[test]
    fn fully_qualified_branch_and_tag_are_simplified_for_clone() {
        assert_eq!(clone_branch_name("refs/heads/main").unwrap(), "main");
        assert_eq!(clone_branch_name("refs/tags/v1.2.0").unwrap(), "v1.2.0");
        assert_eq!(clone_branch_name("main").unwrap(), "main");
        assert!(clone_branch_name("refs/remotes/origin/main").is_err());
    }
}
