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
        let engine_addr = format!("{host}:{port}")
            .parse()
            .with_context(|| format!("invalid engine address {host}:{port}"))?;

        let falkordb_url =
            env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_owned());
        if !falkordb_url.starts_with("redis://") && !falkordb_url.starts_with("rediss://") {
            return Err(anyhow!("FALKORDB_URL must use redis:// or rediss://"));
        }

        let falkordb_graph = env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "codemind".to_owned());
        if falkordb_graph.trim().is_empty() {
            return Err(anyhow!("FALKORDB_GRAPH must not be empty"));
        }

        Ok(Self {
            engine_addr,
            falkordb_url,
            falkordb_graph,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn defaults_are_valid_for_local_development() {
        let config = Config::from_env().expect("local defaults should be valid");
        assert_eq!(config.engine_addr.port(), 50051);
        assert_eq!(config.falkordb_graph, "codemind");
    }
}
