// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// C++ code formatter — post-processes generated code through clang-format.
//
// Provides a default style (bundled) and accepts user-supplied .clang-format
// files for project-specific formatting. The formatter is a declared input of
// generation: one clang-format major shapes every formatted artefact
// (`CLANG_FORMAT_MAJOR`), and a run that cannot find it stops instead of
// emitting differently-shaped bytes (docs/SCE_CODEGEN_DETERMINISM.md §9).

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::toolchain::{self, ToolLocator};

/// Default C++ formatting style, bundled with the codegen binary.
/// Sourced from `tools/codegen/default.clang-format`.
const DEFAULT_STYLE: &str = include_str!("../../tools/codegen/default.clang-format");

/// The one clang-format major that may shape a generated C++ artefact.
///
/// A formatter is part of the function from document to bytes, so its
/// version is an input like any other. Measured 2026-09-24 on the 288 raw
/// C++ forge goldens with the bundled style: clang-format 18.1.3 and 19.1.7
/// disagreed on 17 files (`{ x = 0; }` kept on one line by 18, broken over
/// three by 19), while 19.1.1 and 19.1.7 agreed on every one. So the major
/// is pinned and the patch level is not. 19 is also the major the
/// repository's own C++ is held to (`clang-format-check.yml`).
pub const CLANG_FORMAT_MAJOR: u32 = 19;

/// The tool the locator searches for. Its version-suffixed siblings
/// (`clang-format-19`) and versioned LLVM install directories are part of
/// the same search, which is what finds the pinned major on a host whose
/// unsuffixed `clang-format` is another one.
pub const CLANG_FORMAT: &str = "clang-format";

/// Why a formatter could not be set up for a run.
#[derive(Debug)]
pub enum FormatError {
    /// No `clang-format` binary exists anywhere the locator searches.
    NotFound,
    /// `clang-format` binaries exist, but none reports
    /// [`CLANG_FORMAT_MAJOR`]. Each entry is a binary and the version it
    /// reported, or why it could not report one.
    WrongMajor { found: Vec<(PathBuf, String)> },
    /// `SCE_TOOL_CLANG_FORMAT` names a binary that cannot serve. An
    /// explicit override never falls through to a discovered binary: that
    /// would format with a tool other than the one named.
    OverrideUnusable {
        var: String,
        path: PathBuf,
        reason: String,
    },
    /// User-supplied style file does not exist.
    StyleNotFound(String),
    /// I/O error preparing the formatter.
    Io(std::io::Error),
}

/// A file clang-format could not format, and what it said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatFailure {
    pub file: String,
    pub detail: String,
}

impl std::fmt::Display for FormatFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "clang-format could not format {}: {}",
            self.file, self.detail
        )
    }
}

impl std::error::Error for FormatFailure {}

/// The repair every "no usable formatter" refusal ends with.
fn install_or_opt_out() -> String {
    format!(
        "install clang-format-{CLANG_FORMAT_MAJOR}, point {} at one, or pass --no-format",
        toolchain::override_var_for(CLANG_FORMAT),
    )
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::NotFound => write!(
                f,
                "formatting generated C++ requires clang-format {CLANG_FORMAT_MAJOR} and no \
                 clang-format was found on PATH or in a versioned LLVM install directory; {}",
                install_or_opt_out(),
            ),
            FormatError::WrongMajor { found } => {
                let seen: Vec<String> = found
                    .iter()
                    .map(|(path, version)| format!("{} ({version})", path.display()))
                    .collect();
                write!(
                    f,
                    "formatting generated C++ requires clang-format {CLANG_FORMAT_MAJOR}, and \
                     the only ones found are {}; {}",
                    seen.join(", "),
                    install_or_opt_out(),
                )
            }
            FormatError::OverrideUnusable { var, path, reason } => write!(
                f,
                "{var} names {}, which cannot format generated C++: {reason}; point it at \
                 clang-format {CLANG_FORMAT_MAJOR}, unset it, or pass --no-format",
                path.display(),
            ),
            FormatError::StyleNotFound(p) => write!(f, "style file not found: {p}"),
            FormatError::Io(e) => write!(f, "clang-format I/O error: {e}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// The major and full version one line of `clang-format --version` reports.
///
/// Every vendor prefixes the same phrase: `clang-format version 19.1.7`
/// from an upstream build, `Ubuntu clang-format version 19.1.1
/// (1ubuntu1~24.04.2)` from the distribution, `Homebrew clang-format
/// version 19.1.7` from a keg.
pub fn parse_clang_format_version(line: &str) -> Option<(u32, String)> {
    let rest = line.split("clang-format version ").nth(1)?;
    let version: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let version = version.trim_end_matches('.').to_string();
    let major = version.split('.').next()?.parse().ok()?;
    Some((major, version))
}

/// Ask `binary` which clang-format it is.
fn probe_version(binary: &Path) -> Result<(u32, String), String> {
    let output = Command::new(binary)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("does not run: {e}"))?;
    if !output.status.success() {
        return Err(format!("`--version` exited with {}", output.status));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(parse_clang_format_version)
        .ok_or_else(|| {
            format!(
                "`--version` printed no clang-format version: {:?}",
                stdout.lines().next().unwrap_or(""),
            )
        })
}

/// A clang-format binary of [`CLANG_FORMAT_MAJOR`], and the version it
/// reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedClangFormat {
    pub path: PathBuf,
    pub version: String,
}

/// Find a clang-format of [`CLANG_FORMAT_MAJOR`] in `locator`'s search
/// space.
///
/// An override is the one answer and is judged alone. Otherwise every
/// discovered binary is asked its version, best-ranked first, and the first
/// of the pinned major wins — so a host whose `clang-format` is 18 and whose
/// `clang-format-19` sits beside it formats with 19, and a host with only 18
/// is refused with both the binary and its version named.
pub fn resolve_clang_format(locator: &ToolLocator) -> Result<ResolvedClangFormat, FormatError> {
    if let Some(path) = locator.override_for(CLANG_FORMAT) {
        let unusable = |reason: String| FormatError::OverrideUnusable {
            var: toolchain::override_var_for(CLANG_FORMAT),
            path: path.to_path_buf(),
            reason,
        };
        if !toolchain::is_executable_file(path) {
            return Err(unusable("it is not an executable file".to_string()));
        }
        return match probe_version(path) {
            Ok((major, version)) if major == CLANG_FORMAT_MAJOR => Ok(ResolvedClangFormat {
                path: path.to_path_buf(),
                version,
            }),
            Ok((_, version)) => Err(unusable(format!("it reports clang-format {version}"))),
            Err(reason) => Err(unusable(reason)),
        };
    }

    let mut found: Vec<(PathBuf, String)> = Vec::new();
    // One binary reached by two names — `/usr/bin/clang-format-19` and the
    // `/usr/lib/llvm-19/bin/clang-format` it links to — is asked once.
    let mut asked: BTreeSet<PathBuf> = BTreeSet::new();
    for candidate in locator.discovered(CLANG_FORMAT) {
        let identity = std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
        if !asked.insert(identity) {
            continue;
        }
        match probe_version(&candidate) {
            Ok((major, version)) if major == CLANG_FORMAT_MAJOR => {
                return Ok(ResolvedClangFormat {
                    path: candidate,
                    version,
                });
            }
            Ok((_, version)) => found.push((candidate, version)),
            Err(reason) => found.push((candidate, reason)),
        }
    }
    if found.is_empty() {
        Err(FormatError::NotFound)
    } else {
        Err(FormatError::WrongMajor { found })
    }
}

/// Whether clang-format is the tool for `file_name`. A C++ run can emit
/// other artefacts beside its sources, and clang-format would reshape a
/// JSON sidecar as readily as a header.
fn is_cpp_source(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext,
                "h" | "hh" | "hpp" | "hxx" | "inl" | "c" | "cc" | "cpp" | "cxx"
            )
        })
}

/// Reusable C++ formatter that pipes code through `clang-format`.
///
/// Created once and reused across multiple format calls to avoid repeated
/// availability checks and temp-file setup.
pub struct CppFormatter {
    /// Resolved `clang-format` binary. Held rather than re-resolved per
    /// call so every file in a run is formatted by the same binary — a
    /// name looked up twice can resolve differently if `PATH` changes
    /// mid-run, which would split one build's output across two styles.
    clang_format: ResolvedClangFormat,
    style_path: PathBuf,
    /// Temp directory holding the default style; cleaned up on drop.
    _temp_dir: Option<PathBuf>,
}

impl CppFormatter {
    /// Create a formatter from the ambient environment.
    ///
    /// * `style_path` — explicit `.clang-format` file. When `None`, the built-in
    ///   default style is written to a temp file and used.
    ///
    /// Fails when the style file is missing or no clang-format of
    /// [`CLANG_FORMAT_MAJOR`] can be found; see [`resolve_clang_format`].
    pub fn new(style_path: Option<&Path>) -> Result<Self, FormatError> {
        Self::with_locator(style_path, &ToolLocator::from_env())
    }

    /// [`Self::new`] over an explicit search space.
    pub fn with_locator(
        style_path: Option<&Path>,
        locator: &ToolLocator,
    ) -> Result<Self, FormatError> {
        // The style file is the caller's own argument, so it is judged before
        // any tool is looked for. The other order let a missing style file
        // pass unremarked on every host that also lacked the formatter.
        if let Some(p) = style_path {
            if !p.exists() {
                return Err(FormatError::StyleNotFound(p.display().to_string()));
            }
        }
        let clang_format = resolve_clang_format(locator)?;

        match style_path {
            Some(p) => Ok(Self {
                clang_format,
                style_path: p.to_path_buf(),
                _temp_dir: None,
            }),
            None => {
                let dir = std::env::temp_dir().join(format!("sce-fmt-{}", std::process::id()));
                std::fs::create_dir_all(&dir).map_err(FormatError::Io)?;
                let style_file = dir.join(".clang-format");
                std::fs::write(&style_file, DEFAULT_STYLE).map_err(FormatError::Io)?;
                Ok(Self {
                    clang_format,
                    style_path: style_file,
                    _temp_dir: Some(dir),
                })
            }
        }
    }

    /// The clang-format this formatter runs, as the run manifest names it.
    pub fn clang_format(&self) -> &ResolvedClangFormat {
        &self.clang_format
    }

    /// Format C++ source code through `clang-format`.
    ///
    /// * `code` — the generated C++ source text.
    /// * `assume_filename` — tells clang-format what kind of file this is
    ///   (e.g., `"generated.h"` or the actual filename like `"test144_sm.h"`).
    pub fn format(&self, code: &str, assume_filename: &str) -> Result<String, FormatFailure> {
        let failed = |detail: String| FormatFailure {
            file: assume_filename.to_string(),
            detail,
        };
        let style_arg = format!("file:{}", self.style_path.display());

        let mut child = Command::new(&self.clang_format.path)
            .arg(format!("-style={style_arg}"))
            .arg(format!("--assume-filename={assume_filename}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                failed(format!(
                    "could not start {}: {e}",
                    self.clang_format.path.display()
                ))
            })?;

        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(code.as_bytes())
            .map_err(|e| failed(format!("could not write to clang-format: {e}")))?;

        let output = child
            .wait_with_output()
            .map_err(|e| failed(format!("could not read clang-format's output: {e}")))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(failed(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ))
        }
    }

    /// Format every C/C++ source among `(filename, code)` pairs, passing any
    /// other artefact through unchanged.
    ///
    /// The first file clang-format refuses fails the whole call. Keeping that
    /// file unformatted — what this did until 2026-09-24, with a warning on
    /// stderr — shipped bytes no other host would produce for the same input.
    pub fn format_output(
        &self,
        files: Vec<(String, String)>,
    ) -> Result<Vec<(String, String)>, FormatFailure> {
        files
            .into_iter()
            .map(|(name, code)| {
                if is_cpp_source(&name) {
                    self.format(&code, &name).map(|formatted| (name, formatted))
                } else {
                    Ok((name, code))
                }
            })
            .collect()
    }
}

impl Drop for CppFormatter {
    fn drop(&mut self) {
        if let Some(ref dir) = self._temp_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A stand-in clang-format: answers `--version` with `version_line` and
    /// otherwise copies stdin to stdout, so it formats nothing and the tests
    /// judge the resolver rather than a real tool.
    fn fake_clang_format(dir: &Path, name: &str, version_line: &str) -> PathBuf {
        std::fs::create_dir_all(dir).expect("create fixture dir");
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!("#!/bin/sh\nif [ \"$1\" = --version ]; then echo '{version_line}'; exit 0; fi\ncat\n"),
        )
        .expect("write fake clang-format");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake clang-format");
        }
        path
    }

    fn fixture_root(case: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("sce-formatter-{case}-"))
            .tempdir()
            .expect("create fixture root")
    }

    #[test]
    fn every_vendor_spelling_of_the_version_line_parses() {
        assert_eq!(
            parse_clang_format_version("clang-format version 19.1.7"),
            Some((19, "19.1.7".to_string())),
        );
        assert_eq!(
            parse_clang_format_version("Ubuntu clang-format version 19.1.1 (1ubuntu1~24.04.2)"),
            Some((19, "19.1.1".to_string())),
        );
        assert_eq!(
            parse_clang_format_version(
                "Ubuntu clang-format version 19.1.7 (++20250804090312+cd708029e0b2-1~exp1~20250804210325.79)"
            ),
            Some((19, "19.1.7".to_string())),
        );
        assert_eq!(
            parse_clang_format_version("Homebrew clang-format version 18.1.3"),
            Some((18, "18.1.3".to_string())),
        );
        assert_eq!(parse_clang_format_version("clang version 19.1.7"), None);
        assert_eq!(parse_clang_format_version(""), None);
    }

    /// The host shape that made this necessary: pc2's unsuffixed
    /// `clang-format` is 18 and `clang-format-19` sits beside it. The
    /// locator ranks the unsuffixed name first, so taking its first hit
    /// formatted with 18.
    #[test]
    fn the_pinned_major_wins_over_a_better_ranked_other_major() {
        let fixture = fixture_root("beside");
        let bin = fixture.path().join("bin");
        fake_clang_format(
            &bin,
            "clang-format",
            "Ubuntu clang-format version 18.1.3 (1ubuntu1)",
        );
        let nineteen = fake_clang_format(
            &bin,
            "clang-format-19",
            "Ubuntu clang-format version 19.1.1",
        );
        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());

        let resolved = resolve_clang_format(&locator).expect("clang-format-19 is on PATH");
        assert_eq!(resolved.path, nineteen);
        assert_eq!(resolved.version, "19.1.1");
    }

    #[test]
    fn only_another_major_is_refused_and_named() {
        let fixture = fixture_root("other_only");
        let bin = fixture.path().join("bin");
        let eighteen = fake_clang_format(
            &bin,
            "clang-format",
            "Ubuntu clang-format version 18.1.3 (1ubuntu1)",
        );
        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());

        match resolve_clang_format(&locator) {
            Err(FormatError::WrongMajor { found }) => {
                assert_eq!(found, vec![(eighteen, "18.1.3".to_string())]);
            }
            other => panic!("expected WrongMajor naming the 18, got {other:?}"),
        }
    }

    #[test]
    fn no_clang_format_at_all_is_not_found() {
        let fixture = fixture_root("none");
        let bin = fixture.path().join("bin");
        std::fs::create_dir_all(&bin).expect("create empty PATH dir");
        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());
        assert!(matches!(
            resolve_clang_format(&locator),
            Err(FormatError::NotFound)
        ));
    }

    /// Debian's `clang-format-19` package also installs the binary under
    /// `/usr/lib/llvm-19/bin`, which is where a host without the
    /// `/usr/bin` link has it.
    #[test]
    fn a_versioned_install_directory_supplies_the_pinned_major() {
        let fixture = fixture_root("versioned_dir");
        let lib = fixture.path().join("lib");
        let nineteen = fake_clang_format(
            &lib.join("llvm-19").join("bin"),
            "clang-format",
            "clang-format version 19.1.7",
        );
        let locator = ToolLocator::over(
            Vec::new(),
            &[(lib.to_str().expect("utf-8 fixture path"), "llvm", "bin")],
            BTreeMap::new(),
        );
        assert_eq!(
            resolve_clang_format(&locator).expect("found").path,
            nineteen
        );
    }

    /// An override is the caller naming one binary. When that binary is
    /// the wrong major the answer is a refusal, never a quiet switch to a
    /// discovered one — the run would then format with a tool nobody named.
    #[test]
    fn an_override_of_another_major_is_refused_even_beside_the_pinned_one() {
        let fixture = fixture_root("override");
        let bin = fixture.path().join("bin");
        fake_clang_format(&bin, "clang-format-19", "clang-format version 19.1.7");
        let eighteen = fake_clang_format(
            &fixture.path().join("elsewhere"),
            "cf18",
            "clang-format version 18.1.3",
        );
        let overrides =
            BTreeMap::from([(toolchain::override_var_for(CLANG_FORMAT), eighteen.clone())]);
        let locator = ToolLocator::over(vec![bin], &[], overrides);

        match resolve_clang_format(&locator) {
            Err(FormatError::OverrideUnusable { path, reason, .. }) => {
                assert_eq!(path, eighteen);
                assert!(
                    reason.contains("18.1.3"),
                    "the refusal names what it found: {reason}"
                );
            }
            other => panic!("expected OverrideUnusable, got {other:?}"),
        }
    }

    #[test]
    fn an_override_that_is_not_executable_is_refused_not_a_panic() {
        let fixture = fixture_root("override_missing");
        let missing = fixture.path().join("no-such-clang-format");
        let overrides =
            BTreeMap::from([(toolchain::override_var_for(CLANG_FORMAT), missing.clone())]);
        let locator = ToolLocator::over(Vec::new(), &[], overrides);
        assert!(matches!(
            resolve_clang_format(&locator),
            Err(FormatError::OverrideUnusable { path, .. }) if path == missing
        ));
    }

    /// The style file is the caller's own mistake to report, and it was
    /// accepted silently whenever the formatter lookup failed first.
    #[test]
    fn a_missing_style_file_is_reported_before_the_formatter_is_looked_for() {
        let fixture = fixture_root("style");
        let empty = fixture.path().join("bin");
        std::fs::create_dir_all(&empty).expect("create empty PATH dir");
        let locator = ToolLocator::over(vec![empty], &[], BTreeMap::new());
        let missing_style = fixture.path().join(".clang-format");
        assert!(matches!(
            CppFormatter::with_locator(Some(&missing_style), &locator),
            Err(FormatError::StyleNotFound(_))
        ));
    }

    #[test]
    fn only_cpp_sources_reach_clang_format() {
        let fixture = fixture_root("sources");
        let bin = fixture.path().join("bin");
        // This stand-in upper-cases what it formats, so a file that reached
        // it is visibly changed.
        std::fs::create_dir_all(&bin).expect("create fixture dir");
        let path = bin.join("clang-format-19");
        std::fs::write(
            &path,
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'clang-format version 19.1.1'; exit 0; fi\ntr a-z A-Z\n",
        )
        .expect("write upper-casing clang-format");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());
        let formatter = CppFormatter::with_locator(None, &locator).expect("formatter");

        let out = formatter
            .format_output(vec![
                ("m_sm.h".to_string(), "int x;".to_string()),
                ("sce_sourcemap.json".to_string(), "{\"v\":1}".to_string()),
            ])
            .expect("formats");
        assert_eq!(out[0], ("m_sm.h".to_string(), "INT X;".to_string()));
        assert_eq!(
            out[1],
            ("sce_sourcemap.json".to_string(), "{\"v\":1}".to_string())
        );
    }

    /// A file clang-format refuses fails the call. It used to be kept
    /// unformatted behind a stderr warning.
    #[test]
    fn a_file_clang_format_refuses_fails_the_run() {
        let fixture = fixture_root("refuses");
        let bin = fixture.path().join("bin");
        std::fs::create_dir_all(&bin).expect("create fixture dir");
        let path = bin.join("clang-format-19");
        std::fs::write(
            &path,
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'clang-format version 19.1.1'; exit 0; fi\necho 'unterminated' >&2; exit 1\n",
        )
        .expect("write refusing clang-format");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        let locator = ToolLocator::over(vec![bin], &[], BTreeMap::new());
        let formatter = CppFormatter::with_locator(None, &locator).expect("formatter");

        assert_eq!(
            formatter.format_output(vec![("m_sm.h".to_string(), "int x;".to_string())]),
            Err(FormatFailure {
                file: "m_sm.h".to_string(),
                detail: "unterminated".to_string(),
            }),
        );
    }
}
