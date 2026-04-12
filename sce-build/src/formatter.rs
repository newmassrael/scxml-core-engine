// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// C++ code formatter — post-processes generated code through clang-format.
//
// Provides a default style (bundled) and accepts user-supplied .clang-format
// files for project-specific formatting.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Default C++ formatting style, bundled with the codegen binary.
/// Sourced from `tools/codegen/default.clang-format`.
const DEFAULT_STYLE: &str = include_str!("../../tools/codegen/default.clang-format");

/// Errors from the formatting pipeline.
#[derive(Debug)]
pub enum FormatError {
    /// `clang-format` binary not found on PATH.
    NotFound,
    /// User-supplied style file does not exist.
    StyleNotFound(String),
    /// `clang-format` exited with an error.
    Failed(String),
    /// I/O error communicating with the child process.
    Io(std::io::Error),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::NotFound => write!(f, "clang-format not found on PATH"),
            FormatError::StyleNotFound(p) => write!(f, "style file not found: {p}"),
            FormatError::Failed(msg) => write!(f, "clang-format error: {msg}"),
            FormatError::Io(e) => write!(f, "clang-format I/O error: {e}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// Reusable C++ formatter that pipes code through `clang-format`.
///
/// Created once and reused across multiple format calls to avoid repeated
/// availability checks and temp-file setup.
pub struct CppFormatter {
    style_path: PathBuf,
    /// Temp directory holding the default style; cleaned up on drop.
    _temp_dir: Option<PathBuf>,
}

impl CppFormatter {
    /// Create a new formatter.
    ///
    /// * `style_path` — explicit `.clang-format` file. When `None`, the built-in
    ///   default style is written to a temp file and used.
    ///
    /// Returns `FormatError::NotFound` if `clang-format` is not on PATH.
    pub fn new(style_path: Option<&Path>) -> Result<Self, FormatError> {
        // Verify clang-format availability once
        let status = Command::new("clang-format")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(_) => return Err(FormatError::NotFound),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(FormatError::NotFound);
            }
            Err(e) => return Err(FormatError::Io(e)),
        }

        match style_path {
            Some(p) => {
                if !p.exists() {
                    return Err(FormatError::StyleNotFound(p.display().to_string()));
                }
                Ok(Self {
                    style_path: p.to_path_buf(),
                    _temp_dir: None,
                })
            }
            None => {
                let dir = std::env::temp_dir().join(format!("sce-fmt-{}", std::process::id()));
                std::fs::create_dir_all(&dir).map_err(FormatError::Io)?;
                let style_file = dir.join(".clang-format");
                std::fs::write(&style_file, DEFAULT_STYLE).map_err(FormatError::Io)?;
                Ok(Self {
                    style_path: style_file,
                    _temp_dir: Some(dir),
                })
            }
        }
    }

    /// Format C++ source code through `clang-format`.
    ///
    /// * `code` — the generated C++ source text.
    /// * `assume_filename` — tells clang-format what kind of file this is
    ///   (e.g., `"generated.h"` or the actual filename like `"test144_sm.h"`).
    pub fn format(&self, code: &str, assume_filename: &str) -> Result<String, FormatError> {
        let style_arg = format!("file:{}", self.style_path.display());

        let mut child = Command::new("clang-format")
            .arg(format!("-style={style_arg}"))
            .arg(format!("--assume-filename={assume_filename}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FormatError::NotFound,
                _ => FormatError::Io(e),
            })?;

        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(code.as_bytes())
            .map_err(FormatError::Io)?;

        let output = child.wait_with_output().map_err(FormatError::Io)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(FormatError::Failed(stderr))
        }
    }

    /// Format a list of `(filename, code)` pairs in-place, returning the
    /// formatted pairs. Files that fail to format are returned unchanged
    /// with a warning printed to stderr.
    pub fn format_output(
        &self,
        files: Vec<(String, String)>,
    ) -> Vec<(String, String)> {
        files
            .into_iter()
            .map(|(name, code)| {
                match self.format(&code, &name) {
                    Ok(formatted) => (name, formatted),
                    Err(e) => {
                        eprintln!("  Warning: format failed for {name}: {e}");
                        (name, code)
                    }
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
