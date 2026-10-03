// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The figures of a model, drawn by SCE.
//!
//! The workbench draws nothing. A picture of a model, a table of what it
//! states, a state figure: each is made by `sce-codegen diagram`, and this
//! module is the one place that runs it, so the desktop app, the browser shell
//! and `sce-work` cannot disagree about what a model looks like or about what
//! the product refused. The screen shows the SVG it is handed.
//!
//! # Why a program and not a library
//!
//! `sce-codegen` is the same binary an author and the authoring MCP run, so
//! "what the product says about this model" is one answer however it is asked
//! for. It also keeps the product's compiler out of the application's own
//! process: a model that makes the generator slow or fall over costs a bounded
//! wait and a refusal, not the window.
//!
//! # What a refusal is
//!
//! The generator refuses rather than draws something wrong (a figure that does
//! not fit its page, a character it has not measured, a document it does not
//! accept). That is an answer, and it reaches the screen with the product's own
//! code and sentence, never as an empty picture.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// How long the generator may take for one model.
pub const RENDER_TIMEOUT: Duration = Duration::from_secs(30);

/// The most one set of figures may hold, in bytes of SVG.
pub const MAX_FIGURE_BYTES: usize = 64 * 1024 * 1024;

/// The environment variable that names the generator.
pub const GENERATOR_ENV: &str = "SCE_CODEGEN";

/// How often a running generator is looked at.
const POLL: Duration = Duration::from_millis(20);

/// What to draw and how.
#[derive(Debug, Clone, Default)]
pub struct FigureRequest<'a> {
    /// The model: an SCXML document, as text.
    pub model: &'a str,
    /// What the document is called, which is what the product names its figures
    /// by (it takes a document's name from its file's, so the staged file is
    /// given this one). Lowercase letters, digits and `-`; anything else, or
    /// nothing, is `model`.
    pub name: Option<&'a str>,
    /// A page the product lists (`a4-portrait`, ...). Its default when absent.
    pub page: Option<&'a str>,
    /// A vocabulary the product lists (`en`, `ko`). Its default when absent.
    pub lexicon: Option<&'a str>,
    /// The smallest type size, in points. Its default when absent.
    pub min_pt: Option<f64>,
}

/// One drawn sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sheet {
    /// The file name the product gave it (`layout.svg`, `fields-1.svg`).
    pub name: String,
    pub svg: String,
}

/// What was drawn, in the order the product wrote it: the pictures a kind has
/// first, then the table of every value the model states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FigureSet {
    /// The generator's own version line, when it gave one.
    pub generator: Option<String>,
    pub sheets: Vec<Sheet>,
}

/// Why nothing was drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// There is no generator to run, or it could not be started.
    Unavailable { reason: String },
    /// The generator ran and refused the model, in its own words.
    Refused { code: String, message: String },
    /// The generator did not finish in time and was stopped.
    TimedOut { seconds: u64 },
    /// The generator failed in a way it did not explain, or left something the
    /// workbench cannot read.
    Failed { reason: String },
}

impl RenderError {
    /// The word a program branches on.
    pub fn kind(&self) -> &'static str {
        match self {
            RenderError::Unavailable { .. } => "sce-unavailable",
            RenderError::Refused { .. } => "sce-refused",
            RenderError::TimedOut { .. } => "sce-timeout",
            RenderError::Failed { .. } => "sce-failed",
        }
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Unavailable { reason } => {
                write!(f, "the SCE generator is not available: {reason}")
            }
            RenderError::Refused { code, message } => {
                write!(f, "SCE refused the model ({code}): {message}")
            }
            RenderError::TimedOut { seconds } => write!(
                f,
                "the SCE generator did not finish in {seconds} s and was stopped"
            ),
            RenderError::Failed { reason } => write!(f, "the SCE generator failed: {reason}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Something that can draw a model. The command layer is given one, so a test
/// draws with a stand-in and the product is exercised on its own.
pub trait FigureRenderer: Send + Sync {
    fn render(&self, request: &FigureRequest<'_>) -> Result<FigureSet, RenderError>;
}

/// A renderer for a process that has no generator: every request is
/// unavailable, saying so.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRenderer;

impl FigureRenderer for NoRenderer {
    fn render(&self, _: &FigureRequest<'_>) -> Result<FigureSet, RenderError> {
        Err(RenderError::Unavailable {
            reason: format!(
                "no `sce-codegen` was found; set {GENERATOR_ENV} to its path, or put it beside \
                 this program or on PATH"
            ),
        })
    }
}

/// The renderer a process uses unless it is told otherwise: the generator
/// [`SceCodegen::discover`] finds, or [`NoRenderer`] saying there is none.
///
/// One definition, so the application, the browser shell and `sce-work` look
/// in the same places.
pub fn default_renderer() -> Box<dyn FigureRenderer> {
    match SceCodegen::discover() {
        Some(generator) => Box::new(generator),
        None => Box::new(NoRenderer),
    }
}

/// The product's generator, run as a program.
#[derive(Debug)]
pub struct SceCodegen {
    program: PathBuf,
    timeout: Duration,
    version: OnceLock<Option<String>>,
}

impl SceCodegen {
    /// The generator at `program`.
    pub fn at(program: impl Into<PathBuf>) -> Self {
        SceCodegen {
            program: program.into(),
            timeout: RENDER_TIMEOUT,
            version: OnceLock::new(),
        }
    }

    /// Where the generator is.
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// The same, giving up on one model after `timeout`.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Where the generator is: [`GENERATOR_ENV`] when it is set, otherwise
    /// `sce-codegen` beside the running program, otherwise on `PATH`.
    ///
    /// The environment first, because an installed application has no tree: it
    /// ships the generator beside itself, and a developer points at a build.
    pub fn discover() -> Option<Self> {
        let named = |value: std::ffi::OsString| {
            if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            }
        };
        if let Some(path) = std::env::var_os(GENERATOR_ENV).and_then(named) {
            return Some(SceCodegen::at(path));
        }
        let file = format!("sce-codegen{}", std::env::consts::EXE_SUFFIX);
        let beside = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(&file)))
            .filter(|path| path.is_file());
        if let Some(path) = beside {
            return Some(SceCodegen::at(path));
        }
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(&file))
                .find(|path| path.is_file())
                .map(SceCodegen::at)
        })
    }

    /// The generator's own version line (`sce-codegen --version`), asked once.
    pub fn version(&self) -> Option<&str> {
        self.version
            .get_or_init(|| {
                let scratch = Scratch::new().ok()?;
                let mut command = Command::new(&self.program);
                command.arg("--version");
                let run = run_bounded(command, scratch.path(), self.timeout).ok()?;
                run.stdout
                    .lines()
                    .next()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
            })
            .as_deref()
    }
}

impl FigureRenderer for SceCodegen {
    fn render(&self, request: &FigureRequest<'_>) -> Result<FigureSet, RenderError> {
        let scratch = Scratch::new().map_err(|e| RenderError::Failed {
            reason: format!("no place to stage the model: {e}"),
        })?;
        let document = scratch.path().join(format!("{}.scxml", stem(request.name)));
        fs::write(&document, request.model).map_err(|e| RenderError::Failed {
            reason: format!("the model could not be staged: {e}"),
        })?;
        let out = scratch.path().join("figures");

        let mut command = Command::new(&self.program);
        command
            .args(["--error-format", "json", "diagram"])
            .arg(&document)
            .arg("-o")
            .arg(&out);
        // The product's registry is what these names mean and its defaults are
        // its own; they are passed through, checked only for being a word and
        // not an option.
        for (flag, value) in [("--page", request.page), ("--lexicon", request.lexicon)] {
            if let Some(value) = value {
                command.arg(flag).arg(word(flag, value)?);
            }
        }
        if let Some(min_pt) = request.min_pt {
            if !(min_pt.is_finite() && min_pt > 0.0) {
                return Err(RenderError::Failed {
                    reason: format!("--min-pt must be a positive number of points, got {min_pt}"),
                });
            }
            command.arg("--min-pt").arg(format!("{min_pt}"));
        }

        let run = run_bounded(command, scratch.path(), self.timeout)?;
        if !run.success {
            return Err(refusal(&run.stderr, run.code));
        }

        // The files the generator says it wrote, in the order it said so, and
        // only ones that lie in the folder it was given.
        let mut sheets = Vec::new();
        let mut total = 0usize;
        for line in run.stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let path = PathBuf::from(line);
            let inside = path.starts_with(&out) || scratch.path().join(&path).starts_with(&out);
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            let (true, Some(name)) = (inside, name) else {
                return Err(RenderError::Failed {
                    reason: format!("the generator named a file outside its output folder: {line}"),
                });
            };
            let located = if path.is_absolute() {
                path.clone()
            } else {
                scratch.path().join(&path)
            };
            let svg = fs::read_to_string(&located).map_err(|e| RenderError::Failed {
                reason: format!("{name} could not be read back: {e}"),
            })?;
            total += svg.len();
            if total > MAX_FIGURE_BYTES {
                return Err(RenderError::Failed {
                    reason: format!("the figures are larger than {MAX_FIGURE_BYTES} bytes"),
                });
            }
            sheets.push(Sheet { name, svg });
        }
        if sheets.is_empty() {
            return Err(RenderError::Failed {
                reason: "the generator succeeded and wrote no figure".to_string(),
            });
        }
        Ok(FigureSet {
            generator: self.version().map(str::to_string),
            sheets,
        })
    }
}

/// The file stem a document is staged under: `name` when it is a short word of
/// lowercase letters, digits and `-` that starts with a letter or digit, else
/// `model`. A name is never trusted as a path.
fn stem(name: Option<&str>) -> &str {
    const FALLBACK: &str = "model";
    match name {
        Some(name)
            if !name.is_empty()
                && name.len() <= 60
                && !name.starts_with('-')
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') =>
        {
            name
        }
        _ => FALLBACK,
    }
}

/// A value that is a word: letters, digits, `-` and `_`, and not an option.
fn word(flag: &str, value: &str) -> Result<String, RenderError> {
    let ok = !value.is_empty()
        && !value.starts_with('-')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(value.to_string())
    } else {
        Err(RenderError::Failed {
            reason: format!("{flag} takes a name (letters, digits, `-`), got `{value}`"),
        })
    }
}

/// The product's refusal from what it wrote on standard error: its first record
/// is JSON with a `code` and a `message`.
fn refusal(stderr: &str, status: Option<i32>) -> RenderError {
    let record = stderr
        .lines()
        .find_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(serde_json::Value::is_object);
    match record {
        Some(record) => {
            let text = |key: &str| record.get(key).and_then(|v| v.as_str()).map(str::to_string);
            match (text("code"), text("message")) {
                (Some(code), Some(message)) => RenderError::Refused { code, message },
                _ => RenderError::Failed {
                    reason: format!("exit status {status:?}: {}", excerpt(stderr)),
                },
            }
        }
        None => RenderError::Failed {
            reason: format!("exit status {status:?}: {}", excerpt(stderr)),
        },
    }
}

/// The start of what a program wrote, for a message.
fn excerpt(text: &str) -> String {
    let text = text.trim();
    let mut kept: String = text.chars().take(400).collect();
    if text.chars().count() > 400 {
        kept.push_str(" ...");
    }
    if kept.is_empty() {
        "it wrote nothing".to_string()
    } else {
        kept
    }
}

/// What a bounded run left.
struct Run {
    success: bool,
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Run `command` in `dir`, and stop it if it takes longer than `timeout`.
///
/// Its output goes to files, not pipes: a program that writes more than a pipe
/// holds would otherwise wait for a reader this loop is not.
fn run_bounded(mut command: Command, dir: &Path, timeout: Duration) -> Result<Run, RenderError> {
    let stdout_path = dir.join(".stdout");
    let stderr_path = dir.join(".stderr");
    let open = |path: &Path| {
        File::create(path).map_err(|e| RenderError::Failed {
            reason: format!("no place to collect the generator's output: {e}"),
        })
    };
    command
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(open(&stdout_path)?)
        .stderr(open(&stderr_path)?);
    let mut child = command.spawn().map_err(|e| RenderError::Unavailable {
        reason: format!(
            "{} could not be started: {e}",
            command.get_program().to_string_lossy()
        ),
    })?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RenderError::TimedOut {
                    seconds: timeout.as_secs(),
                });
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RenderError::Failed {
                    reason: format!("the generator could not be waited for: {e}"),
                });
            }
        }
    };
    let read =
        |path: &Path| String::from_utf8_lossy(&fs::read(path).unwrap_or_default()).into_owned();
    Ok(Run {
        success: status.success(),
        code: status.code(),
        stdout: read(&stdout_path),
        stderr: read(&stderr_path),
    })
}

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A folder of its own under the system's temporary directory, removed when it
/// is dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> io::Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "sce-figures-{}-{nanos}-{}",
            std::process::id(),
            SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Scratch(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_a_word_and_never_an_option() {
        assert_eq!(word("--page", "a4-portrait").unwrap(), "a4-portrait");
        for bad in ["", "--out", "-x", "a b", "a;b", "../x", "ko\n"] {
            assert!(word("--page", bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_document_is_staged_under_a_safe_name_or_model() {
        assert_eq!(stem(Some("door-lock")), "door-lock");
        assert_eq!(stem(Some("v2")), "v2");
        for bad in [
            "",
            "-x",
            "Door",
            "a b",
            "../x",
            "a/b",
            "a.scxml",
            &"x".repeat(61),
        ] {
            assert_eq!(stem(Some(bad)), "model", "{bad:?}");
        }
        assert_eq!(stem(None), "model");
    }

    #[test]
    fn a_refusal_carries_the_products_code_and_sentence() {
        let stderr = r#"{"v":1,"id":"x","code":"cli/diagram-does-not-fit","message":"too big"}"#;
        assert_eq!(
            refusal(stderr, Some(20)),
            RenderError::Refused {
                code: "cli/diagram-does-not-fit".into(),
                message: "too big".into()
            }
        );
        // Anything that is not a record is a failure that says what it saw.
        match refusal("segmentation fault", Some(139)) {
            RenderError::Failed { reason } => assert!(reason.contains("segmentation fault")),
            other => panic!("{other:?}"),
        }
        match refusal("", None) {
            RenderError::Failed { reason } => assert!(reason.contains("wrote nothing")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn every_error_has_its_own_kind() {
        let kinds = [
            RenderError::Unavailable {
                reason: String::new(),
            }
            .kind(),
            RenderError::Refused {
                code: String::new(),
                message: String::new(),
            }
            .kind(),
            RenderError::TimedOut { seconds: 1 }.kind(),
            RenderError::Failed {
                reason: String::new(),
            }
            .kind(),
        ];
        let mut sorted = kinds.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), kinds.len());
    }

    /// The staging folder is its own and is gone when it is dropped, whatever
    /// was in it.
    #[test]
    fn a_staging_folder_is_removed_when_it_is_dropped() {
        let path = {
            let scratch = Scratch::new().unwrap();
            fs::write(scratch.path().join("model.scxml"), "x").unwrap();
            fs::create_dir(scratch.path().join("figures")).unwrap();
            assert!(scratch.path().is_dir());
            scratch.path().to_path_buf()
        };
        assert!(!path.exists());
        let other = Scratch::new().unwrap();
        let again = Scratch::new().unwrap();
        assert_ne!(
            other.path(),
            again.path(),
            "two stagings never share a folder"
        );
    }

    #[test]
    fn no_renderer_says_how_to_get_one() {
        let error = NoRenderer.render(&FigureRequest::default()).unwrap_err();
        assert_eq!(error.kind(), "sce-unavailable");
        assert!(error.to_string().contains(GENERATOR_ENV), "{error}");
    }

    #[test]
    fn a_missing_program_is_unavailable_not_a_crash() {
        let renderer = SceCodegen::at("/nonexistent/sce-codegen");
        let error = renderer
            .render(&FigureRequest {
                model: "<scxml/>",
                ..FigureRequest::default()
            })
            .unwrap_err();
        assert_eq!(error.kind(), "sce-unavailable", "{error}");
    }
}
