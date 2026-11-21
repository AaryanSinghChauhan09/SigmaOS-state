//! Integration tests for sigmaos-state.

use std::path::Path;

use state::{PackageEntry, RepoArch, scan_state};

fn mkr(root: &Path, repo_arch: &str, pkg: &str, extra_files: &[&str]) {
    let d = root.join(repo_arch).join(pkg);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("PKGBUILD"),
        format!("pkgname={pkg}\npkgbase={pkg}\npkgver=2.1\npkgrel=3\narch=('x86_64' 'any')\n"),
    )
    .unwrap();
    for f in extra_files {
        let p = d.join(f);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, "data\n").unwrap();
    }
}

#[test]
fn test_scan_full_state_tree() {
    let dir = tempfile::tempdir().unwrap();
    mkr(dir.path(), "core-x86_64", "linux", &["config", "patches/a.patch"]);
    mkr(dir.path(), "extra-x86_64", "firefox", &["firefox.desktop"]);
    mkr(dir.path(), "multilib-x86_64", "lib32-glibc", &[]);
    // Non-package dirs are ignored.
    std::fs::create_dir_all(dir.path().join("extra-x86_64/not-a-pkg")).unwrap();
    std::fs::write(dir.path().join("README"), "x\n").unwrap();

    let idx = scan_state(dir.path()).unwrap();
    assert_eq!(idx.len(), 3);
    assert_eq!(idx.repo_archs.len(), 3);

    let linux = idx.get("core", "x86_64", "linux").unwrap();
    assert_eq!(linux.pkgbuild.full_version(), "2.1-3");
    assert!(linux.files.contains(&"config".to_string()));
    assert!(!linux.files.contains(&"PKGBUILD".to_string()));

    assert_eq!(idx.in_repo("extra").len(), 1);
    assert!(idx.get("extra", "x86_64", "missing").is_none());
}

#[test]
fn test_pkgbuild_metadata_extraction() {
    let dir = tempfile::tempdir().unwrap();
    mkr(dir.path(), "extra-any", "shell-completions", &[]);
    let idx = scan_state(dir.path()).unwrap();
    let e = idx.get("extra", "any", "shell-completions").unwrap();
    assert_eq!(e.pkgbuild.arch, vec!["x86_64", "any"]);
}

#[test]
fn test_repo_arch_dir_name_roundtrip() {
    let ra = RepoArch::parse("multilib-x86_64").unwrap();
    assert_eq!(ra.dir_name(), "multilib-x86_64");
}

#[test]
fn test_state_index_queries() {
    let dir = tempfile::tempdir().unwrap();
    mkr(dir.path(), "core-x86_64", "a", &[]);
    mkr(dir.path(), "core-any", "b", &[]);
    let idx = scan_state(dir.path()).unwrap();
    assert_eq!(idx.len(), 2);
    assert_eq!(idx.in_repo("core").len(), 2);
    assert!(idx.is_empty() == false);
}

#[test]
fn test_package_entry_display_fields() {
    let e = PackageEntry {
        repo: "extra".into(),
        arch: "x86_64".into(),
        name: "x".into(),
        path: Path::new("/tmp/x").to_path_buf(),
        pkgbuild: state::PkgbuildInfo {
            pkgname: "x".into(),
            ..Default::default()
        },
        files: vec![],
    };
    assert!(e.has_pkgbuild());
}
