//! Spec-provenance, requirement-traceability, and unresolved-placeholder
//! types. Shared by the SCXML statechart model ([`crate::model`]), the
//! located error ([`crate::forge::error::Located`]), and the
//! diagnostic record ([`crate::forge::diagnostic`]) so the same shape
//! carries through every consumer of `<sce:req>` / `<sce:provenance>`
//! / `<sce:unresolved>` annotations.
//!
//! ⚠ The Forge IR ([`crate::forge::model`]) is **not** among them, and
//! the absence is load-bearing rather than an omission: these
//! annotations are read off statechart documents, so a Forge document
//! — a codec, a mesh binding, an event schema — has nowhere to write
//! one. This module's header used to name `forge::model` as a sharer;
//! measured 2026-09-11, that file mentions none of the three types.
//!
//! What it decides is the reach of `SCE_ERROR_CONTRACT.md` §2.1.2. A
//! diagnostic raised about a Forge document cannot carry an enclosing
//! anchor however good the lookup gets, because the document has no
//! anchor to enclose it — which is a different situation from a
//! statechart-document code that simply has no resolver wired to it
//! yet, and the two must not be registered under one reason.
//!
//! Not to be confused with [`crate::forge::provenance`] — that
//! module is the codegen-internal `source_location`-populate guard
//! for the §synth-5-O traceability sourcemap (Atomic 0a). This module is
//! the wire-level metadata family that flows through the
//! parser → IR → diagnostic → codegen pipeline.

use crate::forge::error::SourceLocation;

/// Where inside its source document a requirement or node sits.
///
/// ⭐ A variant, because this half of a coordinate is SOURCE-SHAPED and
/// the landed design had it nailed to one shape. RFC §5.2g measured the
/// corpus a real consumer holds — 146 PDF, 3 xlsx, 1 docx, 1 arxml — and
/// `page: Option<u32>` can only address the first of those. A
/// spreadsheet requirement is at a row, a structured document's at a
/// path; a field named `page` makes every non-paginated source lose the
/// coordinate that makes a review report checkable at all.
///
/// ⚠ The DIVISION a requirement belongs to is deliberately NOT part of
/// this type. That is `section`, it stays a plain key beside this, and
/// the reason is load-bearing: `section` is what
/// [`crate::requirement_manifest::Classification::section_counts`]
/// groups by, and RFC §5.2a calls those per-division counts its only
/// handle on omission. Folding the division into a source-shaped variant
/// would mean each shape had to answer "which group" its own way — and
/// for a path-addressed source there is no honest answer without
/// measuring one, which nobody has. Keeping it out means every source
/// shape answers the grouping question the same way, by naming a
/// division the manifest already declares.
///
/// ⚠⚠ Closed, like the extraction block's fields and for the same
/// reason: an open string here would let a source's identity in through
/// the data, where the executable-code gate cannot see it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Position {
    /// A page of a paginated document.
    Page(u32),
    /// A row of a worksheet.
    Row(u32),
    /// A path into a structured document.
    Path(String),
}

/// The position a compact form's trailing segment spells, if it spells
/// one.
///
/// Two spellings, and the bare one is the compatibility affordance the
/// [`SpecProvenance`] documentation argues for: `page=118` is the
/// explicit form every shape shares, and `118` is what documents written
/// before [`Position`] existed say.
fn position_of(segment: &str) -> Option<Position> {
    let segment = segment.trim();
    Position::parse_tagged(segment).or_else(|| segment.parse().ok().map(Position::Page))
}

impl Position {
    /// The `kind=value` spelling the compact URI form uses.
    ///
    /// Parsed here rather than at the call site so the compact form and
    /// the JSON form cannot drift into naming the same shape two ways.
    pub fn parse_tagged(text: &str) -> Option<Self> {
        let (kind, value) = text.split_once('=')?;
        match (kind.trim(), value.trim()) {
            ("page", v) => v.parse().ok().map(Position::Page),
            ("row", v) => v.parse().ok().map(Position::Row),
            ("path", v) if !v.is_empty() => Some(Position::Path(v.to_string())),
            _ => None,
        }
    }
}

/// Pointer to the source-of-truth specification document anchoring an
/// IR node, requirement ID, or diagnostic.
///
/// SCE never *infers* this — IR generators (hand-authored DSL,
/// NL→IR pipeline, ARXML transcoder) populate it; SCE merely
/// propagates it through model nodes and onto diagnostics. Absent
/// `doc_id` means the producer did not record provenance.
///
/// SCXML serialisation accepts two forms:
///
/// - compact URI: `sce:provenance="OEM-SPEC-01@23#4.4.2"`
///   (`doc_id @ rev # section`; a trailing `:kind=value` after the
///   section carries a [`Position`], e.g. `OEM-SPEC-01#4.4.2:page=118`,
///   `WB-2#Sheet1:row=41`, `AR-1#Pkg:path=/Elem`)
/// - child element: `<sce:provenance doc-id="..." rev="..." section="..." page="..."/>`
///   (one or more allowed; element form lets one node anchor at
///   multiple documents)
///
/// ⚠ The compact form also accepts a bare number — `#4.4.2:118` — and
/// reads it as a page. That spelling predates [`Position`] and is kept
/// because the alternative is worse: with it removed, an unmigrated
/// document does not fail, it silently re-reads `4.4.2:118` as a section
/// id, and a coordinate that quietly becomes part of a division name is
/// the kind of wrong nobody sees. Every shape is expressible in the
/// explicit spelling, and the committed documents use it.
///
/// `doc-id` is the element form's only required attribute — it is the
/// decomposed spelling of the compact form's `doc_id`, and an anchor
/// without it names no document. Both forms reject that the same way
/// (`validation/provenance-malformed`), and a `doc_id` repeated on one
/// node across either form rejects as
/// `validation/provenance-duplicate`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct SpecProvenance {
    pub doc_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    /// The division of the source this anchor names. Source-neutral —
    /// a subclause, a worksheet, a package — and the key the per-division
    /// coverage counts group by. See [`Position`] for why it is not part
    /// of that type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Where inside that division, if the anchor says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Position>,
}

impl SpecProvenance {
    /// Parse the compact URI form `doc_id[@rev][#section[:position]]`.
    /// Returns `None` if `input` is empty or `doc_id` would be empty.
    pub fn parse_compact(input: &str) -> Option<Self> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        let (doc_and_rev, section_and_page) = match input.split_once('#') {
            Some((head, tail)) => (head, Some(tail)),
            None => (input, None),
        };
        let (doc_id, rev) = match doc_and_rev.split_once('@') {
            Some((doc, rev)) => (doc.trim(), Some(rev.trim().to_string())),
            None => (doc_and_rev.trim(), None),
        };
        if doc_id.is_empty() {
            return None;
        }
        // A trailing `:` segment is a position when it spells one, and
        // part of the section otherwise — a division id may legitimately
        // contain a colon, and reading such an id as a malformed
        // coordinate would lose the division instead of reporting it.
        let (section, at) = match section_and_page {
            None => (None, None),
            Some(tail) => match tail.rsplit_once(':') {
                Some((sec, rest)) => match position_of(rest) {
                    Some(position) => (Some(sec.trim().to_string()), Some(position)),
                    None => (Some(tail.trim().to_string()), None),
                },
                None => (Some(tail.trim().to_string()), None),
            },
        };
        Some(Self {
            doc_id: doc_id.to_string(),
            rev,
            section: section.filter(|s| !s.is_empty()),
            at,
        })
    }
}

/// Opaque requirement identifier. The string is treated as a token —
/// SCE does not assign semantics to its shape or interpret it as a
/// path into any catalogue. Consumers (req-coverage reporters, IDE
/// linters) own the semantic layer.
///
/// Wrapped in a newtype so it survives serde round-trip distinct
/// from a free `String` (e.g. accidental concatenation with a state
/// id fails to compile).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct RequirementId(pub String);

impl RequirementId {
    // ⛔ There is deliberately no `validate` here, and this note is
    // what keeps one from coming back.
    //
    // A `validate` used to sit at this spot. It enforced a shape —
    // first character a letter or underscore, the rest letters,
    // digits, `.`, `-`, `_`, `:` — and it was called by nothing but
    // its own unit tests (measured: 8 call sites, all inside this
    // file's `#[cfg(test)]` module).
    //
    // It was deleted rather than wired up, because the published
    // contract says the opposite of what it enforced.
    // `docs/SCE_ACCEPTED_SUBSET.md` §2.10: *"Tokens are opaque to SCE
    // (no shape enforcement — IR generators own the semantic layer)."*
    // The type doc above says the same. The parser agrees with both:
    // measured, it accepts `3.DoIP-152`, `ns:req.1`, `12345`, `REQ/1`
    // and `REQ#9` alike. So the question this hole was registered
    // under — NMTOKEN or Name — had a third answer, which is that SCE
    // constrains the shape at all only by mistake.
    //
    // ⚠ Wiring it up would have rejected a real standard's own
    // spelling: ISO 13400-2 numbers its requirements `3.DoIP-152`,
    // which begins with a digit, and the committed fixtures under
    // `tests/fixtures/requirement_closure/` are built entirely from
    // ids of that shape. The unit test that went with it asserted
    // `1bad_starts_digit` is invalid, so the policy lived in the
    // assertion as much as in the code — both are gone.
    //
    // What replaces it is a check rather than a promise:
    // `tests/requirement_id_opacity.rs` drives diverse id spellings
    // through the parser and fails if any is refused or altered, at
    // the parse AND at the wire.
    //
    // ⚠ How long it sat there, because the number decides whether
    // this needs a gate: introduced `7f1ec92c66` (2026-05-22),
    // deleted 2026-09-12 — **113 days**, called by nothing but its
    // own tests the whole time.
    //
    // Nothing flagged it, and the reasons are worth stating so the
    // next person does not assume a lint will:
    //
    //   - `dead_code` does not apply to a `pub` item in a library —
    //     rustc cannot know whether a consumer outside the crate
    //     calls it.
    //   - `unreachable_pub` would not have fired either: `provenance`
    //     is a `pub mod`, so the function really was reachable from
    //     outside.
    //   - Had it been private, `cargo build` would not merely have
    //     warned — it FAILS. Measured by adding such a function and
    //     building: *error: associated function … is never used*,
    //     an error rather than a warning because this crate denies
    //     dead code. Its only callers sat behind `#[cfg(test)]` and
    //     are compiled out of a normal build. `pub` is what
    //     suppressed that, and `pub` was not needed — no caller was
    //     ever outside.
    //
    // So the cheap rule that would have caught this is "do not make
    // an item `pub` before something outside the crate calls it".
    // Detecting the general case — a `pub` item no one anywhere
    // calls — needs whole-workspace analysis that neither rustc nor
    // clippy offers, which is why this is recorded rather than
    // gated.

    /// Split a whitespace-separated `sce:req="ID1 ID2 ID3"` value
    /// into individual ids without performing validation. Empty
    /// tokens are dropped (collapsing runs of whitespace).
    pub fn split(raw: &str) -> Vec<&str> {
        raw.split_whitespace().collect()
    }
}

impl std::fmt::Display for RequirementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `<sce:unresolved>` marker — an explicit "this value is a guess,
/// revisit later" placeholder that the parser can detect, the
/// codegen propagates as a comment, and `--strict` builds reject.
///
/// SCE stores the marker; any consumer (linter, IDE, NL→IR
/// pipeline) interprets it. SCE never resolves it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct UnresolvedMarker {
    /// Author-chosen identifier — opaque to SCE, but unique within
    /// the enclosing document is the convention.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Suggested values the author was choosing between. Whitespace
    /// is the separator on the attribute form
    /// (`sce:unresolved-candidates="a b c"`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_doc_only() {
        let p = SpecProvenance::parse_compact("OEM-SPEC-01").unwrap();
        assert_eq!(p.doc_id, "OEM-SPEC-01");
        assert!(p.rev.is_none() && p.section.is_none() && p.at.is_none());
    }

    #[test]
    fn compact_doc_rev_section() {
        let p = SpecProvenance::parse_compact("OEM-SPEC-01@23#4.4.2").unwrap();
        assert_eq!(p.doc_id, "OEM-SPEC-01");
        assert_eq!(p.rev.as_deref(), Some("23"));
        assert_eq!(p.section.as_deref(), Some("4.4.2"));
        assert!(p.at.is_none());
    }

    /// The bare-number spelling, kept so an unmigrated document is read
    /// the way it was written rather than silently re-read as a longer
    /// section id. See [`SpecProvenance`].
    #[test]
    fn compact_with_bare_page() {
        let p = SpecProvenance::parse_compact("OEM-SPEC-01@23#4.4.2:118").unwrap();
        assert_eq!(p.at, Some(Position::Page(118)));
        assert_eq!(p.section.as_deref(), Some("4.4.2"));
    }

    /// Every position shape reaches the compact form, which is the
    /// point of the variant: a source that is not paginated still has a
    /// coordinate a reviewer can open.
    #[test]
    fn compact_carries_every_position_shape() {
        let cases: [(&str, Position, &str); 3] = [
            ("D#4.4.2:page=118", Position::Page(118), "4.4.2"),
            ("D#Sheet1:row=41", Position::Row(41), "Sheet1"),
            (
                "D#Pkg:path=/Elem/x",
                Position::Path("/Elem/x".into()),
                "Pkg",
            ),
        ];
        let mut checked = 0usize;
        for (input, expected, section) in cases {
            let p = SpecProvenance::parse_compact(input)
                .unwrap_or_else(|| panic!("`{input}` must parse"));
            assert_eq!(p.at, Some(expected), "for `{input}`");
            assert_eq!(p.section.as_deref(), Some(section), "for `{input}`");
            checked += 1;
        }
        assert_eq!(checked, 3, "every shape must have a case");
    }

    /// A division id may contain a colon, and a trailing segment that
    /// spells no position leaves it part of the section — losing the
    /// division would be worse than carrying no coordinate.
    #[test]
    fn a_colon_that_spells_no_position_stays_in_the_section() {
        let p = SpecProvenance::parse_compact("D#4.4.2:draft").unwrap();
        assert_eq!(p.section.as_deref(), Some("4.4.2:draft"));
        assert!(p.at.is_none());
    }

    #[test]
    fn compact_rejects_empty_doc() {
        assert!(SpecProvenance::parse_compact("").is_none());
        assert!(SpecProvenance::parse_compact("@23").is_none());
    }

    #[test]
    fn requirement_id_split_collapses_whitespace() {
        assert_eq!(
            RequirementId::split("  A   B\tC\nD "),
            vec!["A", "B", "C", "D"]
        );
        assert_eq!(RequirementId::split(""), Vec::<&str>::new());
    }

    #[test]
    fn json_skips_absent_optionals() {
        let p = SpecProvenance {
            doc_id: "OEM-SPEC-01".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, r#"{"doc_id":"OEM-SPEC-01"}"#);
    }
}
