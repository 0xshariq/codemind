use std::{env, net::SocketAddr};

use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone)]
pub struct Config {
    pub engine_addr: SocketAddr,
    pub falkordb_url: String,
    pub falkordb_graph: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let host = env::var("CODEMIND_ENGINE_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("CODEMIND_ENGINE_PORT")
            .unwrap_or_else(|_| "50051".to_owned())
            .parse::<u16>()
            .context("CODEMIND_ENGINE_PORT must be a valid TCP port")?;
        let falkordb_url =
            env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_owned());
        let falkordb_graph = env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "codemind".to_owned());
        Self::from_values(&host, port, &falkordb_url, &falkordb_graph)
    }

    fn from_values(
        host: &str,
        port: u16,
        falkordb_url: &str,
        falkordb_graph: &str,
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
            return Err(anyhow!("FALKORDB_GRAPH must not be empty"));
        }
        if !falkordb_graph.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        }) {
            return Err(anyhow!(
                "FALKORDB_GRAPH may contain only ASCII letters, numbers, '_' or '-'"
            ));
        }

        Ok(Self {
            engine_addr,
            falkordb_url: falkordb_url.to_owned(),
            falkordb_graph: falkordb_graph.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn defaults_are_valid_for_local_development() {
        let config = Config::from_values("127.0.0.1", 50051, "redis://127.0.0.1:6380", "codemind")
            .expect("local defaults should be valid");
        assert_eq!(config.engine_addr.port(), 50051);
        assert_eq!(config.falkordb_graph, "codemind");
    }

    #[test]
    fn rejects_invalid_values_without_environment_mutation() {
        assert!(Config::from_values("", 50051, "redis://127.0.0.1:6380", "codemind").is_err());
        assert!(Config::from_values("127.0.0.1", 0, "redis://127.0.0.1:6380", "codemind").is_err());
        assert!(
            Config::from_values("127.0.0.1", 50051, "http://127.0.0.1:6380", "codemind").is_err()
        );
        assert!(
            Config::from_values("127.0.0.1", 50051, "redis://127.0.0.1:6380", "bad name").is_err()
        );
    }
}
