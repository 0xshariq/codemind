#![recursion_limit = "256"]

pub mod config;
pub mod engines;
pub mod graph_storage;
pub mod lifecycle;
pub mod memory;
pub mod source;
pub mod transport;

pub mod contract {
    pub mod generated {
        tonic::include_proto!("codemind.v1");
    }
    pub mod result;
}
