// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An original: the document a spec is brought in from, kept as a bundle (ADR 0032).
//!
//! A word-processor file or a PDF is a bundle of one member; a Markdown file with its pictures, or a
//! folder brought in whole, is a bundle of several. A bundle is a manifest of members, each with a path
//! inside the bundle, the hash of its bytes, its size and a role, and the original is named by the hash
//! of its manifest. This module is the record and the rules about it, with no folder: the store
//! (`store/original_store.rs`) keeps the bytes under the hashes a manifest names.
//!
//! **A path in a bundle stays inside the bundle.** It is relative, written with `/`, and names no
//! `.` or `..` and no empty segment, so a reference cannot climb out of the bundle and a path cannot be
//! made to mean a file of the machine. It is also unique ignoring case, because a bundle is carried
//! between systems that do not tell `Img.png` from `img.png`, and one that keeps both loses one.
//!
//! **A reference that is not there is said, not dropped.** A Markdown file brought in alone has pictures
//! that were not brought. Each is kept in the manifest as a path with no member (`missing`), so that the
//! review can show it as a picture that was not brought, and adding the file later is a new original.
//!
//! **What is derived is a member of its own and never the original.** A rendition is a picture made from
//! another member; it names the member it was made from by hash and says what drew it, and a rendition
//! of a rendition is refused, because the chain would hide which bytes the original holds.
//!
//! **The manifest is canonical.** Members and missing paths are written in path order and the text is
//! the compact form of the record, so the same bundle is the same bytes and so the same name whatever
//! order its members were given in.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::StoreError;
use crate::revision::Revision;

/// The `format` word of a manifest.
pub const MANIFEST_FORMAT: &str = "sce-original";
/// The version of the manifest's form. A change an older reader would misread moves it.
pub const MANIFEST_VERSION: u32 = 1;

/// The most members one original holds. A specification is not an archive.
pub const MAX_MEMBERS: usize = 2000;
/// The most one member may weigh. A larger one is refused and not cut: half a document is another one.
pub const MAX_MEMBER_BYTES: usize = 64 * 1024 * 1024;
/// The most the members of one original may weigh together.
pub const MAX_ORIGINAL_BYTES: usize = 256 * 1024 * 1024;
/// The longest path inside a bundle, in bytes.
pub const MAX_PATH_BYTES: usize = 255;
/// The longest label, in characters.
pub const MAX_LABEL_CHARS: usize = 200;
/// The longest name of what drew a rendition, in bytes.
pub const MAX_RENDERER_BYTES: usize = 64;
/// The most one saved extraction may hold. The form belongs to the authoring tools: the store keeps
/// it under its hash and reads three fields of it to say what it is.
pub const MAX_EXTRACTION_BYTES: usize = 16 * 1024 * 1024;

/// What a member is to the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A document that is read for its text (a word file, a PDF, a Markdown file).
    Document,
    /// A picture of the original, as it was brought.
    Picture,
    /// An object embedded in the original, or brought with it.
    Attachment,
    /// A picture made from another member, never the original itself.
    Rendition,
}

impl Role {
    pub fn word(self) -> &'static str {
        match self {
            Role::Document => "document",
            Role::Picture => "picture",
            Role::Attachment => "attachment",
            Role::Rendition => "rendition",
        }
    }
}

/// One member of a bundle, as the manifest says it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub path: String,
    pub hash: Revision,
    pub size: u64,
    pub role: Role,
    /// For a rendition: the hash of the member it was made from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<Revision>,
    /// For a rendition: what drew it, with its version (`pdfium 7.0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<String>,
}

/// The manifest of an original: what it holds, and what it names and does not hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub v: u32,
    pub label: String,
    pub members: Vec<Member>,
    /// Paths the original refers to and was not brought with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
}

impl Manifest {
    /// The text the manifest is named by: the compact form of the record, in path order.
    pub fn canonical_text(&self) -> String {
        serde_json::to_string(self).expect("a manifest is always JSON")
    }

    /// The name of the original.
    pub fn revision(&self) -> Revision {
        Revision::of(self.canonical_text().as_bytes())
    }

    /// The bytes the members hold together.
    pub fn bytes(&self) -> u64 {
        self.members.iter().map(|member| member.size).sum()
    }

    /// The member at `path`, if the original holds one.
    pub fn member(&self, path: &str) -> Option<&Member> {
        self.members.iter().find(|member| member.path == path)
    }

    /// Read a manifest from the text the store kept, refusing one that is not a manifest of this form.
    /// The text is held to the rules it was made under: a file that was edited by hand and now names
    /// a path that climbs out of the bundle is refused, as it would have been when it was added.
    pub fn read(text: &str) -> Result<Manifest, String> {
        let manifest: Manifest = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if manifest.format != MANIFEST_FORMAT {
            return Err(format!(
                "its format is `{}`, not `{MANIFEST_FORMAT}`",
                manifest.format
            ));
        }
        if manifest.v != MANIFEST_VERSION {
            return Err(format!(
                "it is of version {}; this build reads version {MANIFEST_VERSION}",
                manifest.v
            ));
        }
        let mut paths: Vec<&str> = manifest.members.iter().map(|m| m.path.as_str()).collect();
        paths.extend(manifest.missing.iter().map(String::as_str));
        check_paths(&paths).map_err(|(_, why)| why)?;
        Ok(manifest)
    }
}

/// A member as a caller brings it: the bytes, and what they are to the original.
#[derive(Debug, Clone)]
pub struct MemberDraft {
    pub path: String,
    pub role: Role,
    pub bytes: Vec<u8>,
    /// For a rendition: the path of the member it was made from.
    pub of: Option<String>,
    /// For a rendition: what drew it.
    pub renderer: Option<String>,
}

impl MemberDraft {
    pub fn new(path: impl Into<String>, role: Role, bytes: Vec<u8>) -> Self {
        MemberDraft {
            path: path.into(),
            role,
            bytes,
            of: None,
            renderer: None,
        }
    }

    /// A rendition of the member at `of`, drawn by `renderer`.
    pub fn rendition(
        path: impl Into<String>,
        bytes: Vec<u8>,
        of: impl Into<String>,
        renderer: impl Into<String>,
    ) -> Self {
        MemberDraft {
            path: path.into(),
            role: Role::Rendition,
            bytes,
            of: Some(of.into()),
            renderer: Some(renderer.into()),
        }
    }
}

/// An original as a caller brings it.
#[derive(Debug, Clone, Default)]
pub struct OriginalDraft {
    pub label: String,
    pub members: Vec<MemberDraft>,
    pub missing: Vec<String>,
}

/// What a draft comes to: its manifest, and the bytes to keep under each hash it names.
#[derive(Debug)]
pub struct Built {
    pub manifest: Manifest,
    pub blobs: Vec<(Revision, Vec<u8>)>,
}

fn refused(message: impl Into<String>, path: Option<&str>) -> StoreError {
    StoreError::refused(
        "bad-original",
        message,
        path.map_or(serde_json::Value::Null, |path| json!({ "path": path })),
    )
}

/// Why `path` cannot be a path inside a bundle, or `None` when it can.
pub fn path_fault(path: &str) -> Option<String> {
    if path.is_empty() {
        return Some("is empty".to_string());
    }
    if path.len() > MAX_PATH_BYTES {
        return Some(format!("is longer than {MAX_PATH_BYTES} bytes"));
    }
    if path.starts_with('/') {
        return Some("is absolute; a path is relative to the bundle".to_string());
    }
    if path.contains('\\') {
        return Some("has a backslash; a path is written with `/`".to_string());
    }
    if path.chars().any(|c| c.is_control()) {
        return Some("has a control character".to_string());
    }
    for segment in path.split('/') {
        match segment {
            "" => return Some("has an empty segment".to_string()),
            "." | ".." => {
                return Some(format!("names `{segment}`, which could leave the bundle"));
            }
            _ => {}
        }
    }
    None
}

/// The paths all fit, and no two are the same ignoring case.
fn check_paths(paths: &[&str]) -> Result<(), (String, String)> {
    let mut seen = std::collections::BTreeMap::<String, &str>::new();
    for path in paths {
        if let Some(fault) = path_fault(path) {
            return Err((path.to_string(), format!("the path `{path}` {fault}")));
        }
        if let Some(earlier) = seen.insert(path.to_lowercase(), path) {
            let same = if earlier == *path {
                "twice"
            } else {
                "twice, differing only in case"
            };
            return Err((
                path.to_string(),
                format!("the path `{path}` is in the bundle {same} (`{earlier}`)"),
            ));
        }
    }
    Ok(())
}

impl OriginalDraft {
    /// The manifest this draft is, and the bytes it holds, or why it is not an original.
    pub fn build(self) -> Result<Built, StoreError> {
        if self.label.chars().count() > MAX_LABEL_CHARS {
            return Err(refused(
                format!("a label is at most {MAX_LABEL_CHARS} characters"),
                None,
            ));
        }
        if self.members.is_empty() {
            return Err(refused(
                "an original holds at least one member: a bundle of nothing is not the document it was brought in as",
                None,
            ));
        }
        if self.members.len() > MAX_MEMBERS {
            return Err(refused(
                format!(
                    "an original holds at most {MAX_MEMBERS} members; this has {}",
                    self.members.len()
                ),
                None,
            ));
        }
        let mut paths: Vec<&str> = self.members.iter().map(|m| m.path.as_str()).collect();
        paths.extend(self.missing.iter().map(String::as_str));
        check_paths(&paths).map_err(|(path, why)| refused(why, Some(&path)))?;

        let mut total = 0usize;
        for member in &self.members {
            if member.bytes.len() > MAX_MEMBER_BYTES {
                return Err(StoreError::TooLarge {
                    what: "a member of an original",
                    bytes: member.bytes.len(),
                    limit: MAX_MEMBER_BYTES,
                });
            }
            total += member.bytes.len();
        }
        if total > MAX_ORIGINAL_BYTES {
            return Err(StoreError::TooLarge {
                what: "an original",
                bytes: total,
                limit: MAX_ORIGINAL_BYTES,
            });
        }

        let hashes: std::collections::BTreeMap<&str, Revision> = self
            .members
            .iter()
            .map(|member| (member.path.as_str(), Revision::of(&member.bytes)))
            .collect();
        let mut members = Vec::with_capacity(self.members.len());
        let mut blobs: Vec<(Revision, Vec<u8>)> = Vec::new();
        for draft in &self.members {
            let (of, renderer) = match draft.role {
                Role::Rendition => {
                    let source = draft.of.as_deref().ok_or_else(|| {
                        refused(
                            format!(
                                "the rendition `{}` does not say which member it was made from",
                                draft.path
                            ),
                            Some(&draft.path),
                        )
                    })?;
                    let made_from = self
                        .members
                        .iter()
                        .find(|member| member.path == source)
                        .ok_or_else(|| {
                            refused(
                                format!(
                                    "the rendition `{}` is made from `{source}`, which is not a member",
                                    draft.path
                                ),
                                Some(&draft.path),
                            )
                        })?;
                    if made_from.role == Role::Rendition {
                        return Err(refused(
                            format!(
                                "the rendition `{}` is made from the rendition `{source}`; a rendition is made from what the original holds",
                                draft.path
                            ),
                            Some(&draft.path),
                        ));
                    }
                    let renderer = draft
                        .renderer
                        .as_deref()
                        .filter(|r| !r.is_empty())
                        .ok_or_else(|| {
                            refused(
                                format!("the rendition `{}` does not say what drew it", draft.path),
                                Some(&draft.path),
                            )
                        })?;
                    if renderer.len() > MAX_RENDERER_BYTES {
                        return Err(refused(
                            format!("the name of what drew a rendition is at most {MAX_RENDERER_BYTES} bytes"),
                            Some(&draft.path),
                        ));
                    }
                    (Some(hashes[source].clone()), Some(renderer.to_string()))
                }
                _ => {
                    if draft.of.is_some() || draft.renderer.is_some() {
                        return Err(refused(
                            format!(
                                "`{}` is a {}, and only a rendition is made from another member",
                                draft.path,
                                draft.role.word()
                            ),
                            Some(&draft.path),
                        ));
                    }
                    (None, None)
                }
            };
            let hash = hashes[draft.path.as_str()].clone();
            if !blobs.iter().any(|(held, _)| *held == hash) {
                blobs.push((hash.clone(), draft.bytes.clone()));
            }
            members.push(Member {
                path: draft.path.clone(),
                hash,
                size: draft.bytes.len() as u64,
                role: draft.role,
                of,
                renderer,
            });
        }
        members.sort_by(|a, b| a.path.cmp(&b.path));
        let mut missing = self.missing;
        missing.sort();
        Ok(Built {
            manifest: Manifest {
                format: MANIFEST_FORMAT.to_string(),
                v: MANIFEST_VERSION,
                label: self.label,
                members,
                missing,
            },
            blobs,
        })
    }
}
