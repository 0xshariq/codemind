use std::{env, net::SocketAddr};
use crate::git::GitHistoryOptions;
use anyhow::{Context, Result, anyhow};

pub const DEFAULT_FALKORDB_URL: &str = "redis://default:default@127.0.0.1:6380";

#[derive(Debug, Clone)]
pub struct Config {
    pub engine_addr: SocketAddr,
    pub falkordb_url: String,
    /// Trusted FalkorDB graph-name prefix; the adapter appends a workspace digest.
    pub falkordb_graph: String,
    /// Server-side query timeout in milliseconds for bounded graph operations.
    pub falkordb_query_timeout_ms: i64,
    /// Maximum in-flight commands per multiplexed FalkorDB connection.
    pub falkordb_max_inflight: usize,
    /// Bounded Git history extraction used during repository indexing.
    pub git_history_options: GitHistoryOptions,
    /// Always false in production until generation-staged mutation and
    /// incremental/full-rebuild parity have passed the acceptance gate.
    pub enable_incremental_mutation: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let host = env::var("CODEMIND_ENGINE_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("CODEMIND_ENGINE_PORT")
            .unwrap_or_else(|_| "50051".to_owned())
            .parse::<u16>()
            .context("CODEMIND_ENGINE_PORT must be a valid TCP port")?;
        let falkordb_url =
            env::var("FALKORDB_URL").unwrap_or_else(|_| DEFAULT_FALKORDB_URL.to_owned());
        let falkordb_graph = env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "codemind".to_owned());
        let falkordb_query_timeout_ms = env::var("FALKORDB_QUERY_TIMEOUT_MS")
            .unwrap_or_else(|_| "30000".to_owned())
            .parse::<i64>()
            .context("FALKORDB_QUERY_TIMEOUT_MS must be a positive integer")?;
        let falkordb_max_inflight = env::var("FALKORDB_MAX_INFLIGHT")
            .unwrap_or_else(|_| "64".to_owned())
            .parse::<usize>()
            .context("FALKORDB_MAX_INFLIGHT must be a positive integer")?;
        let git_history_max_commits = env::var("CODEMIND_GIT_MAX_HISTORY_COMMITS")
            .unwrap_or_else(|_| "100".to_owned())
            .parse::<usize>()
            .context("CODEMIND_GIT_MAX_HISTORY_COMMITS must be a positive integer")?;
        let git_history_max_changed_paths_per_commit =
            env::var("CODEMIND_GIT_MAX_PATHS_PER_COMMIT")
                .unwrap_or_else(|_| "5000".to_owned())
                .parse::<usize>()
                .context("CODEMIND_GIT_MAX_PATHS_PER_COMMIT must be a positive integer")?;
        let git_history_max_total_paths = env::var("CODEMIND_GIT_MAX_TOTAL_HISTORY_PATHS")
            .unwrap_or_else(|_| "100000".to_owned())
            .parse::<usize>()
            .context("CODEMIND_GIT_MAX_TOTAL_HISTORY_PATHS must be a positive integer")?;
        // Incremental graph mutation is deliberately hard-disabled until
        // generation-staged writes and incremental/full-rebuild parity have
        // passed the production acceptance gate. The legacy environment flag
        // is ignored so it cannot re-enable the unsafe in-place mutation path.
        let enable_incremental_mutation = false;

        Self::from_values_with_options(
            &host,
            port,
            &falkordb_url,
            &falkordb_graph,
            falkordb_query_timeout_ms,
            falkordb_max_inflight,
            GitHistoryOptions {
                max_commits: git_history_max_commits,
                max_changed_paths_per_commit: git_history_max_changed_paths_per_commit,
                max_total_paths: git_history_max_total_paths,
            },
            enable_incremental_mutation,
        )
    }

    #[cfg(test)]
    fn from_values(
        host: &str,
        port: u16,
        falkordb_url: &str,
        falkordb_graph: &str,
        falkordb_query_timeout_ms: i64,
        falkordb_max_inflight: usize,
    ) -> Result<Self> {
        Self::from_values_with_options(
            host,
            port,
            falkordb_url,
            falkordb_graph,
            falkordb_query_timeout_ms,
            falkordb_max_inflight,
            GitHistoryOptions::default(),
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_values_with_options(
        host: &str,
        port: u16,
        falkordb_url: &str,
        falkordb_graph: &str,
        falkordb_query_timeout_ms: i64,
        falkordb_max_inflight: usize,
        git_history_options: GitHistoryOptions,
        enable_incremental_mutation: bool,
    ) -> Result<Self> {
        if host.trim().is_empty() {
            return Err(anyhow!("CODEMIND_ENGINE_HOST must not be empty"));
        }
        if port == 0 {
            return Err(anyhow!("CODEMIND_ENGINE_PORT must be greater than zero"));
        }
        let engine_addr = format!("{host}:{port}")
            .parse()
            .with_context(|| format!("invalid engine address {host}:{port}"))?;
        if !matches!(
            falkordb_url.split_once("://").map(|(scheme, _)| scheme),
            Some("redis" | "rediss")
        ) {
            return Err(anyhow!("FALKORDB_URL must use redis:// or rediss://"));
        }
        if falkordb_url.ends_with("://") {
            return Err(anyhow!("FALKORDB_URL must include a host"));
        }
        if falkordb_graph.trim().is_empty() {
            return Err(anyhow!("FALKORDB_GRAPH namespace prefix must not be empty"));
        }
        if falkordb_query_timeout_ms <= 0 {
            return Err(anyhow!(
                "FALKORDB_QUERY_TIMEOUT_MS must be greater than zero"
            ));
        }
        if falkordb_max_inflight == 0 {
            return Err(anyhow!("FALKORDB_MAX_INFLIGHT must be greater than zero"));
        }
        if git_history_options.max_commits == 0
            || git_history_options.max_commits > 1_000
            || git_history_options.max_changed_paths_per_commit == 0
            || git_history_options.max_changed_paths_per_commit > 50_000
            || git_history_options.max_total_paths == 0
            || git_history_options.max_total_paths > 100_000
        {
            return Err(anyhow!(
                "Git history bounds are outside the supported engine limits"
            ));
        }
        if !falkordb_graph.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        }) {
            return Err(anyhow!(
                "FALKORDB_GRAPH namespace may contain only ASCII letters, numbers, '_' or '-'"
            ));
        }

        Ok(Self {
            engine_addr,
            falkordb_url: falkordb_url.to_owned(),
            falkordb_graph: falkordb_graph.to_owned(),
            falkordb_query_timeout_ms,
            falkordb_max_inflight,
            git_history_options,
            enable_incremental_mutation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn default_falkordb_url_uses_the_local_compose_credentials() {
        assert_eq!(
            super::DEFAULT_FALKORDB_URL,
            "redis://default:default@127.0.0.1:6380"
        );
    }

    #[test]
    fn defaults_are_valid_for_local_development() {
        let config = Config::from_values(
            "127.0.0.1",
            50051,
            super::DEFAULT_FALKORDB_URL,
            "codemind",
            30_000,
            64,
        )
        .expect("local defaults should be valid");
        assert_eq!(config.engine_addr.port(), 50051);
        assert_eq!(config.falkordb_graph, "codemind");
        assert_eq!(config.falkordb_query_timeout_ms, 30_000);
        assert_eq!(config.falkordb_max_inflight, 64);
        assert!(!config.enable_incremental_mutation);
    }

    #[test]
    fn rejects_invalid_values_without_environment_mutation() {
        assert!(
            Config::from_values(
                "",
                50051,
                super::DEFAULT_FALKORDB_URL,
                "codemind",
                30_000,
                64
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "127.0.0.1",
                0,
                super::DEFAULT_FALKORDB_URL,
                "codemind",
                30_000,
                64
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "127.0.0.1",
                50051,
                "http://127.0.0.1:6380",
                "codemind",
                30_000,
                64
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "127.0.0.1",
                50051,
                super::DEFAULT_FALKORDB_URL,
                "bad name",
                30_000,
                64
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "127.0.0.1",
                50051,
                super::DEFAULT_FALKORDB_URL,
                "codemind-prod",
                30_000,
                64
            )
            .is_ok()
        );
        assert!(
            Config::from_values(
                "127.0.0.1",
                50051,
                super::DEFAULT_FALKORDB_URL,
                "codemind",
                0,
                64
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "127.0.0.1",
                50051,
                super::DEFAULT_FALKORDB_URL,
                "codemind",
                -1,
                64
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "127.0.0.1",
                50051,
                super::DEFAULT_FALKORDB_URL,
                "codemind",
                30_000,
                0
            )
            .is_err()
        );
    }

    #[test]
    fn git_history_options_validate_each_supported_boundary() {
        use crate::git::GitHistoryOptions;

        let base = GitHistoryOptions::default();
        let build = |options| {
            Config::from_values_with_options(
                "127.0.0.1",
                50051,
                super::DEFAULT_FALKORDB_URL,
                "codemind",
                30_000,
                64,
                options,
                false,
            )
        };

        assert!(build(base).is_ok());
        assert!(
            build(GitHistoryOptions {
                max_commits: 0,
                ..base
            })
            .is_err()
        );
        assert!(
            build(GitHistoryOptions {
                max_commits: 1_001,
                ..base
            })
            .is_err()
        );
        assert!(
            build(GitHistoryOptions {
                max_changed_paths_per_commit: 0,
                ..base
            })
            .is_err()
        );
        assert!(
            build(GitHistoryOptions {
                max_changed_paths_per_commit: 50_001,
                ..base
            })
            .is_err()
        );
        assert!(
            build(GitHistoryOptions {
                max_total_paths: 0,
                ..base
            })
            .is_err()
        );
        assert!(
            build(GitHistoryOptions {
                max_total_paths: 100_001,
                ..base
            })
            .is_err()
        );
        assert!(
            build(GitHistoryOptions {
                max_commits: 1_000,
                max_changed_paths_per_commit: 50_000,
                max_total_paths: 100_000,
            })
            .is_ok()
        );
    }
}
