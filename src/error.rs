//! Error types for the state component.

/// Errors that can occur while scanning package repo state.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// The state root does not contain `<repo>-<arch>` directories.
    #[error("invalid state root: {0}")]
    InvalidRoot(String),

    /// A PKGBUILD could not be read or parsed.
    #[error("PKGBUILD error in {0}: {1}")]
    Pkgbuild(String, String),

    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
