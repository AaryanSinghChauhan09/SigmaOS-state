//! Minimal PKGBUILD scalar parser (shared semantics with svntogit port).

/// Scalar fields extracted from a PKGBUILD.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PkgbuildInfo {
    /// pkgname (first element if array).
    pub pkgname: String,
    /// pkgbase.
    pub pkgbase: String,
    /// pkgver.
    pub pkgver: String,
    /// pkgrel.
    pub pkgrel: String,
    /// epoch.
    pub epoch: u64,
    /// arch list.
    pub arch: Vec<String>,
}

impl PkgbuildInfo {
    /// Full version string `[epoch:]pkgver-pkgrel`.
    pub fn full_version(&self) -> String {
        let epoch = if self.epoch != 0 {
            format!("{}:", self.epoch)
        } else {
            String::new()
        };
        format!("{}{}-{}", epoch, self.pkgver, self.pkgrel)
    }
}

/// Parse scalar `key=value` assignments from PKGBUILD text.
pub fn parse_pkgbuild(text: &str) -> PkgbuildInfo {
    let mut info = PkgbuildInfo::default();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = strip_comment(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "pkgname" => info.pkgname = first_value(value),
            "pkgbase" => info.pkgbase = unquote(value),
            "pkgver" => info.pkgver = unquote(value),
            "pkgrel" => info.pkgrel = unquote(value),
            "epoch" => info.epoch = unquote(value).parse().unwrap_or(0),
            "arch" => info.arch = array_values(value),
            _ => {}
        }
    }
    info
}

fn strip_comment(line: &str) -> &str {
    for (i, _) in line.match_indices('#') {
        if i == 0 || line.as_bytes()[i - 1].is_ascii_whitespace() {
            return line[..i].trim_end();
        }
    }
    line
}

fn unquote(v: &str) -> String {
    v.trim().trim_matches(|c| c == '"' || c == '\'').to_string()
}

fn first_value(v: &str) -> String {
    let v = v.trim();
    if v.starts_with('(') {
        array_values(v).into_iter().next().unwrap_or_default()
    } else {
        unquote(v)
    }
}

fn array_values(v: &str) -> Vec<String> {
    let v = v.trim();
    if !v.starts_with('(') {
        return vec![unquote(v)];
    }
    let inner = v.trim_start_matches('(').trim_end_matches(')').trim_end_matches(';');
    inner
        .split_whitespace()
        .map(|s| s.trim_matches(|c| c == '"' || c == '\''))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
