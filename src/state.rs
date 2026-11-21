//! Repo-state tree scanning and indexing.
//!
//! Layout: `<root>/<repo>-<arch>/<pkgname>/{PKGBUILD,files...}`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::pkgbuild::{PkgbuildInfo, parse_pkgbuild};
use crate::StateError;

/// A `<repo>-<arch>` pair parsed from a directory name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoArch {
    /// Repository (e.g. `core`, `extra`, `multilib`).
    pub repo: String,
    /// Architecture (e.g. `x86_64`, `any`).
    pub arch: String,
}

impl RepoArch {
    /// Parse `extra-x86_64` → (extra, x86_64); splits on first '-'.
    pub fn parse(dirname: &str) -> Option<Self> {
        let (repo, arch) = dirname.split_once('-')?;
        if repo.is_empty() || arch.is_empty() {
            return None;
        }
        Some(Self {
            repo: repo.to_string(),
            arch: arch.to_string(),
        })
    }

    /// Directory name form.
    pub fn dir_name(&self) -> String {
        format!("{}-{}", self.repo, self.arch)
    }
}

/// One package directory in the state tree.
#[derive(Debug, Clone)]
pub struct PackageEntry {
    /// Repository.
    pub repo: String,
    /// Architecture.
    pub arch: String,
    /// Package directory name (== pkgbase typically).
    pub name: String,
    /// Absolute path.
    pub path: PathBuf,
    /// Parsed PKGBUILD scalar fields.
    pub pkgbuild: PkgbuildInfo,
    /// Files in the package dir (relative paths, sorted).
    pub files: Vec<String>,
}

impl PackageEntry {
    /// Whether a PKGBUILD exists and parsed to a non-empty pkgname.
    pub fn has_pkgbuild(&self) -> bool {
        !self.pkgbuild.pkgname.is_empty()
    }
}

/// Queryable index of a repo-state checkout.
#[derive(Debug, Default)]
pub struct StateIndex {
    /// Packages keyed by `<repo>-<arch>/<name>`.
    pub packages: BTreeMap<String, PackageEntry>,
    /// repo-arch dirs seen.
    pub repo_archs: Vec<RepoArch>,
}

impl StateIndex {
    /// Find a package by repo/arch/name.
    pub fn get(&self, repo: &str, arch: &str, name: &str) -> Option<&PackageEntry> {
        self.packages.get(&format!("{repo}-{arch}/{name}"))
    }

    /// All packages in a repo (any arch).
    pub fn in_repo(&self, repo: &str) -> Vec<&PackageEntry> {
        self.packages
            .values()
            .filter(|p| p.repo == repo)
            .collect()
    }

    /// Total package count.
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// Scan a state checkout root into a [`StateIndex`].
///
/// `root` should contain `<repo>-<arch>/` directories. Package dirs without
/// a PKGBUILD are skipped (e.g. `.gitkeep`-only dirs).
pub fn scan_state(root: &Path) -> Result<StateIndex, StateError> {
    let mut index = StateIndex::default();
    let mut ra_set = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let dirname = entry.file_name().to_string_lossy().to_string();
        let Some(ra) = RepoArch::parse(&dirname) else {
            continue;
        };
        ra_set.insert(ra.clone());
        for pkg_entry in std::fs::read_dir(&dir)? {
            let pkg_entry = pkg_entry?;
            let pkg_dir = pkg_entry.path();
            if !pkg_dir.is_dir() {
                continue;
            }
            let name = pkg_entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let pkgbuild_path = pkg_dir.join("PKGBUILD");
            if !pkgbuild_path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&pkgbuild_path).map_err(|e| {
                StateError::Pkgbuild(pkgbuild_path.display().to_string(), e.to_string())
            })?;
            let pkgbuild = parse_pkgbuild(&text);
            let files = list_files(&pkg_dir)?;
            index.packages.insert(
                format!("{dirname}/{name}"),
                PackageEntry {
                    repo: ra.repo.clone(),
                    arch: ra.arch.clone(),
                    name,
                    path: pkg_dir,
                    pkgbuild,
                    files,
                },
            );
        }
    }
    index.repo_archs = ra_set.into_iter().collect();
    if index.packages.is_empty() && index.repo_archs.is_empty() {
        return Err(StateError::InvalidRoot(format!(
            "{}: no <repo>-<arch> directories found",
            root.display()
        )));
    }
    Ok(index)
}

/// List files in a package dir as sorted relative paths (excludes PKGBUILD).
fn list_files(dir: &Path) -> Result<Vec<String>, StateError> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_file() {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name != "PKGBUILD" {
                    out.push(name.to_string());
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkr(root: &Path, repo_arch: &str, pkg: &str, files: &[&str]) {
        let d = root.join(repo_arch).join(pkg);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("PKGBUILD"),
            format!("pkgname={pkg}\npkgbase={pkg}\npkgver=1.0\npkgrel=1\narch=('x86_64')\n"),
        )
        .unwrap();
        for f in files {
            let p = d.join(f);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&p, "x\n").unwrap();
        }
    }

    #[test]
    fn test_repo_arch_parse() {
        let ra = RepoArch::parse("extra-x86_64").unwrap();
        assert_eq!(ra.repo, "extra");
        assert_eq!(ra.arch, "x86_64");
        assert!(RepoArch::parse("nodash").is_none());
        assert!(RepoArch::parse("-x86_64").is_none());
    }

    #[test]
    fn test_scan_state() {
        let dir = tempfile::tempdir().unwrap();
        mkr(dir.path(), "core-x86_64", "linux", &["config", "patches/fix.patch"]);
        mkr(dir.path(), "extra-any", "bash-completion", &[]);
        let idx = scan_state(dir.path()).unwrap();
        assert_eq!(idx.len(), 2);
        let linux = idx.get("core", "x86_64", "linux").unwrap();
        assert!(linux.has_pkgbuild());
        assert_eq!(linux.pkgbuild.pkgver, "1.0");
        assert_eq!(idx.in_repo("extra").len(), 1);
        assert_eq!(idx.repo_archs.len(), 2);
    }

    #[test]
    fn test_empty_state_root_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_state(dir.path()).is_err());
    }
}
