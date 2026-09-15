// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a person accepted, pinned so that its authority can lapse.
//!
//! Requirement-closure RFC §8.3: an acceptance is of a `(manifest revision,
//! variant, document)` triple, and if any of the three moves, the authority
//! lapses and a person must accept again. Without the pin an acceptance is
//! a golden file — it goes on defending old behaviour after the
//! specification or the design has moved under it.
//!
//! # What is pinned, and why each part
//!
//! ```text
//!   manifest   doc_id + rev, and its bytes    a requirement added under an
//!                                             unchanged rev still moves it
//!   variant    the name the acceptance was    accepting base says nothing
//!              given for                      about base + tls (RFC §5.2d)
//!   inputs     every file the parse READ,     the entry document is not the
//!              each by sha256                 whole design: an <xi:include>d
//!                                             fragment is part of it
//! ```
//!
//! ⚠ The inputs are the parser's own answer,
//! [`SCXMLParser::preprocessor_deps`], and never a walk of the directory. A
//! walk decides the set a second time, and deciding a set twice is how a
//! member ends up outside one of the answers — `forge::drift`'s depfile
//! writer lost the shared macro family and then the import closure that
//! way. Re-checking parses again for the same reason: a change of
//! composition is a change of design even when every file that was pinned
//! still has its old bytes.
//!
//! # Bytes, not a model
//!
//! A pin over the parsed model would survive an edit that changes nothing a
//! person saw — a comment, a reflowed attribute — but only by trusting that
//! the model's serialisation is total over behaviour, which nothing here
//! checks. The byte pin lapses more often than the behaviour moves, and that
//! is the direction to be wrong in: a lapse costs a person a second look,
//! and a false hold costs the acceptance its meaning.
//!
//! # Paths are relative to a root the caller names
//!
//! A record is committed and read on other checkouts, so an absolute path
//! would lapse it everywhere but the machine that took it. Every pinned path
//! is stored relative to the root given when the record was taken, and an
//! input outside that root is refused rather than recorded as a path that
//! silently stops meaning anything elsewhere.
//!
//! # Deliberately not `Deserialize`
//!
//! [`AcceptanceRecord`] is obtained from [`AcceptanceRecord::take`] or
//! [`AcceptanceRecord::from_json`] and no other way, for the reason
//! [`crate::requirement_manifest`] gives for its own manifest: a derive on
//! the public type lets a caller load bytes the format check refuses.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::generator_witness::{hex_encode, sha256_bytes};
use crate::parser::SCXMLParser;
use crate::requirement_manifest::{ManifestError, RequirementManifest};

/// The value a record's `record` field holds. A JSON file naming any other
/// kind is refused before its pins are read.
pub const RECORD_KIND: &str = "sce-acceptance-record";

/// The format version. Moves only when a pinned field changes meaning.
pub const RECORD_VERSION: u32 = 1;

/// The manifest an acceptance was measured against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestPin {
    /// Relative to the record's root.
    pub path: String,
    pub doc_id: String,
    pub rev: String,
    /// Over the manifest's bytes. `rev` alone is what the specification's
    /// owner says moved; the bytes are what did.
    pub sha256: String,
}

/// One file the design was read from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPin {
    /// Relative to the record's root.
    pub path: String,
    pub sha256: String,
}

/// An acceptance, pinned. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceRecord {
    /// The name the acceptance was given for. Compared verbatim.
    pub variant: String,
    /// The entry document, relative to the root.
    pub document: String,
    pub manifest: ManifestPin,
    /// Every file the parse read, the entry document included, sorted by
    /// path.
    pub inputs: Vec<InputPin>,
}

/// The record exactly as JSON spells it. Private — see the module docs.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordWire {
    record: String,
    v: u32,
    variant: String,
    document: String,
    manifest: ManifestPin,
    inputs: Vec<InputPin>,
}

/// One way what was accepted is no longer what is there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lapse {
    /// The record was taken for a different variant than the one asked
    /// about.
    Variant { recorded: String, asked: String },
    /// The manifest's bytes moved. `current_rev` is `None` when the
    /// manifest no longer loads, which is a lapse and not a failure to
    /// check: the record still says what was accepted.
    Manifest {
        recorded_rev: String,
        current_rev: Option<String>,
        recorded_sha256: String,
        current_sha256: String,
    },
    /// A pinned file is gone.
    Missing { path: String },
    /// A pinned file's bytes moved.
    Moved {
        path: String,
        recorded_sha256: String,
        current_sha256: String,
    },
    /// The parse now reads a file the acceptance never saw.
    Added { path: String },
    /// The entry document no longer parses, so which files make up the
    /// design cannot be said.
    Unparseable { path: String, detail: String },
}

impl fmt::Display for Lapse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lapse::Variant { recorded, asked } => write!(
                f,
                "the acceptance was taken for variant `{recorded}`, not `{asked}`"
            ),
            Lapse::Manifest {
                recorded_rev,
                current_rev,
                ..
            } => match current_rev {
                Some(current) if current == recorded_rev => write!(
                    f,
                    "the manifest changed under an unchanged revision `{recorded_rev}`"
                ),
                Some(current) => write!(
                    f,
                    "the manifest moved from revision `{recorded_rev}` to `{current}`"
                ),
                None => write!(
                    f,
                    "the manifest accepted at revision `{recorded_rev}` no longer loads"
                ),
            },
            Lapse::Missing { path } => write!(f, "{path}: pinned, and no longer there"),
            Lapse::Moved { path, .. } => write!(f, "{path}: changed since it was accepted"),
            Lapse::Added { path } => {
                write!(f, "{path}: now part of the design, and never accepted")
            }
            Lapse::Unparseable { path, detail } => {
                write!(f, "{path}: no longer parses ({detail})")
            }
        }
    }
}

/// Why a record could not be taken or read.
#[derive(Debug)]
pub enum RecordError {
    Read {
        path: String,
        source: std::io::Error,
    },
    /// The document refused to parse when the record was being taken. A
    /// record of a design that cannot be read would pin nothing.
    Parse {
        path: String,
        detail: String,
    },
    Manifest(ManifestError),
    /// A file the design depends on lies outside the root, so no path
    /// relative to it can name the file.
    OutsideRoot {
        path: String,
        root: String,
    },
    /// A variant name that cannot be compared verbatim.
    Variant {
        value: String,
    },
    /// Bytes that are not a record this module wrote.
    Format {
        detail: String,
    },
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::Read { path, source } => write!(f, "{path}: {source}"),
            RecordError::Parse { path, detail } => {
                write!(f, "{path}: the document does not parse: {detail}")
            }
            RecordError::Manifest(inner) => write!(f, "{inner}"),
            RecordError::OutsideRoot { path, root } => write!(
                f,
                "{path} lies outside {root}; an acceptance record names its files \
                 relative to one root, so take it from a root that contains the design"
            ),
            RecordError::Variant { value } => write!(
                f,
                "variant `{value}` is not a name: it must be non-empty and hold no \
                 whitespace, because it is compared verbatim"
            ),
            RecordError::Format { detail } => {
                write!(f, "not an acceptance record: {detail}")
            }
        }
    }
}

impl std::error::Error for RecordError {}

impl AcceptanceRecord {
    /// Pin the design at `document`, measured against `manifest`, as the
    /// acceptance of `variant`.
    ///
    /// Every path is recorded relative to `root`.
    pub fn take(
        root: &Path,
        document: &Path,
        manifest: &Path,
        variant: &str,
    ) -> Result<Self, RecordError> {
        check_variant(variant)?;
        let root = canonical(root)?;
        let loaded = RequirementManifest::load(manifest).map_err(RecordError::Manifest)?;
        let manifest_pin = ManifestPin {
            path: relative(&root, &canonical(manifest)?)?,
            doc_id: loaded.doc_id,
            rev: loaded.rev,
            sha256: file_sha256(manifest)?,
        };
        let document_path = canonical(document)?;
        let inputs = read_design(&root, &document_path)
            .map_err(|failure| failure.into_take_error(document))?;
        Ok(AcceptanceRecord {
            variant: variant.to_string(),
            document: relative(&root, &document_path)?,
            manifest: manifest_pin,
            inputs,
        })
    }

    /// Everything that moved since the record was taken, asked about
    /// `variant` under `root`. Empty means the acceptance still holds.
    ///
    /// Every component is examined even after one has lapsed, so a reader
    /// learns the whole of what moved at once rather than one part a run.
    pub fn recheck(&self, root: &Path, variant: &str) -> Result<Vec<Lapse>, RecordError> {
        let root = canonical(root)?;
        let mut lapses = Vec::new();

        if variant != self.variant {
            lapses.push(Lapse::Variant {
                recorded: self.variant.clone(),
                asked: variant.to_string(),
            });
        }

        let manifest_path = root.join(&self.manifest.path);
        match std::fs::read(&manifest_path) {
            Err(_) => lapses.push(Lapse::Missing {
                path: self.manifest.path.clone(),
            }),
            Ok(bytes) => {
                let current_sha256 = hex_encode(&sha256_bytes(&bytes));
                if current_sha256 != self.manifest.sha256 {
                    lapses.push(Lapse::Manifest {
                        recorded_rev: self.manifest.rev.clone(),
                        current_rev: RequirementManifest::load(&manifest_path)
                            .ok()
                            .map(|m| m.rev),
                        recorded_sha256: self.manifest.sha256.clone(),
                        current_sha256,
                    });
                }
            }
        }

        let document_path = root.join(&self.document);
        if !document_path.is_file() {
            lapses.push(Lapse::Missing {
                path: self.document.clone(),
            });
            return Ok(lapses);
        }
        let current = match read_design(&root, &canonical(&document_path)?) {
            Ok(inputs) => inputs,
            Err(DesignFailure::Parse(detail)) => {
                lapses.push(Lapse::Unparseable {
                    path: self.document.clone(),
                    detail,
                });
                return Ok(lapses);
            }
            Err(DesignFailure::Record(error)) => return Err(error),
        };

        let now: BTreeMap<&str, &str> = current
            .iter()
            .map(|pin| (pin.path.as_str(), pin.sha256.as_str()))
            .collect();
        let then: BTreeMap<&str, &str> = self
            .inputs
            .iter()
            .map(|pin| (pin.path.as_str(), pin.sha256.as_str()))
            .collect();
        for (path, recorded) in &then {
            match now.get(path) {
                // Gone from the design either way: deleted, or still on
                // disk and no longer read because the composition moved
                // away from it. Both mean the design that was accepted is
                // not the one the parse assembles now.
                None => lapses.push(Lapse::Missing {
                    path: (*path).to_string(),
                }),
                Some(current) if current != recorded => lapses.push(Lapse::Moved {
                    path: (*path).to_string(),
                    recorded_sha256: (*recorded).to_string(),
                    current_sha256: (*current).to_string(),
                }),
                Some(_) => {}
            }
        }
        for path in now.keys() {
            if !then.contains_key(path) {
                lapses.push(Lapse::Added {
                    path: (*path).to_string(),
                });
            }
        }
        Ok(lapses)
    }

    /// The record as committed: pretty JSON with a trailing newline, so a
    /// re-take that pins the same design writes the same bytes.
    pub fn to_json(&self) -> String {
        let wire = RecordWire {
            record: RECORD_KIND.to_string(),
            v: RECORD_VERSION,
            variant: self.variant.clone(),
            document: self.document.clone(),
            manifest: self.manifest.clone(),
            inputs: self.inputs.clone(),
        };
        let mut text = serde_json::to_string_pretty(&wire)
            .expect("a record of strings and one integer always serialises");
        text.push('\n');
        text
    }

    /// Read a record, refusing anything this module would not have written.
    pub fn from_json(text: &str) -> Result<Self, RecordError> {
        let wire: RecordWire = serde_json::from_str(text).map_err(|e| RecordError::Format {
            detail: e.to_string(),
        })?;
        if wire.record != RECORD_KIND {
            return Err(RecordError::Format {
                detail: format!("`record` is `{}`, not `{RECORD_KIND}`", wire.record),
            });
        }
        if wire.v != RECORD_VERSION {
            return Err(RecordError::Format {
                detail: format!("version {} is not {RECORD_VERSION}", wire.v),
            });
        }
        check_variant(&wire.variant)?;
        for path in std::iter::once(&wire.document)
            .chain(std::iter::once(&wire.manifest.path))
            .chain(wire.inputs.iter().map(|pin| &pin.path))
        {
            check_relative(path)?;
        }
        for digest in
            std::iter::once(&wire.manifest.sha256).chain(wire.inputs.iter().map(|pin| &pin.sha256))
        {
            check_sha256(digest)?;
        }
        // The entry document is the one input no composition can drop, so a
        // record whose inputs omit it pins a design it never read.
        if !wire.inputs.iter().any(|pin| pin.path == wire.document) {
            return Err(RecordError::Format {
                detail: format!("the inputs do not pin the document `{}`", wire.document),
            });
        }
        let mut inputs = wire.inputs;
        let before = inputs.len();
        inputs.sort();
        inputs.dedup_by(|a, b| a.path == b.path);
        if inputs.len() != before {
            return Err(RecordError::Format {
                detail: "an input path is pinned more than once".to_string(),
            });
        }
        Ok(AcceptanceRecord {
            variant: wire.variant,
            document: wire.document,
            manifest: wire.manifest,
            inputs,
        })
    }
}

/// Why the design's files could not be read.
enum DesignFailure {
    /// The document does not parse. A lapse when re-checking, a refusal
    /// when taking.
    Parse(String),
    Record(RecordError),
}

impl DesignFailure {
    fn into_take_error(self, document: &Path) -> RecordError {
        match self {
            DesignFailure::Parse(detail) => RecordError::Parse {
                path: document.display().to_string(),
                detail,
            },
            DesignFailure::Record(error) => error,
        }
    }
}

impl From<RecordError> for DesignFailure {
    fn from(error: RecordError) -> Self {
        DesignFailure::Record(error)
    }
}

/// Every file a parse of `document` reads, pinned relative to `root`.
fn read_design(root: &Path, document: &Path) -> Result<Vec<InputPin>, DesignFailure> {
    let mut parser = SCXMLParser::new();
    parser
        .parse_file(&document.display().to_string())
        .map_err(|located| DesignFailure::Parse(located.error.to_string()))?;
    let mut pins: BTreeMap<String, String> = BTreeMap::new();
    for path in std::iter::once(document.to_path_buf()).chain(parser.preprocessor_deps().to_vec()) {
        let path = canonical(&path)?;
        pins.insert(relative(root, &path)?, file_sha256(&path)?);
    }
    Ok(pins
        .into_iter()
        .map(|(path, sha256)| InputPin { path, sha256 })
        .collect())
}

fn canonical(path: &Path) -> Result<PathBuf, RecordError> {
    std::fs::canonicalize(path).map_err(|source| RecordError::Read {
        path: path.display().to_string(),
        source,
    })
}

fn file_sha256(path: &Path) -> Result<String, RecordError> {
    let bytes = std::fs::read(path).map_err(|source| RecordError::Read {
        path: path.display().to_string(),
        source,
    })?;
    Ok(hex_encode(&sha256_bytes(&bytes)))
}

/// `path` relative to `root`, spelled with `/` on every platform so a
/// record reads the same wherever it was taken.
fn relative(root: &Path, path: &Path) -> Result<String, RecordError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| RecordError::OutsideRoot {
            path: path.display().to_string(),
            root: root.display().to_string(),
        })?;
    Ok(rel
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

fn check_variant(value: &str) -> Result<(), RecordError> {
    if value.is_empty() || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(RecordError::Variant {
            value: value.to_string(),
        });
    }
    Ok(())
}

/// A pinned path must stay under the root it is read against.
fn check_relative(path: &str) -> Result<(), RecordError> {
    let parsed = Path::new(path);
    let escapes = parsed
        .components()
        .any(|part| !matches!(part, Component::Normal(_)));
    if path.is_empty() || escapes || path.contains('\\') {
        return Err(RecordError::Format {
            detail: format!("`{path}` is not a path relative to the record's root"),
        });
    }
    Ok(())
}

fn check_sha256(digest: &str) -> Result<(), RecordError> {
    let well_formed = digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !well_formed {
        return Err(RecordError::Format {
            detail: format!("`{digest}` is not a lowercase sha256"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn wire(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value = serde_json::json!({
            "record": RECORD_KIND,
            "v": RECORD_VERSION,
            "variant": "base",
            "document": "machine.scxml",
            "manifest": {"path": "spec.manifest.json", "doc_id": "SPEC", "rev": "D3", "sha256": SHA},
            "inputs": [{"path": "machine.scxml", "sha256": SHA}],
        });
        edit(&mut value);
        value.to_string()
    }

    /// The control for every refusal below: the same bytes, unedited, load.
    #[test]
    fn a_record_this_module_would_write_loads() {
        let record = AcceptanceRecord::from_json(&wire(|_| {})).expect("loads");
        assert_eq!(record.variant, "base");
        assert_eq!(
            AcceptanceRecord::from_json(&record.to_json()).expect("round-trips"),
            record
        );
    }

    #[test]
    fn bytes_this_module_would_not_write_are_refused() {
        let cases: Vec<(&str, String)> = vec![
            ("an unknown field", wire(|v| v["note"] = "x".into())),
            (
                "another kind",
                wire(|v| v["record"] = "sce-manifest".into()),
            ),
            ("another version", wire(|v| v["v"] = 2.into())),
            (
                "a variant with a space",
                wire(|v| v["variant"] = "base tls".into()),
            ),
            ("an empty variant", wire(|v| v["variant"] = "".into())),
            (
                "an absolute path",
                wire(|v| v["inputs"][0]["path"] = "/etc/passwd".into()),
            ),
            (
                "a path that climbs",
                wire(|v| v["manifest"]["path"] = "../spec.json".into()),
            ),
            (
                "an upper-case digest",
                wire(|v| v["manifest"]["sha256"] = SHA.to_uppercase().replace('0', "A").into()),
            ),
            (
                "a short digest",
                wire(|v| v["inputs"][0]["sha256"] = "abc".into()),
            ),
            (
                "inputs that omit the document",
                wire(|v| v["inputs"][0]["path"] = "other.scxml".into()),
            ),
            (
                "an input pinned twice",
                wire(|v| {
                    v["inputs"] = serde_json::json!([
                        {"path": "machine.scxml", "sha256": SHA},
                        {"path": "machine.scxml", "sha256": SHA},
                    ])
                }),
            ),
        ];
        for (what, bytes) in cases {
            assert!(
                AcceptanceRecord::from_json(&bytes).is_err(),
                "{what} was loaded as a record: {bytes}"
            );
        }
    }
}
