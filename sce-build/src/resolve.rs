// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Single source of truth for `<xi:include>` / `<sce:use>` fragment
// path resolution. Both preprocessors — [`crate::xinclude`] and
// [`crate::template`] — resolve a referenced fragment with the same
// search-path precedence, so the precedence lives in one function and
// each caller maps the miss-trail to its own diagnostic variant.
//
// The C++ runtime mirrors this exact precedence in
// `sce/include/parsing/FragmentResolver.h::resolveFragment`; the two
// implementations are kept byte-equivalent by the template-parity
// harness (`tests/w3c_template_parity/`), which drives include-dir
// fixtures through both engines and diffs the canonicalised output.

use std::path::{Path, PathBuf};

/// Resolve `name` to an existing file using the SCE fragment search
/// precedence: absolute → including-file base directory →
/// operator-configured include directories (in declaration order) →
/// current working directory.
///
/// `base_dir` is the directory a relative `name` is first resolved
/// against (typically the including file's parent). `extra_dirs` is
/// the `--include-dir` / `-I` search path, tried after `base_dir` and
/// before the implicit cwd fallback so an explicit search path always
/// wins over the cwd guess. Empty `extra_dirs` resolves exactly as
/// `absolute → base → cwd`.
///
/// Returns the resolved path on the first hit. On a miss, returns the
/// ordered list of paths that were tried so the caller can render its
/// own `NotFound` diagnostic trail (XInclude and template expansion
/// surface different diagnostic codes for the same physical miss).
pub(crate) fn resolve_fragment(
    name: &str,
    base_dir: Option<&Path>,
    extra_dirs: &[PathBuf],
) -> Result<PathBuf, Vec<String>> {
    let path = Path::new(name);
    let mut tried: Vec<String> = Vec::new();

    if path.is_absolute() {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        tried.push(path.display().to_string());
        return Err(tried);
    }

    if let Some(base) = base_dir {
        let candidate = base.join(path);
        if candidate.exists() {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }
    for dir in extra_dirs {
        let candidate = dir.join(path);
        if candidate.exists() {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }
    if path.exists() {
        return Ok(path.to_path_buf());
    }
    tried.push(path.display().to_string());
    Err(tried)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).expect("tempfile write");
        path
    }

    #[test]
    fn precedence_base_before_include_before_cwd() {
        let base = TempDir::new().unwrap();
        let inc = TempDir::new().unwrap();

        // Only in the include dir: resolves there after base misses.
        write(inc.path(), "only_inc.xml", "x");
        let hit = resolve_fragment(
            "only_inc.xml",
            Some(base.path()),
            &[inc.path().to_path_buf()],
        )
        .expect("include-dir hit");
        assert_eq!(hit, inc.path().join("only_inc.xml"));

        // In both base and include dir: base wins.
        write(base.path(), "both.xml", "base");
        write(inc.path(), "both.xml", "inc");
        let hit = resolve_fragment("both.xml", Some(base.path()), &[inc.path().to_path_buf()])
            .expect("base hit");
        assert_eq!(hit, base.path().join("both.xml"));
    }

    #[test]
    fn miss_returns_ordered_trail() {
        let base = TempDir::new().unwrap();
        let inc = TempDir::new().unwrap();
        let trail = resolve_fragment("ghost.xml", Some(base.path()), &[inc.path().to_path_buf()])
            .expect_err("miss");
        // base candidate first, then include-dir candidate, then cwd.
        assert_eq!(trail.len(), 3);
        assert!(trail[0].contains("ghost.xml"));
        assert!(trail[1].starts_with(&inc.path().display().to_string()));
    }

    #[test]
    fn absolute_miss_does_not_consult_search_path() {
        let inc = TempDir::new().unwrap();
        let abs = inc.path().join("nonexistent_abs.xml");
        let trail = resolve_fragment(
            abs.to_str().unwrap(),
            Some(inc.path()),
            &[inc.path().to_path_buf()],
        )
        .expect_err("absolute miss");
        // Absolute paths are resolved as-is: exactly one entry, no
        // base/include/cwd fallback.
        assert_eq!(trail.len(), 1);
    }
}
