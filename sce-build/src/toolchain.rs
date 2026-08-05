// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Single source of truth for locating the external toolchain binaries
//! the generated-code verification harness compiles with.
//!
//! The harness proves that emitted artefacts survive a real consumer
//! toolchain — `clang -std=c11 -Werror`, `g++ -std=c++17`, `rustc`,
//! `kotlinc`, `gofmt`, `python3`. A tool that cannot be located is
//! skipped, so *where* this module looks decides what is actually
//! verified. That makes the search space a correctness concern, not a
//! convenience: every directory omitted here is a check that silently
//! reports success without running.
//!
//! Before this module each caller ran `which <name>` and treated a
//! non-zero exit as "not installed". `PATH` is the wrong question.
//! Distributions routinely install a toolchain into a versioned
//! directory and ship no unversioned symlink:
//!
//! * Debian / Ubuntu place Clang in `/usr/lib/llvm-<major>/bin` and
//!   provide `/usr/bin/clang` only when the `clang` metapackage is
//!   installed. `llvm-19` alone leaves `which clang` empty while Clang
//!   19 is present and working.
//! * Homebrew keeps LLVM keg-only under `<prefix>/opt/llvm@<major>/bin`
//!   and deliberately does not link it into `PATH`.
//!
//! On such a host the PATH-only lookup reported "no Clang" and every
//! Clang-gated test skipped. Skips read as green, so the verification
//! surface differed between two machines with the same packages
//! installed, and defects that only Clang diagnoses (`-Wnewline-eof`
//! among them) stayed invisible.
//!
//! # Search order
//!
//! [`ToolLocator::locate`] applies four strategies in order and returns
//! the first hit:
//!
//! 1. An explicit override, `SCE_TOOL_<NAME>` (see
//!    [`override_var_for`]). An override that does not resolve to an
//!    executable is an error, never a fallthrough — a mistyped path
//!    must not silently degrade into "some other compiler".
//! 2. The exact name in a `PATH` directory. Standard behaviour, and it
//!    stays first among the discovery strategies so a caller's `PATH`
//!    always outranks anything this module infers.
//! 3. A version-suffixed name in a `PATH` directory (`clang-19`),
//!    highest version first.
//! 4. A versioned install directory (see [`VERSIONED_BIN_DIRS`]),
//!    highest version first, exact name then version-suffixed name.
//!
//! Version ordering is numeric, not lexical: sorting directory names as
//! text puts `llvm-9` above `llvm-19` and would pick a six-release-old
//! compiler on any host that has both. Directory scans are sorted
//! before use so the result does not depend on `readdir` order.
//!
//! # Absence is a result, not a silence
//!
//! Skipping on a missing tool is the right default for a contributor
//! workstation, but it makes coverage unfalsifiable in CI: a job whose
//! image lost its Clang keeps passing. Setting [`REQUIRE_TOOLS_VAR`]
//! promotes every miss into a hard failure, so a CI lane can assert
//! that the checks it is named for actually ran. See
//! [`ToolLocator::require_or_skip`].
//!
//! # Platform
//!
//! Unix only, matching the `which`-based callers this module replaces.
//! On other platforms discovery is limited to strategies 1 and 2 with
//! no executable-bit test.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Environment variable that promotes a missing tool from a skip into a
/// hard failure. Any value other than `0`, `false`, or the empty string
/// enables it.
pub const REQUIRE_TOOLS_VAR: &str = "SCE_REQUIRE_TOOLS";

/// Prefix for per-tool path overrides. See [`override_var_for`].
pub const OVERRIDE_VAR_PREFIX: &str = "SCE_TOOL_";

/// Directories where a distribution may install a versioned toolchain
/// that is deliberately absent from `PATH`, as
/// `(parent, family, bin_subdir)`.
///
/// `family` is the bare directory name without any version separator:
/// `llvm` matches `llvm`, `llvm-19`, and Homebrew's `llvm@19` alike.
///
/// Entries are probed in listed order; within one entry, candidate
/// directories are ordered by trailing version, highest first. A
/// directory whose name carries no version (Homebrew's `llvm`, which
/// tracks the current stable release) sorts ahead of every versioned
/// sibling.
///
/// This list is LLVM-shaped because Clang is the only toolchain in this
/// repository's harness that distributions install off-`PATH`: measured
/// across the thirteen tools the harness resolves (`g++`, `gcc`, `cc`,
/// `clang`, `clang++`, `gofmt`, `kotlinc`, `rustc`, `go`, `python3`,
/// `python`, `cargo`, `arm-none-eabi-gcc`), Clang and `clang++` were
/// the only ones a versioned-directory install hides. Go, Kotlin, and
/// Rust ship through version managers that do link into `PATH`. Adding
/// a family here is a one-line change when that stops being true.
pub const VERSIONED_BIN_DIRS: &[(&str, &str, &str)] = &[
    // Debian, Ubuntu, and derivatives: apt.llvm.org and the distro
    // packages both use this layout.
    ("/usr/lib", "llvm", "bin"),
    // Multilib RPM distributions.
    ("/usr/lib64", "llvm", "bin"),
    // Homebrew on Apple Silicon and on Intel, keg-only LLVM.
    ("/opt/homebrew/opt", "llvm", "bin"),
    ("/usr/local/opt", "llvm", "bin"),
];

/// Name of the environment variable that overrides the path used for
/// `name`.
///
/// The tool name is upper-cased, `+` becomes `X` (so `g++` reads as the
/// conventional `GXX`), and every other non-alphanumeric character
/// becomes `_`:
///
/// ```
/// use sce_build::toolchain::override_var_for;
/// assert_eq!(override_var_for("clang"), "SCE_TOOL_CLANG");
/// assert_eq!(override_var_for("g++"), "SCE_TOOL_GXX");
/// assert_eq!(override_var_for("arm-none-eabi-gcc"), "SCE_TOOL_ARM_NONE_EABI_GCC");
/// ```
pub fn override_var_for(name: &str) -> String {
    let mut var = String::with_capacity(OVERRIDE_VAR_PREFIX.len() + name.len());
    var.push_str(OVERRIDE_VAR_PREFIX);
    for ch in name.chars() {
        match ch {
            '+' => var.push('X'),
            c if c.is_ascii_alphanumeric() => var.push(c.to_ascii_uppercase()),
            _ => var.push('_'),
        }
    }
    var
}

/// True when `path` names a file this process could execute.
#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// How a directory entry relates to a toolchain family name.
///
/// Both search levels ask the same question — "is this `clang`, or
/// `clang-19`?" for binaries and "is this `llvm`, or `llvm-19`?" for
/// install directories — so they share one matcher and one ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FamilyMatch {
    /// Bare family name, e.g. `clang` or Homebrew's `llvm`.
    Unversioned,
    /// Family name plus an explicit major version, e.g. `clang-19`.
    Versioned(u32),
}

impl FamilyMatch {
    /// Sort key, higher first. An unversioned name tracks the current
    /// stable release on every distribution that ships one, so it
    /// outranks explicitly-versioned siblings.
    fn rank(self) -> u32 {
        match self {
            Self::Unversioned => u32::MAX,
            Self::Versioned(major) => major,
        }
    }
}

/// Classify `name` against `family`, accepting any of `separators`
/// between the family name and its version digits. Debian writes
/// `clang-19` and `llvm-19`; Homebrew writes `llvm@19`.
///
/// Returns `None` when `name` is not a member of the family, which
/// includes near neighbours like `clangd` and `clang-format`.
fn match_family(name: &str, family: &str, separators: &[char]) -> Option<FamilyMatch> {
    let rest = name.strip_prefix(family)?;
    if rest.is_empty() {
        return Some(FamilyMatch::Unversioned);
    }
    let digits = rest.strip_prefix(separators)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().map(FamilyMatch::Versioned)
}

/// Members of `family` directly inside `dir`, best candidate first.
///
/// The sort is total — rank descending, then name ascending — so the
/// result never depends on the order `readdir` returned entries in.
fn family_members_in(dir: &Path, family: &str, separators: &[char]) -> Vec<(FamilyMatch, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut matches: Vec<(FamilyMatch, String, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let file_name = entry.file_name();
            let name = file_name.to_str()?;
            let matched = match_family(name, family, separators)?;
            Some((matched, name.to_string(), entry.path()))
        })
        .collect();
    matches.sort_by(|a, b| b.0.rank().cmp(&a.0.rank()).then_with(|| a.1.cmp(&b.1)));
    matches
        .into_iter()
        .map(|(matched, _, path)| (matched, path))
        .collect()
}

/// Version-suffixed binaries named after `stem` in `dir`, best first.
/// Excludes the exact name, which every caller probes ahead of this.
fn version_suffixed_binaries_in(dir: &Path, stem: &str) -> Vec<PathBuf> {
    family_members_in(dir, stem, &['-'])
        .into_iter()
        .filter(|(matched, _)| matches!(matched, FamilyMatch::Versioned(_)))
        .map(|(_, path)| path)
        .collect()
}

/// Resolved search space for external tools.
///
/// Construct with [`ToolLocator::from_env`] in production and with
/// [`ToolLocator::over`] in tests, where an explicit search space makes
/// discovery deterministic and independent of the host's installs.
#[derive(Debug, Clone, Default)]
pub struct ToolLocator {
    path_dirs: Vec<PathBuf>,
    versioned_bin_dirs: Vec<PathBuf>,
    overrides: BTreeMap<String, PathBuf>,
    require_tools: bool,
}

impl ToolLocator {
    /// Build a locator from the ambient environment: `PATH`, the
    /// `SCE_TOOL_*` overrides, [`VERSIONED_BIN_DIRS`], and
    /// [`REQUIRE_TOOLS_VAR`].
    pub fn from_env() -> Self {
        let path_dirs = std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default();

        let overrides = std::env::vars()
            .filter_map(|(key, value)| {
                let stripped = key.strip_prefix(OVERRIDE_VAR_PREFIX)?;
                (!stripped.is_empty() && !value.is_empty()).then(|| (key, PathBuf::from(value)))
            })
            .collect();

        Self {
            path_dirs,
            versioned_bin_dirs: Self::discover_versioned_bin_dirs(VERSIONED_BIN_DIRS),
            overrides,
            require_tools: Self::require_tools_from_env(),
        }
    }

    /// Build a locator over an explicit search space. `overrides` is
    /// keyed by environment-variable name, matching
    /// [`override_var_for`], so a test exercises the same lookup the
    /// environment drives.
    pub fn over(
        path_dirs: Vec<PathBuf>,
        versioned_dir_specs: &[(&str, &str, &str)],
        overrides: BTreeMap<String, PathBuf>,
    ) -> Self {
        Self {
            path_dirs,
            versioned_bin_dirs: Self::discover_versioned_bin_dirs(versioned_dir_specs),
            overrides,
            require_tools: false,
        }
    }

    /// Promote misses into failures for this locator, as
    /// [`REQUIRE_TOOLS_VAR`] does globally.
    #[must_use]
    pub fn requiring_tools(mut self, require: bool) -> Self {
        self.require_tools = require;
        self
    }

    /// Whether a miss is a failure rather than a skip.
    pub fn requires_tools(&self) -> bool {
        self.require_tools
    }

    fn require_tools_from_env() -> bool {
        std::env::var(REQUIRE_TOOLS_VAR)
            .map(|raw| !matches!(raw.trim(), "" | "0" | "false" | "FALSE" | "False"))
            .unwrap_or(false)
    }

    /// Expand the `(parent, prefix, bin_subdir)` specs into concrete
    /// existing directories, best candidate first.
    fn discover_versioned_bin_dirs(specs: &[(&str, &str, &str)]) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        for (parent, family, bin_subdir) in specs {
            // A distro directory is `llvm-19`; a Homebrew keg is
            // `llvm@19` or the unversioned `llvm`.
            dirs.extend(
                family_members_in(Path::new(parent), family, &['-', '@'])
                    .into_iter()
                    .map(|(_, dir)| dir.join(bin_subdir))
                    .filter(|bin| bin.is_dir()),
            );
        }
        dirs
    }

    /// Locate `name`, or `None` when no strategy finds it.
    ///
    /// # Panics
    ///
    /// When `SCE_TOOL_<NAME>` is set but does not name an executable.
    /// An explicit override that silently falls through to a different
    /// compiler would make the setting worse than useless.
    pub fn locate(&self, name: &str) -> Option<PathBuf> {
        let var = override_var_for(name);
        if let Some(path) = self.overrides.get(&var) {
            assert!(
                is_executable_file(path),
                "{var} is set to {} but that path is not an executable file. \
                 Unset it or point it at a real {name} binary — an override \
                 that falls through would silently verify against a \
                 different toolchain than the one named.",
                path.display(),
            );
            return Some(path.clone());
        }

        // A caller's PATH outranks anything inferred below it.
        for dir in &self.path_dirs {
            let candidate = dir.join(name);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }

        for dir in &self.path_dirs {
            if let Some(hit) = version_suffixed_binaries_in(dir, name)
                .into_iter()
                .find(|path| is_executable_file(path))
            {
                return Some(hit);
            }
        }

        for dir in &self.versioned_bin_dirs {
            let exact = dir.join(name);
            if is_executable_file(&exact) {
                return Some(exact);
            }
            if let Some(hit) = version_suffixed_binaries_in(dir, name)
                .into_iter()
                .find(|path| is_executable_file(path))
            {
                return Some(hit);
            }
        }

        None
    }

    /// Locate the first of `names` that resolves. Use for interchangeable
    /// tools, e.g. `&["gcc", "cc"]` or `&["python3", "python"]`.
    pub fn locate_any(&self, names: &[&str]) -> Option<PathBuf> {
        names.iter().find_map(|name| self.locate(name))
    }

    /// Locate `name`, or `None` after printing why the caller is about
    /// to skip. `purpose` completes the sentence "skipped — no `<name>`
    /// … required to `<purpose>`".
    ///
    /// # Panics
    ///
    /// When tools are required (see [`REQUIRE_TOOLS_VAR`]) and `name`
    /// cannot be located.
    pub fn require_or_skip(&self, name: &str, purpose: &str) -> Option<PathBuf> {
        self.require_any_or_skip(&[name], purpose)
    }

    /// [`Self::require_or_skip`] over interchangeable tool names.
    ///
    /// # Panics
    ///
    /// When tools are required and none of `names` can be located.
    pub fn require_any_or_skip(&self, names: &[&str], purpose: &str) -> Option<PathBuf> {
        if let Some(path) = self.locate_any(names) {
            return Some(path);
        }
        let wanted = names.join(" or ");
        assert!(
            !self.require_tools,
            "{REQUIRE_TOOLS_VAR} is set but no {wanted} could be located, \
             so the check that would {purpose} cannot run. Searched PATH, \
             version-suffixed names, and {} versioned install \
             director{}. Install it, or point {} at it.",
            self.versioned_bin_dirs.len(),
            if self.versioned_bin_dirs.len() == 1 {
                "y"
            } else {
                "ies"
            },
            override_var_for(names[0]),
        );
        eprintln!(
            "skipped — no {wanted} found, so the check that would {purpose} did not run. \
             Set {} to point at one, or set {REQUIRE_TOOLS_VAR}=1 to make this a failure.",
            override_var_for(names[0]),
        );
        None
    }
}

/// Locate `name` using the ambient environment.
///
/// Prefer building one [`ToolLocator::from_env`] per test when
/// resolving several tools; this convenience re-reads the environment
/// on every call.
pub fn locate(name: &str) -> Option<PathBuf> {
    ToolLocator::from_env().locate(name)
}

/// [`locate`] over interchangeable tool names.
pub fn locate_any(names: &[&str]) -> Option<PathBuf> {
    ToolLocator::from_env().locate_any(names)
}

/// Record that a check is being skipped because a tool is missing.
///
/// Callers that already phrase their own reason use this instead of
/// [`require_or_skip`]: it keeps their wording and still routes the
/// skip through the one place that knows whether skips are permitted.
/// `reason` completes the sentence "SKIP `<reason>`", e.g.
/// `"smoke_c11: no clang or gcc"`.
///
/// # Panics
///
/// When [`REQUIRE_TOOLS_VAR`] is set. A skipped check is an unrun
/// check; a lane that declares its tools present must fail rather than
/// report success for work it did not do.
pub fn skipped(reason: &str) {
    assert!(
        !ToolLocator::from_env().requires_tools(),
        "{REQUIRE_TOOLS_VAR} is set, but this check skipped: {reason}. \
         Install the missing tool, point the matching {OVERRIDE_VAR_PREFIX}* \
         variable at it, or unset {REQUIRE_TOOLS_VAR}.",
    );
    eprintln!("SKIP {reason}");
}

/// Locate `name` from the ambient environment, or skip informatively.
///
/// # Panics
///
/// When [`REQUIRE_TOOLS_VAR`] is set and `name` cannot be located.
pub fn require_or_skip(name: &str, purpose: &str) -> Option<PathBuf> {
    ToolLocator::from_env().require_or_skip(name, purpose)
}

/// [`require_or_skip`] over interchangeable tool names.
///
/// # Panics
///
/// When [`REQUIRE_TOOLS_VAR`] is set and none of `names` can be located.
pub fn require_any_or_skip(names: &[&str], purpose: &str) -> Option<PathBuf> {
    ToolLocator::from_env().require_any_or_skip(names, purpose)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create `dir/name` with the executable bit set.
    fn touch_exe(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).expect("create fixture dir");
        let path = dir.join(name);
        std::fs::write(&path, b"#!/bin/sh\n").expect("write fixture binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fixture binary");
        }
        path
    }

    /// Private fixture tree, removed when the returned handle drops.
    /// Every case builds its own so no test inherits another's layout
    /// and none depends on what this host has installed.
    fn fixture_root(case: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("sce-toolchain-{case}-"))
            .tempdir()
            .expect("create fixture root")
    }

    #[test]
    fn override_var_names_follow_the_documented_normalisation() {
        assert_eq!(override_var_for("clang"), "SCE_TOOL_CLANG");
        assert_eq!(override_var_for("clang++"), "SCE_TOOL_CLANGXX");
        assert_eq!(override_var_for("g++"), "SCE_TOOL_GXX");
        assert_eq!(override_var_for("python3"), "SCE_TOOL_PYTHON3");
        assert_eq!(
            override_var_for("arm-none-eabi-gcc"),
            "SCE_TOOL_ARM_NONE_EABI_GCC",
        );
    }

    /// The defect this module exists for: Clang installed only under a
    /// versioned directory, exactly as Debian's `llvm-19` package
    /// leaves it. A `PATH`-only lookup reports "not installed" and
    /// every Clang-gated check skips.
    #[test]
    fn finds_a_tool_installed_only_in_a_versioned_directory() {
        let fixture = fixture_root("versioned_only");
        let root = fixture.path();
        let empty_path_dir = root.join("bin");
        std::fs::create_dir_all(&empty_path_dir).expect("create empty PATH dir");
        let lib = root.join("lib");
        touch_exe(&lib.join("llvm-19").join("bin"), "clang");

        let path_only = ToolLocator::over(vec![empty_path_dir.clone()], &[], BTreeMap::new());
        assert_eq!(
            path_only.locate("clang"),
            None,
            "fixture precondition: clang must not be on the synthetic PATH",
        );

        let with_versioned_dirs = ToolLocator::over(
            vec![empty_path_dir],
            &[(lib.to_str().expect("utf-8 fixture path"), "llvm", "bin")],
            BTreeMap::new(),
        );
        assert_eq!(
            with_versioned_dirs.locate("clang"),
            Some(lib.join("llvm-19").join("bin").join("clang")),
            "a versioned install directory must be searched once PATH misses",
        );
    }

    /// Lexical ordering puts `llvm-9` above `llvm-19`; numeric ordering
    /// is the only one that picks the newer compiler.
    #[test]
    fn versioned_directories_are_ordered_numerically_not_lexically() {
        let fixture = fixture_root("numeric_order");
        let root = fixture.path();
        let lib = root.join("lib");
        touch_exe(&lib.join("llvm-9").join("bin"), "clang");
        touch_exe(&lib.join("llvm-19").join("bin"), "clang");
        touch_exe(&lib.join("llvm-11").join("bin"), "clang");

        let locator = ToolLocator::over(
            Vec::new(),
            &[(lib.to_str().expect("utf-8 fixture path"), "llvm", "bin")],
            BTreeMap::new(),
        );
        assert_eq!(
            locator.locate("clang"),
            Some(lib.join("llvm-19").join("bin").join("clang")),
            "highest version must win; lexical order would have chosen llvm-9",
        );
    }

    /// Same ordering rule for version-suffixed binaries sitting in one
    /// `PATH` directory, which is how Debian ships `clang-19` without a
    /// `clang` symlink.
    #[test]
    fn version_suffixed_names_on_path_are_ordered_numerically() {
        let fixture = fixture_root("suffix_order");
        let root = fixture.path();
        let bin = root.join("bin");
        touch_exe(&bin, "clang-9");
        touch_exe(&bin, "clang-19");

        let locator = ToolLocator::over(vec![bin.clone()], &[], BTreeMap::new());
        assert_eq!(locator.locate("clang"), Some(bin.join("clang-19")));
    }

    /// An unversioned keg (Homebrew's `llvm`) tracks the current stable
    /// release and must outrank a pinned older sibling.
    #[test]
    fn an_unversioned_install_directory_outranks_versioned_siblings() {
        let fixture = fixture_root("unversioned_first");
        let root = fixture.path();
        let opt = root.join("opt");
        touch_exe(&opt.join("llvm").join("bin"), "clang");
        touch_exe(&opt.join("llvm@18").join("bin"), "clang");

        let locator = ToolLocator::over(
            Vec::new(),
            &[(opt.to_str().expect("utf-8 fixture path"), "llvm", "bin")],
            BTreeMap::new(),
        );
        assert_eq!(
            locator.locate("clang"),
            Some(opt.join("llvm").join("bin").join("clang")),
        );
    }

    /// PATH stays authoritative: an inferred directory must never
    /// override what the caller put on their PATH.
    #[test]
    fn path_outranks_inferred_directories() {
        let fixture = fixture_root("path_wins");
        let root = fixture.path();
        let bin = root.join("bin");
        let on_path = touch_exe(&bin, "clang");
        let lib = root.join("lib");
        touch_exe(&lib.join("llvm-19").join("bin"), "clang");

        let locator = ToolLocator::over(
            vec![bin],
            &[(lib.to_str().expect("utf-8 fixture path"), "llvm", "bin")],
            BTreeMap::new(),
        );
        assert_eq!(locator.locate("clang"), Some(on_path));
    }

    /// An exact name outranks a version-suffixed one within PATH, so a
    /// deliberate `clang` symlink is not shadowed by `clang-9`.
    #[test]
    fn exact_names_outrank_version_suffixed_names_on_path() {
        let fixture = fixture_root("exact_first");
        let root = fixture.path();
        let bin = root.join("bin");
        let exact = touch_exe(&bin, "clang");
        touch_exe(&bin, "clang-19");

        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());
        assert_eq!(locator.locate("clang"), Some(exact));
    }

    #[test]
    fn an_explicit_override_wins_over_every_discovery_strategy() {
        let fixture = fixture_root("override_wins");
        let root = fixture.path();
        let bin = root.join("bin");
        touch_exe(&bin, "clang");
        let chosen = touch_exe(&root.join("chosen"), "clang");

        let overrides = BTreeMap::from([("SCE_TOOL_CLANG".to_string(), chosen.clone())]);
        let locator = ToolLocator::over(vec![bin], &[], overrides);
        assert_eq!(locator.locate("clang"), Some(chosen));
    }

    /// A mistyped override must fail loudly. Falling through would
    /// verify against a different toolchain than the one the operator
    /// named, which is worse than not honouring the setting at all.
    #[test]
    #[should_panic(expected = "is not an executable file")]
    fn an_override_that_does_not_resolve_is_an_error_not_a_fallthrough() {
        let fixture = fixture_root("override_broken");
        let root = fixture.path();
        let bin = root.join("bin");
        touch_exe(&bin, "clang");

        let overrides = BTreeMap::from([("SCE_TOOL_CLANG".to_string(), root.join("typo/clang"))]);
        ToolLocator::over(vec![bin], &[], overrides).locate("clang");
    }

    /// A non-executable file of the right name is not a tool. Without
    /// the permission test, a stray `clang` note file would shadow the
    /// real compiler.
    #[test]
    #[cfg(unix)]
    fn a_non_executable_file_is_not_a_match() {
        let fixture = fixture_root("not_executable");
        let root = fixture.path();
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).expect("create fixture dir");
        std::fs::write(bin.join("clang"), b"notes\n").expect("write non-executable");

        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());
        assert_eq!(locator.locate("clang"), None);
    }

    /// `clangd` and `clang-format` start with `clang` but are not it.
    #[test]
    fn unrelated_neighbours_sharing_a_prefix_are_not_matched() {
        let fixture = fixture_root("prefix_neighbours");
        let root = fixture.path();
        let bin = root.join("bin");
        touch_exe(&bin, "clangd");
        touch_exe(&bin, "clang-format");
        touch_exe(&bin, "clang-tidy");

        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());
        assert_eq!(
            locator.locate("clang"),
            None,
            "only `clang` or `clang-<digits>` may match",
        );
    }

    #[test]
    fn locate_any_returns_the_first_name_that_resolves() {
        let fixture = fixture_root("locate_any");
        let root = fixture.path();
        let bin = root.join("bin");
        let python3 = touch_exe(&bin, "python3");

        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());
        assert_eq!(locator.locate_any(&["python3", "python"]), Some(python3));
        assert_eq!(locator.locate_any(&["nonesuch", "neither"]), None);
    }

    /// Discovery must not depend on `readdir` order, which is
    /// filesystem- and creation-order-dependent.
    #[test]
    fn discovery_is_deterministic_across_repeated_scans() {
        let fixture = fixture_root("deterministic");
        let root = fixture.path();
        let lib = root.join("lib");
        for major in ["12", "19", "9", "18"] {
            touch_exe(&lib.join(format!("llvm-{major}")).join("bin"), "clang");
        }

        let locator = ToolLocator::over(
            Vec::new(),
            &[(lib.to_str().expect("utf-8 fixture path"), "llvm", "bin")],
            BTreeMap::new(),
        );
        let first = locator.locate("clang");
        for _ in 0..8 {
            assert_eq!(
                ToolLocator::over(
                    Vec::new(),
                    &[(lib.to_str().expect("utf-8 fixture path"), "llvm", "bin")],
                    BTreeMap::new(),
                )
                .locate("clang"),
                first,
                "repeated scans of the same tree must agree",
            );
        }
        assert_eq!(first, Some(lib.join("llvm-19").join("bin").join("clang")));
    }

    /// With tools required, a miss is a failure rather than a skip —
    /// this is what lets a CI lane assert its checks actually ran.
    #[test]
    #[should_panic(expected = "SCE_REQUIRE_TOOLS is set")]
    fn requiring_tools_turns_a_miss_into_a_failure() {
        ToolLocator::over(Vec::new(), &[], BTreeMap::new())
            .requiring_tools(true)
            .require_or_skip("clang", "compile the generated C11 headers");
    }

    #[test]
    fn without_the_requirement_a_miss_is_a_skip() {
        assert_eq!(
            ToolLocator::over(Vec::new(), &[], BTreeMap::new())
                .require_or_skip("clang", "compile the generated C11 headers"),
            None,
        );
    }

    /// Every `.rs` file in this crate, recursively.
    ///
    /// The source-scanning gates below are only as good as this list:
    /// a scan that silently found nothing would satisfy them without
    /// reading a line, so this asserts it walked a plausible tree
    /// rather than trusting that it did.
    fn crate_sources() -> Vec<PathBuf> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut out = Vec::new();
        let mut stack = vec![root.join("src"), root.join("tests")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }
        out.sort();

        assert!(
            out.len() > 50,
            "source scan found only {} .rs file(s) under {}; the gates \
             built on it would pass without checking anything",
            out.len(),
            root.display(),
        );
        assert!(
            out.iter().any(|p| p.ends_with("toolchain.rs")),
            "source scan did not reach this module's own file, so it is \
             not walking the tree it claims to",
        );
        out
    }

    /// No source resolves a tool by shelling out to `which`.
    ///
    /// `which` consults `PATH` and nothing else — precisely the defect
    /// this module exists to fix. Five near-identical `which`-based
    /// helpers had accumulated across the test suite, and each one
    /// silently narrowed what its tests verified on any host whose
    /// Clang lives in a versioned directory. Nothing failed; checks
    /// just stopped running.
    ///
    /// Duplication is what made that invisible, so this guards the
    /// property rather than the count: one locator, and no path back to
    /// a private `PATH`-only probe.
    #[test]
    fn no_source_resolves_a_tool_by_shelling_out_to_which() {
        // Assembled at runtime so this test does not match itself.
        let forbidden = format!("Command::new(\"{}\")", "which");
        let offenders: Vec<String> = crate_sources()
            .into_iter()
            .filter(|path| {
                std::fs::read_to_string(path)
                    .map(|text| text.contains(&forbidden))
                    .unwrap_or(false)
            })
            .map(|path| format!("  {}", path.display()))
            .collect();

        assert!(
            offenders.is_empty(),
            "{} file(s) resolve a tool through `which`, which reads PATH \
             and nothing else:\n{}\nUse `sce_build::toolchain::locate` \
             (or `locate_any`) instead — it searches versioned install \
             directories too, so a tool that is installed is found \
             rather than silently skipped.",
            offenders.len(),
            offenders.join("\n"),
        );
    }

    /// No source spawns a Clang-family binary by bare name.
    ///
    /// A bare name is resolved by the OS against `PATH` alone, so it
    /// carries the same blind spot as `which`. Clang is singled out
    /// because it is the family distributions actually install off
    /// `PATH` — measured on this repository's thirteen harness tools,
    /// `clang` and `clang++` were the only ones hidden that way.
    ///
    /// `gcc` / `g++` / `cc` are deliberately NOT covered: distributions
    /// put them on `PATH`, and the sites that spawn them treat a
    /// missing compiler as a hard error rather than a skip, so a wrong
    /// answer there is loud rather than silent. If that ever changes,
    /// add them here.
    #[test]
    fn no_source_spawns_a_clang_family_binary_by_bare_name() {
        let forbidden: Vec<String> = ["clang", "clang++", "clang-format", "clang-tidy", "clangd"]
            .iter()
            .map(|tool| format!("Command::new(\"{tool}\")"))
            .collect();

        let offenders: Vec<String> = crate_sources()
            .into_iter()
            .filter_map(|path| {
                let text = std::fs::read_to_string(&path).ok()?;
                let hit = forbidden.iter().find(|pattern| text.contains(*pattern))?;
                Some(format!("  {} contains {hit}", path.display()))
            })
            .collect();

        assert!(
            offenders.is_empty(),
            "{} file(s) spawn a Clang-family binary by bare name, which \
             the OS resolves against PATH alone:\n{}\nResolve through \
             `sce_build::toolchain::locate` and spawn the returned path.",
            offenders.len(),
            offenders.join("\n"),
        );
    }

    #[test]
    fn family_matching_accepts_only_the_family_and_its_versions() {
        let dash = &['-'][..];
        assert_eq!(
            match_family("clang", "clang", dash),
            Some(FamilyMatch::Unversioned),
        );
        assert_eq!(
            match_family("clang-19", "clang", dash),
            Some(FamilyMatch::Versioned(19)),
        );
        assert_eq!(match_family("clang-", "clang", dash), None);
        assert_eq!(match_family("clang-format", "clang", dash), None);
        assert_eq!(match_family("clangd", "clang", dash), None);
        assert_eq!(match_family("gcc-13", "clang", dash), None);

        // Homebrew's `@` separator, accepted only where specified.
        let both = &['-', '@'][..];
        assert_eq!(
            match_family("llvm@19", "llvm", both),
            Some(FamilyMatch::Versioned(19)),
        );
        assert_eq!(match_family("llvm@19", "llvm", dash), None);
    }

    #[test]
    fn an_unversioned_match_outranks_every_versioned_one() {
        assert!(FamilyMatch::Unversioned.rank() > FamilyMatch::Versioned(u32::MAX - 1).rank());
        assert!(FamilyMatch::Versioned(19).rank() > FamilyMatch::Versioned(9).rank());
    }
}
