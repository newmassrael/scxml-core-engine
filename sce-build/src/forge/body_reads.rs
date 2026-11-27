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
//!
//! Which reads are on a build's hot path is not a fact of the set but of the
//! build's workload, which neither a document nor the generator knows: the
//! build names the codecs its hot path reads ([`InlineDecode::Codecs`],
//! docs/adr/0139), each by its document's stem or by an alias an import of the
//! set gives it, checked against the set ([`BodyReadSites::hot_codecs`]) so a
//! name that is no codec of it, or names two, is refused.
//!
//! Hotness is a property of a site, the read of one codec's body inside
//! another's, not of a codec (docs/adr/0140): the body of a codec the build
//! names is hot, and so is every read a hot body makes, through the codecs it
//! reads, transitively ([`HotCodecs`]). Every body is printed in its entry
//! points, whose reads are cold sites and call the entry points of what it
//! holds, and which keep the rule of the cold sites that call them
//! ([`BodyReadForms`]); a hot body is printed a second time, from the same
//! text, as forms held in place where they are called, which every hot site
//! calls and whose reads call the forms held in place of what it holds. A body
//! read from a hot and from a cold site so has its code once in each hot site
//! and once in its entry point, and a cold read of it expands no hot body. The
//! walk of a list is a cold site whatever the body that holds the list: a read
//! a consumer makes after the frame is read.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::forge::error::{ForgeError, GenerateError};
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

impl BodyReads {
    /// These sites and `other`'s.
    fn and(self, other: Self) -> Self {
        Self {
            eager: self.eager + other.eager,
            view: self.view + other.view,
        }
    }

    /// These sites but `other`'s, a part of them.
    fn but(self, other: Self) -> Self {
        Self {
            eager: self.eager.saturating_sub(other.eager),
            view: self.view.saturating_sub(other.view),
        }
    }
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
/// codec's name, the sites of each body that read each codec it holds, the name
/// of every codec of the set and the codecs each import alias of it names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BodyReadSites {
    /// The sites that read each codec's body; a codec no site reads has no
    /// entry.
    reads: BTreeMap<String, BodyReads>,
    /// The sites of each codec's body that read each codec it holds, by the
    /// holder's name and then the held codec's: the edges the hot closure is
    /// walked along (docs/adr/0140). The walk of a list is no site of the body
    /// that holds the list. A codec that reads no body has no entry.
    children: BTreeMap<String, BTreeMap<String, BodyReads>>,
    /// The name of every codec of the set, read by a site of it or not.
    codecs: BTreeSet<String>,
    /// The codecs of the set each import alias of a codec of the set names, by
    /// the alias: one, unless two imports give one alias to two codecs.
    aliases: BTreeMap<String, BTreeSet<String>>,
}

impl BodyReadSites {
    /// The sites that read the body of the codec named `codec`: none where the
    /// set reads it nowhere, or holds no codec of that name.
    pub fn of(&self, codec: &str) -> BodyReads {
        self.reads.get(codec).copied().unwrap_or_default()
    }

    /// Whether the set holds a codec named `codec`.
    pub fn holds(&self, codec: &str) -> bool {
        self.codecs.contains(codec)
    }

    /// The codecs of the set named by `names`, which a build names because its
    /// hot path reads them, and every codec a hot body reads, transitively
    /// (docs/adr/0139, docs/adr/0140): the one way a [`HotCodecs`] is made, so
    /// a name reaches a generation only once the set it names a codec of has
    /// been asked.
    ///
    /// A codec is named by its document's stem, the name the count keeps it
    /// by, or by an alias an import of the set gives it; both resolve to the
    /// one codec. A name that is no codec of the set is refused
    /// (`codegen/inline-decode-codec-not-in-set`), and so is one that names two
    /// (`codegen/inline-decode-codec-ambiguous`), the first such one as the
    /// names are given: a name misspelt or read as another codec would
    /// otherwise leave the hot read it meant out of line, and the build would
    /// measure the slower image with no word said of why. A name given twice,
    /// or a codec given by its stem and by an alias, is one codec.
    pub fn hot_codecs<I>(&self, names: I) -> Result<HotCodecs, ForgeError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut named = BTreeSet::new();
        for name in names {
            named.insert(self.resolve(name.as_ref())?);
        }
        // The hot bodies: the codecs named, and each codec a hot body reads.
        let mut bodies: BTreeSet<String> = BTreeSet::new();
        let mut pending: Vec<&str> = named.iter().map(String::as_str).collect();
        while let Some(codec) = pending.pop() {
            if !bodies.insert(codec.to_string()) {
                continue;
            }
            if let Some(held) = self.children.get(codec) {
                pending.extend(held.keys().map(String::as_str));
            }
        }
        // The sites of the hot bodies that read each codec, which call the
        // forms held in place and not the entry points.
        let mut sites: BTreeMap<String, BodyReads> = BTreeMap::new();
        for held in bodies.iter().filter_map(|body| self.children.get(body)) {
            for (codec, reads) in held {
                let hot = sites.entry(codec.clone()).or_default();
                *hot = hot.and(*reads);
            }
        }
        Ok(HotCodecs {
            named,
            bodies,
            sites,
        })
    }

    /// The one codec of the set `name` names, its document's stem or an alias
    /// an import of the set gives it, or the refusal of a name that names none
    /// or two.
    fn resolve(&self, name: &str) -> Result<String, ForgeError> {
        let mut named: BTreeSet<&str> = self
            .aliases
            .get(name)
            .map(|codecs| codecs.iter().map(String::as_str).collect())
            .unwrap_or_default();
        if self.holds(name) {
            named.insert(name);
        }
        let mut found = named.iter();
        match (found.next(), found.next()) {
            (Some(codec), None) => Ok((*codec).to_string()),
            (None, _) => Err(ForgeError::from(
                GenerateError::CodegenInlineDecodeCodecNotInSet {
                    codec: name.to_string(),
                    set: self.codecs.iter().cloned().collect(),
                },
            )),
            (Some(_), Some(_)) => Err(ForgeError::from(
                GenerateError::CodegenInlineDecodeCodecAmbiguous {
                    name: name.to_string(),
                    codecs: named.iter().map(|codec| (*codec).to_string()).collect(),
                },
            )),
        }
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
    ///
    /// The sites are kept by the body that holds them too, which the hot
    /// closure is walked along, and each import alias by the codecs it names,
    /// which a build may name a codec by ([`Self::hot_codecs`]). A body holds
    /// every site but the walk of a list: a walk is a read a consumer makes of
    /// the list after the frame is read, by the entry's entry point, and no read
    /// the body makes, so a hot body makes it no hot site (docs/adr/0140).
    pub fn count(set: &[SetCodec<'_>]) -> Self {
        let codecs: BTreeSet<String> = set.iter().map(|member| member.codec.name.clone()).collect();
        let own_view: BTreeMap<&str, bool> = set
            .iter()
            .map(|member| (member.codec.name.as_str(), member.codec.has_own_view()))
            .collect();
        let mut sites: BTreeMap<String, BodyReads> = BTreeMap::new();
        let mut children: BTreeMap<String, BTreeMap<String, BodyReads>> = BTreeMap::new();
        let mut aliases: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for parent in set {
            let codec = parent.codec;
            // The name a build may give each codec of the set the parent
            // imports, beside its document's stem (docs/adr/0140).
            for (alias, name) in parent.imports {
                if own_view.contains_key(name.as_str()) {
                    aliases
                        .entry(alias.clone())
                        .or_default()
                        .insert(name.clone());
                }
            }
            // (alias, sites of the parent's view that read it, those of them
            // its body holds): a list's walk is no site of the body.
            let arms = codec
                .variant
                .iter()
                .flat_map(|v| v.arms.iter().chain(v.default_arm.iter()))
                .map(|arm| (arm.body_alias.as_str(), 1u32, 1u32));
            let fields = codec.fields.iter().filter_map(|f| {
                if let Some(alias) = f.embed_body_alias.as_deref() {
                    return Some((alias, 1u32, 1u32));
                }
                f.repeat_body_alias
                    .as_deref()
                    .or(f.tlv_chain_body_alias.as_deref())
                    .map(|alias| (alias, 2u32, 1u32))
            });
            for (alias, view_sites, body_view_sites) in arms.chain(fields) {
                let Some(name) = parent.imports.get(alias) else {
                    continue;
                };
                let Some(&has_own_view) = own_view.get(name.as_str()) else {
                    continue;
                };
                // The sites that read the body, and those the parent's body
                // holds: a codec that is its own view is read by a parent's
                // view through its eager read.
                let read_at = |view: u32| {
                    let mut reads = BodyReads { eager: 1, view: 0 };
                    if parent.views {
                        if has_own_view {
                            reads.view += view;
                        } else {
                            reads.eager += view;
                        }
                    }
                    reads
                };
                let total = sites.entry(name.clone()).or_default();
                *total = total.and(read_at(view_sites));
                let held = children
                    .entry(codec.name.clone())
                    .or_default()
                    .entry(name.clone())
                    .or_default();
                *held = held.and(read_at(body_view_sites));
            }
        }
        Self {
            reads: sites,
            children,
            codecs,
            aliases,
        }
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

/// The codecs of a generated set a build names because its hot path reads
/// them, each checked to be a codec of the set ([`BodyReadSites::hot_codecs`],
/// the one way one is made; docs/adr/0139), and what that makes hot
/// (docs/adr/0140): the body of each codec named, and each read a hot body
/// makes, transitively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotCodecs {
    /// The codecs the build names, by their documents' stems.
    named: BTreeSet<String>,
    /// The codecs whose bodies are read on the hot path: those named and every
    /// codec a hot body reads.
    bodies: BTreeSet<String>,
    /// The sites of the hot bodies that read each codec's body; a codec no hot
    /// body reads has no entry.
    sites: BTreeMap<String, BodyReads>,
}

impl HotCodecs {
    /// Whether the build names the codec whose document's stem is `codec`.
    pub fn named(&self, codec: &str) -> bool {
        self.named.contains(codec)
    }

    /// Whether the body of the codec `codec` is read on the hot path: it is
    /// named, or a hot body reads it.
    pub fn is_hot(&self, codec: &str) -> bool {
        self.bodies.contains(codec)
    }

    /// The sites of the hot bodies that read the body of the codec `codec`,
    /// which call the forms held in place.
    pub fn sites_of(&self, codec: &str) -> BodyReads {
        self.sites.get(codec).copied().unwrap_or_default()
    }
}

/// How the generation of one codec prints the functions that read its body
/// (docs/adr/0137, docs/adr/0139, docs/adr/0140), as [`InlineDecode::forms`]
/// answers it. The default is the body read of ADR 0137: one function, placed
/// by the sites that call it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BodyReadForms {
    /// Whether the body is read on the build's hot path ([`HotCodecs::is_hot`]).
    /// Its code is printed twice from its one text: in its entry points
    /// (`read_into`, `read`, `read_with`), which read the codecs it holds and
    /// its ZInts by their entry points, and in the forms held in place where
    /// they are called (`read_into_inline`, `read_inline`, `read_with_inline`),
    /// which read them by their forms held in place. No entry point calls a
    /// form held in place.
    pub hot: bool,
    /// Whether the build names its hot roots and the body is not read on its
    /// hot path ([`InlineDecode::Codecs`]): read once per event, so where its
    /// fields allow it the body is read by the runtime's one walker over a
    /// table of its fields (docs/adr/0155). Never set with [`Self::hot`].
    pub cold: bool,
    /// Whether every entry point is asked inlined (`--inline-decode`,
    /// [`InlineDecode::Every`]).
    pub every: bool,
    /// The sites of the set that call the entry points, which place them by
    /// ADR 0137's rule: every site that reads the body but those hot bodies
    /// hold, which call the forms held in place; the walks of lists among
    /// them, whatever the body that holds the list.
    pub entry: BodyReads,
}

/// Which body reads a build asks inlined (docs/adr/0136 section 4,
/// docs/adr/0139, docs/adr/0140). Like `inline_encode` it is the build's
/// declaration: what a build's hot path reads is a fact of its workload, and
/// nothing in a document says it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InlineDecode {
    /// No read is asked inlined: each body read is placed by the sites of its
    /// set that read it (docs/adr/0137). The default.
    #[default]
    None,
    /// Every body read (`generate --inline-decode`, docs/adr/0136 section 4):
    /// the one function that reads each body is `#[inline(always)]`. A
    /// property of one document, which knows nothing of the forms its imports
    /// print, so no form held in place is asked of them.
    Every,
    /// The reads of the hot path of a set (`generate --with-imports
    /// --inline-decode-for`, docs/adr/0139, docs/adr/0140): the hot bodies are
    /// printed in forms held in place, which every hot site calls, and every
    /// other site calls an entry point placed by the rule of [`Self::None`]. A
    /// property of a set, whose every document is generated with it.
    Codecs(HotCodecs),
}

impl InlineDecode {
    /// Whether the body of the codec `codec` is read on the build's hot path
    /// (docs/adr/0140): the build names hot codecs and `codec` is in their
    /// closure ([`HotCodecs::is_hot`]). Neither [`Self::None`] nor
    /// [`Self::Every`] makes a body hot: `--inline-decode` prints no form held
    /// in place.
    pub fn is_hot(&self, codec: &str) -> bool {
        matches!(self, Self::Codecs(hot) if hot.is_hot(codec))
    }

    /// How the generation of the codec `codec` prints the functions that read
    /// its body, `sites` being the count of the set it is generated in, the
    /// one [`BodyReadSites::hot_codecs`] was asked of.
    pub fn forms(&self, codec: &str, sites: &BodyReadSites) -> BodyReadForms {
        let all = sites.of(codec);
        match self {
            Self::None => BodyReadForms {
                entry: all,
                ..BodyReadForms::default()
            },
            Self::Every => BodyReadForms {
                every: true,
                entry: all,
                ..BodyReadForms::default()
            },
            Self::Codecs(hot) => BodyReadForms {
                hot: hot.is_hot(codec),
                cold: !hot.is_hot(codec),
                every: false,
                entry: all.but(hot.sites_of(codec)),
            },
        }
    }
}
