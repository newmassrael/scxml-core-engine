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
///   (`doc_id @ rev # section`; trailing `:page` optional after the
///   section to carry a page number, e.g. `OEM-SPEC-01#4.4.2:118`)
/// - child element: `<sce:provenance doc-id="..." rev="..." section="..." page="..."/>`
///   (one or more allowed; element form lets one node anchor at
///   multiple documents)
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

impl SpecProvenance {
    /// Parse the compact URI form `doc_id[@rev][#section[:page]]`.
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
        let (section, page) = match section_and_page {
            None => (None, None),
            Some(tail) => match tail.rsplit_once(':') {
                Some((sec, page_str)) => match page_str.trim().parse::<u32>() {
                    Ok(p) => (Some(sec.trim().to_string()), Some(p)),
                    Err(_) => (Some(tail.trim().to_string()), None),
                },
                None => (Some(tail.trim().to_string()), None),
            },
        };
        Some(Self {
            doc_id: doc_id.to_string(),
            rev,
            section: section.filter(|s| !s.is_empty()),
            page,
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
    // through the parser and fails if any is refused or altered.

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
        assert!(p.rev.is_none() && p.section.is_none() && p.page.is_none());
    }

    #[test]
    fn compact_doc_rev_section() {
        let p = SpecProvenance::parse_compact("OEM-SPEC-01@23#4.4.2").unwrap();
        assert_eq!(p.doc_id, "OEM-SPEC-01");
        assert_eq!(p.rev.as_deref(), Some("23"));
        assert_eq!(p.section.as_deref(), Some("4.4.2"));
        assert!(p.page.is_none());
    }

    #[test]
    fn compact_with_page() {
        let p = SpecProvenance::parse_compact("OEM-SPEC-01@23#4.4.2:118").unwrap();
        assert_eq!(p.page, Some(118));
        assert_eq!(p.section.as_deref(), Some("4.4.2"));
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
