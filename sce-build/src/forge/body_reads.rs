// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How many sites of a generated codec set read each codec's body, which is what
//! decides whether the function that reads the body is kept out of line
//! (docs/adr/0137).
//!
//! A body read from one site is best left to the compiler: kept out of line it
//! is a call frame its one caller pays for and a function the caller cannot
//! inline, and the copy it would save does not exist. A body the set reads from
//! two sites or more (an arm two dispatching codecs share, a body several codecs
//! embed, the entry of a list a view both scans and walks) is one function, out
//! of line, so it is not copied into each site. A site is a call in the code the
//! set generates, not a call a consumer writes: the decode a consumer calls is
//! the codec's own wrapper, which a consumer's build inlines or not as it calls
//! it.
//!
//! What a set reads is a fact of the set, not of one document: a document does
//! not know which documents import it. It is counted once, here, from the
//! codecs of the set and the codec each of their imports names
//! ([`BodyReadSites::count`]), and handed to the generation of each codec
//! ([`crate::ForgeCompileOptions::body_reads`]). A document generated alone is a
//! set of one, which reads no body of its own.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::forge::model::{CodecModel, ForgeDocument, ForgeKind};
use crate::forge::stdlib;
use crate::DocumentLabel;

/// The sites of a set that read one codec's body, by the function each calls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BodyReads {
    /// The sites that call the codec's eager body read: `read_into` of a codec
    /// that reads into its destination, `read` of one built as a value. A codec
    /// that is its own view (`{Codec}View` an alias of `{Codec}`) is read by
    /// the views of the set through the same function, so their sites are
    /// counted here too.
    pub eager: u32,
    /// The sites that call the `read_with` of a codec's view of its own.
    pub view: u32,
}

/// A codec of a generated set, with what the count reads of it beside its
/// model.
#[derive(Debug, Clone, Copy)]
pub struct SetCodec<'a> {
    /// The codec.
    pub codec: &'a CodecModel,
    /// The name of the codec each import alias of [`Self::codec`] names.
    pub imports: &'a BTreeMap<String, String>,
    /// Whether the codec is generated with its view (`generate --views`), whose
    /// read reads the view of each codec it holds.
    pub views: bool,
}

/// The sites of a generated set that read each of its codecs' bodies, by the
/// codec's name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BodyReadSites(BTreeMap<String, BodyReads>);

impl BodyReadSites {
    /// The sites that read the body of the codec named `codec`: none where the
    /// set reads it nowhere, or holds no codec of that name.
    pub fn of(&self, codec: &str) -> BodyReads {
        self.0.get(codec).copied().unwrap_or_default()
    }

    /// Count, for each codec `set` imports, the sites of the set that read its
    /// body: the one count of the set's import graph (docs/adr/0137).
    ///
    /// Each arm of a variant and its default arm is a site, and so is each
    /// embedded body; a list's entry (`<sce:repeat>`, `<sce:tlv-chain>`) is read
    /// at one site of the eager decode (its fill, or the rule of its chain) and
    /// at two of a view, the scan that counts the entries and the walk that
    /// yields them. A codec that is its own view is read by a parent's view
    /// through its eager body read, so those sites are counted as eager ones. An
    /// alias that names no codec of the set is no site of it.
    pub fn count(set: &[SetCodec<'_>]) -> Self {
        let own_view: BTreeMap<&str, bool> = set
            .iter()
            .map(|member| (member.codec.name.as_str(), member.codec.has_own_view()))
            .collect();
        let mut sites: BTreeMap<String, BodyReads> = BTreeMap::new();
        for parent in set {
            let codec = parent.codec;
            // (alias, sites of the parent's view that read it)
            let arms = codec
                .variant
                .iter()
                .flat_map(|v| v.arms.iter().chain(v.default_arm.iter()))
                .map(|arm| (arm.body_alias.as_str(), 1u32));
            let fields = codec.fields.iter().filter_map(|f| {
                if let Some(alias) = f.embed_body_alias.as_deref() {
                    return Some((alias, 1u32));
                }
                f.repeat_body_alias
                    .as_deref()
                    .or(f.tlv_chain_body_alias.as_deref())
                    .map(|alias| (alias, 2u32))
            });
            for (alias, view_sites) in arms.chain(fields) {
                let Some(name) = parent.imports.get(alias) else {
                    continue;
                };
                let Some(&has_own_view) = own_view.get(name.as_str()) else {
                    continue;
                };
                let reads = sites.entry(name.clone()).or_default();
                reads.eager += 1;
                if parent.views {
                    if has_own_view {
                        reads.view += view_sites;
                    } else {
                        reads.eager += view_sites;
                    }
                }
            }
        }
        Self(sites)
    }

    /// [`Self::count`] of the codecs `documents` hold and every codec they
    /// import, each document read once however many import it, its fragments
    /// expanded from `include_dirs` as its generation expands them. `views` says
    /// of each document, by its identity ([`stdlib::identity`]), whether it is
    /// generated with its view.
    ///
    /// A document that is no codec, or that cannot be read or parsed, holds no
    /// site: its own generation is what refuses it, and a site the count misses
    /// only leaves a body's read to the compiler.
    pub fn of_documents(
        documents: &[String],
        include_dirs: &[PathBuf],
        views: impl Fn(&str) -> bool,
    ) -> Self {
        struct ReadCodec {
            identity: String,
            codec: CodecModel,
            /// Each codec import's alias and the identity of the document it
            /// names.
            imports: Vec<(String, String)>,
        }
        let mut pending: VecDeque<PathBuf> = documents.iter().map(PathBuf::from).collect();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut read: Vec<ReadCodec> = Vec::new();
        while let Some(path) = pending.pop_front() {
            let identity = stdlib::identity(&path);
            if !seen.insert(identity.clone()) {
                continue;
            }
            let Ok(loaded) = crate::load_forge_source(&path, include_dirs) else {
                continue;
            };
            let spelled = path.to_string_lossy();
            let Ok(Some(parsed)) = crate::forge::parser::parse_forge_with_imports(
                loaded.text(),
                DocumentLabel::for_input_path(&spelled),
            ) else {
                continue;
            };
            let ForgeDocument::Codec(codec) = parsed.document else {
                continue;
            };
            let base = path.parent().unwrap_or_else(|| Path::new(""));
            let mut imports = Vec::new();
            for import in parsed
                .imports
                .iter()
                .filter(|i| matches!(i.kind, ForgeKind::Codec))
            {
                let imported = stdlib::resolve(base, &import.src);
                imports.push((import.alias.clone(), stdlib::identity(&imported)));
                pending.push_back(imported);
            }
            read.push(ReadCodec {
                identity,
                codec,
                imports,
            });
        }
        let names: BTreeMap<&str, &str> = read
            .iter()
            .map(|r| (r.identity.as_str(), r.codec.name.as_str()))
            .collect();
        let imports: Vec<BTreeMap<String, String>> = read
            .iter()
            .map(|r| {
                r.imports
                    .iter()
                    .filter_map(|(alias, identity)| {
                        names
                            .get(identity.as_str())
                            .map(|name| (alias.clone(), (*name).to_string()))
                    })
                    .collect()
            })
            .collect();
        let set: Vec<SetCodec<'_>> = read
            .iter()
            .zip(&imports)
            .map(|(r, imports)| SetCodec {
                codec: &r.codec,
                imports,
                views: views(&r.identity),
            })
            .collect();
        Self::count(&set)
    }
}
