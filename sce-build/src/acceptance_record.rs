// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! # What the design was authored from
//!
//! ```text
//!   authored_from   the specification files and    an author asked again for
//!                   the owner's decision record,   the same specification is
//!                   each by sha256                 handed this design instead
//!                                                  of a new draft
//! ```
//!
//! Optional, and absent from a record taken without it, which then keeps
//! the bytes it always had. The model that wrote the draft runs in the
//! owner's client, where nothing makes two drafts equal, so the only way a
//! second request with the same inputs gets the same design is to be given
//! the one already accepted — and "the same inputs" is a question about
//! these files. A record pins them so that a revised specification lapses
//! the acceptance as surely as an edited document does, and so that
//! [`AcceptanceRecord::recheck_for`] can say whether a specification it is
//! asked about is the one this design was authored from.
//!
//! The authoring profile ([`crate::authoring_profile`]) is the third of them,
//! under [`crate::acceptance_record::SourceRole::Profile`], and is not an
//! input to the prose: it is what the finished design was judged against. It
//! is here because an acceptance is the owner's statement that THIS design is
//! what they accept, and a design accepted under one profile — say, one that
//! requires a closed interface — is not the answer for a profile that asks
//! something else. The same specification and the same decisions under another
//! profile are a different request, so they get a different design and a
//! different acceptance, and the digest is the whole of what makes them
//! different.
//!
//! # What was still open when a person accepted
//!
//! ```text
//!   open_at_acceptance   the questions, assumed    a reader of the record
//!                        values, needed parent     sees what the owner
//!                        and host processor the    accepted WITH, in the
//!                        design left to a person   words the report used
//! ```
//!
//! The product accepts a design that leaves a question open — it is valid
//! SCXML, and a draft in progress is meant to — and an acceptance that did
//! not say so read, later, as though the design had been finished. Optional
//! and omitted when empty, so a record for a finished design keeps the bytes
//! it always had. It states and does not enforce: accepting with an open
//! question is the owner's decision, and this only keeps the record honest
//! about it.
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

/// The record's stability, as `SCE_WIRE_CONTRACTS.md` states it. The record
/// has no schema file, so the registry's row and this constant are the two
/// places the status lives, and `cli_acceptance_record_wire` holds them
/// together.
pub const ACCEPTANCE_RECORD_STATUS: &str = "pre-release";

/// What an [`authored_from`](AcceptanceRecord::authored_from) file is to the
/// design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceRole {
    /// The prose the design was written from; there may be several files.
    Specification,
    /// The owner's answers to what the specification left open; one at most.
    Decisions,
    /// The authoring profile the design was held to
    /// ([`crate::authoring_profile`]); one at most.
    Profile,
}

impl SourceRole {
    /// Every role, in the order a record lists them.
    pub const ALL: [SourceRole; 3] = [
        SourceRole::Specification,
        SourceRole::Decisions,
        SourceRole::Profile,
    ];

    /// How the design stands to a file of this role, for a sentence that
    /// says it: a design is written FROM a specification and the owner's
    /// answers, and HELD TO a profile — the profile was not an input to the
    /// prose, it is what the finished design was judged against.
    fn relation(self) -> &'static str {
        match self {
            SourceRole::Specification | SourceRole::Decisions => "authored from",
            SourceRole::Profile => "held to",
        }
    }
}

impl fmt::Display for SourceRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SourceRole::Specification => "specification",
            SourceRole::Decisions => "decision record",
            SourceRole::Profile => "authoring profile",
        })
    }
}

/// One file the design was authored from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub role: SourceRole,
    /// Relative to the record's root.
    pub path: String,
    pub sha256: String,
}

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
    /// The specification, the decision record and the authoring profile the
    /// design was authored from and held to, sorted by role and path. Empty when the record was taken
    /// without them — see the module docs.
    pub authored_from: Vec<SourcePin>,
    /// What the design left to a person when it was accepted: the open
    /// questions, the values chosen without the specification, a parent it
    /// needs, a processor the host must serve ([`crate::open_matters`]).
    /// Empty when it left nothing.
    ///
    /// ⚠ A statement of what a person accepted WITH, not a condition the
    /// record enforces. Accepting a design that leaves a question open is
    /// the owner's decision to make; what this refuses is a record that
    /// reads as though they had not. It cannot lapse: the design that was
    /// open is pinned by its bytes, and a design whose open matters moved
    /// has moved.
    pub open_at_acceptance: Vec<crate::open_matters::OpenMatter>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    authored_from: Vec<SourcePin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    open_at_acceptance: Vec<crate::open_matters::OpenMatter>,
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
    /// A file the design was authored from is gone (`current_sha256` is
    /// `None`) or its bytes moved: the specification or the owner's answers
    /// changed, and the design has not been looked at since.
    Source {
        role: SourceRole,
        path: String,
        recorded_sha256: String,
        current_sha256: Option<String>,
    },
    /// The files a caller asked about are not the ones the design was
    /// authored from — compared by content, so the same specification under
    /// another name still matches. `pinned` is empty when the record does
    /// not say what it was authored from.
    NotAuthoredFrom {
        role: SourceRole,
        pinned: Vec<String>,
        asked: Vec<String>,
    },
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
            Lapse::Source {
                role,
                path,
                current_sha256,
                ..
            } => match current_sha256 {
                None => write!(
                    f,
                    "{path}: the {role} this design was {} is no longer there",
                    role.relation()
                ),
                Some(_) => write!(
                    f,
                    "{path}: the {role} this design was {} changed since it was accepted",
                    role.relation()
                ),
            },
            Lapse::NotAuthoredFrom {
                role,
                pinned,
                asked,
            } => {
                let asked = if asked.is_empty() {
                    "none was given".to_string()
                } else {
                    format!("the one asked about ({}) differs", asked.join(", "))
                };
                if pinned.is_empty() {
                    write!(
                        f,
                        "the record does not say which {role} the design was {}, so it \
                         cannot answer for this one: {asked}",
                        role.relation()
                    )
                } else {
                    write!(
                        f,
                        "the design was {} the {role} {}; {asked}",
                        role.relation(),
                        pinned.join(", ")
                    )
                }
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
    /// The files named as what the design was authored from cannot be
    /// pinned as given: a file named twice, or more than one decision
    /// record.
    Source {
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
            RecordError::Source { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for RecordError {}

impl RecordError {
    /// Which refusal this is, in words that do not carry a path — see
    /// [`crate::requirement_manifest::ManifestError::kind`] for why. A
    /// manifest that would not load keeps the manifest's own kind, so the
    /// same broken manifest is one diagnostic whichever door read it.
    pub fn kind(&self) -> &'static str {
        match self {
            RecordError::Read { .. } => "read",
            RecordError::Parse { .. } => "document-unparseable",
            RecordError::Manifest(inner) => inner.kind(),
            RecordError::OutsideRoot { .. } => "outside-root",
            RecordError::Variant { .. } => "variant",
            RecordError::Format { .. } => "format",
            RecordError::Source { .. } => "source",
        }
    }
}

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
        Self::take_authored(root, document, manifest, variant, &[])
    }

    /// [`Self::take`], also pinning the files the design was authored from
    /// — see the module docs. At most one of them may be a decision record,
    /// and at most one an authoring profile.
    pub fn take_authored(
        root: &Path,
        document: &Path,
        manifest: &Path,
        variant: &str,
        sources: &[(SourceRole, &Path)],
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
        let (inputs, open_at_acceptance) = read_design(&root, &document_path)
            .map_err(|failure| failure.into_take_error(document))?;
        let mut authored_from = Vec::with_capacity(sources.len());
        for (role, path) in sources {
            authored_from.push(SourcePin {
                role: *role,
                path: relative(&root, &canonical(path)?)?,
                sha256: file_sha256(path)?,
            });
        }
        authored_from.sort();
        check_sources(&authored_from)?;
        Ok(AcceptanceRecord {
            variant: variant.to_string(),
            document: relative(&root, &document_path)?,
            manifest: manifest_pin,
            inputs,
            authored_from,
            open_at_acceptance,
        })
    }

    /// [`Self::recheck`], and also whether the files a caller asks about are
    /// the ones the design was authored from.
    ///
    /// Compared by content, role by role: the specification files asked
    /// about must be exactly the pinned ones' bytes, as a set, and so must the
    /// decision record and the authoring profile — including none when none
    /// was pinned. This is the
    /// question "is this accepted design the answer for these inputs", which
    /// a caller asks before writing a new draft of the same specification.
    pub fn recheck_for(
        &self,
        root: &Path,
        variant: &str,
        asked: &[(SourceRole, &Path)],
    ) -> Result<Vec<Lapse>, RecordError> {
        let mut lapses = self.recheck(root, variant)?;
        for role in SourceRole::ALL {
            let pinned: Vec<&SourcePin> = self
                .authored_from
                .iter()
                .filter(|p| p.role == role)
                .collect();
            let mut asked_paths = Vec::new();
            let mut asked_digests = std::collections::BTreeSet::new();
            for (asked_role, path) in asked {
                if *asked_role == role {
                    asked_paths.push(path.display().to_string());
                    asked_digests.insert(file_sha256(path)?);
                }
            }
            let pinned_digests: std::collections::BTreeSet<String> =
                pinned.iter().map(|p| p.sha256.clone()).collect();
            // A caller that names nothing is asking about the design alone,
            // which `recheck` has answered. One that names anything is asking
            // about a set of inputs, and a role it left out is part of the
            // answer: a decision record pinned and not given, or a
            // specification not given, is a different set.
            if !asked.is_empty() && asked_digests != pinned_digests {
                lapses.push(Lapse::NotAuthoredFrom {
                    role,
                    pinned: pinned.iter().map(|p| p.path.clone()).collect(),
                    asked: asked_paths,
                });
            }
        }
        Ok(lapses)
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

        for pin in &self.authored_from {
            let current_sha256 = std::fs::read(root.join(&pin.path))
                .ok()
                .map(|bytes| hex_encode(&sha256_bytes(&bytes)));
            if current_sha256.as_deref() != Some(pin.sha256.as_str()) {
                lapses.push(Lapse::Source {
                    role: pin.role,
                    path: pin.path.clone(),
                    recorded_sha256: pin.sha256.clone(),
                    current_sha256,
                });
            }
        }

        let document_path = root.join(&self.document);
        if !document_path.is_file() {
            lapses.push(Lapse::Missing {
                path: self.document.clone(),
            });
            return Ok(lapses);
        }
        // What was open is a fact about the moment of acceptance and not a
        // thing to re-judge: a design that changed is a lapse by its bytes.
        let current = match read_design(&root, &canonical(&document_path)?) {
            Ok((inputs, _open)) => inputs,
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
            authored_from: self.authored_from.clone(),
            open_at_acceptance: self.open_at_acceptance.clone(),
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
            .chain(wire.authored_from.iter().map(|pin| &pin.path))
        {
            check_relative(path)?;
        }
        for digest in std::iter::once(&wire.manifest.sha256)
            .chain(wire.inputs.iter().map(|pin| &pin.sha256))
            .chain(wire.authored_from.iter().map(|pin| &pin.sha256))
        {
            check_sha256(digest)?;
        }
        let mut authored_from = wire.authored_from;
        authored_from.sort();
        check_sources(&authored_from).map_err(|e| RecordError::Format {
            detail: e.to_string(),
        })?;
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
            authored_from,
            open_at_acceptance: wire.open_at_acceptance,
        })
    }
}

/// The rules an `authored_from` list keeps, sorted: no path twice, and one
/// decision record and one profile at most — the owner's answers are one
/// record, and what a design is held to is one profile, so two of either
/// would leave open which of them a design followed.
fn check_sources(sorted: &[SourcePin]) -> Result<(), RecordError> {
    let mut paths = std::collections::BTreeSet::new();
    for pin in sorted {
        if !paths.insert(pin.path.as_str()) {
            return Err(RecordError::Source {
                detail: format!(
                    "{} is named twice as a file the design was authored from",
                    pin.path
                ),
            });
        }
    }
    for (role, what, followed) in [
        (
            SourceRole::Decisions,
            "decision records",
            "one record of the owner's answers",
        ),
        (
            SourceRole::Profile,
            "authoring profiles",
            "one authoring profile",
        ),
    ] {
        let count = sorted.iter().filter(|p| p.role == role).count();
        if count > 1 {
            return Err(RecordError::Source {
                detail: format!("{count} {what} given; a design follows {followed}"),
            });
        }
    }
    Ok(())
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

/// Every file a parse of `document` reads, pinned relative to `root`, and
/// what the design leaves to a person ([`crate::open_matters`]).
fn read_design(
    root: &Path,
    document: &Path,
) -> Result<(Vec<InputPin>, Vec<crate::open_matters::OpenMatter>), DesignFailure> {
    let mut pins: BTreeMap<String, String> = BTreeMap::new();
    let (files, open) = design_inputs(document)?;
    for path in std::iter::once(document.to_path_buf()).chain(files) {
        let path = canonical(&path)?;
        pins.insert(relative(root, &path)?, file_sha256(&path)?);
    }
    let inputs = pins
        .into_iter()
        .map(|(path, sha256)| InputPin { path, sha256 })
        .collect();
    Ok((inputs, open))
}

/// The files other than `document` that its parse reads, for either
/// pipeline — what an acceptance rests on besides the document itself.
///
/// - A statechart: the fragments its preprocessors spliced in, and the
///   forge documents its `<sce:import>` declarations name (the event
///   schemas and enums its guards and sends are typed by). ⚠ The second
///   half was missing until the forge route below was written: the parser
///   reads those siblings through a path that does not add to
///   `preprocessor_deps`, so a statechart's record did not notice its
///   schema moving.
/// - A forge document: the fragments its expansion read, and the
///   transitive `<sce:import>` closure — the set a forge compile reports
///   as its dependencies.
fn design_inputs(
    document: &Path,
) -> Result<(Vec<PathBuf>, Vec<crate::open_matters::OpenMatter>), DesignFailure> {
    let text = std::fs::read_to_string(document).map_err(|source| {
        DesignFailure::Record(RecordError::Read {
            path: document.display().to_string(),
            source,
        })
    })?;
    let unparseable = |error: String| DesignFailure::Parse(error);
    let base_dir = document.parent().unwrap_or_else(|| Path::new("."));
    match crate::classify_document(&text) {
        crate::Pipeline::Scxml => {
            let mut parser = SCXMLParser::new();
            let model = parser
                .parse_file(&document.display().to_string())
                .map_err(|located| unparseable(located.error.to_string()))?;
            let mut inputs = parser.preprocessor_deps().to_vec();
            inputs.extend(
                model
                    .forge_imports
                    .iter()
                    .map(|import| Path::new(&import.src))
                    .filter(|src| !crate::forge::stdlib::names_standard(src))
                    .map(|src| base_dir.join(src)),
            );
            // The same reading the acceptance report makes, so what a
            // person was shown as open and what the record keeps as open
            // are one list.
            Ok((inputs, crate::open_matters::of_statechart(&model)))
        }
        crate::Pipeline::Forge => {
            let label_text = document.display().to_string();
            let label = crate::DocumentLabel::for_input_path(&label_text);
            let loaded = crate::load_forge_source(document, &[])
                .map_err(|located| unparseable(located.error.to_string()))?;
            let parsed = crate::forge::parser::parse_forge_with_imports(loaded.text(), label)
                .map_err(|located| unparseable(located.error.to_string()))?
                .ok_or_else(|| unparseable("the document is not a forge kind".to_string()))?;
            let imports =
                crate::forge::cross_kind_check::check(&parsed, base_dir, label.diagnostic_label)
                    .map_err(|located| unparseable(located.error.to_string()))?;
            // A forge kind sends to no parent and no processor, so what it
            // leaves open is its markers.
            let markers = crate::unresolved_check::unresolved_records_forge(&loaded.positions)
                .map_err(|located| unparseable(located.error.to_string()))?;
            let open = crate::open_matters::of(&markers, &[], &[], &[]);
            // A `sce:std/...` document is read from the library compiled
            // into the generator; it has no file to pin, and moves only
            // with the generator itself.
            Ok((
                loaded
                    .deps
                    .into_iter()
                    .chain(imports)
                    .filter(|path| !crate::forge::stdlib::names_standard(path))
                    .collect(),
                open,
            ))
        }
    }
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
