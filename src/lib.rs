//! SigmaOS state component.
//!
//! Rust implementation of `archlinux/state`: the git packaging repository
//! state that holds every official Arch package as
//! `<repo>-<arch>/<pkgname>/PKGBUILD` plus package files.
//!
//! This port implements the read side of that state: scanning a checkout
//! into a queryable package index (repo, arch, pkgbase, files, PKGBUILD
//! metadata) — the same data `dbscripts`/`devtools` consume upstream.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod pkgbuild;
pub mod state;

pub use error::StateError;
pub use pkgbuild::PkgbuildInfo;
pub use state::{PackageEntry, RepoArch, StateIndex, scan_state};
