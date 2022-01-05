#![warn(rust_2018_idioms)]
#![allow(unused_imports)]
#![allow(clippy::blacklisted_name)]
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Kube Api Error: {0}")]
    KubeError(#[source] kube::Error),

    #[error("SerializationError: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

// api
pub mod api;
// Generated type, for crdgen
pub use api::journal::Journal;

/// State machinery for kube, as exposeable to actix
pub mod operator;
pub use operator::journal::Manager;

/// Log and trace integrations
pub mod telemetry;
