//! NL→IR Mapping Roadmap Item 8 — the innermost-enclosing-anchor
//! lookup behind `SCE_ERROR_CONTRACT.md` §2.1.2.
//!
//! The contract that clause states:
//!
//! > A diagnostic carries the `spec_provenance` of the **innermost
//! > anchored node enclosing its source location**, whenever one
//! > exists. An empty `spec_provenance` means exactly one thing: no
//! > enclosing node carried an anchor.
//!
//! This module is the half of that sentence a producer can execute:
//! given a source location, which anchors enclose it. It answers that
//! and nothing else — which stages call it, and what they do with the
//! answer, is the wiring that follows.
//!
//! # Why a positional index rather than a walk over the IR
//!
//! The obvious alternative is to walk [`crate::model::SCXMLModel`] and
//! read `provenance` off the state / transition / action / invoke the
//! complaint is about. It was rejected for two reasons, and both are
//! the contract's own words rather than implementation taste.
//!
//! The first is that §2.1.2 is stated in terms of a *source location*,
//! not an IR node. A node-keyed lookup would answer a differently
//! shaped question, and then what the field means would depend on
//! which helper the raising site happened to reach for — the exact
//! thing the clause's second half forbids, since a consumer is
//! promised one meaning without knowing how SCE is structured
//! internally.
//!
//! The second is precision. IR nodes record where they *start*
//! ([`crate::model::State::source_location`] and its four siblings) and
//! nothing records where they end, so an IR-keyed answer would have to
//! reconstruct extents from the starts of the nodes that follow. That
//! reconstruction is wrong in one band, and it is not an exotic band:
//! everything between the last annotated descendant and the parent's
//! own close — the closing tags — reads as belonging to the innermost
//! descendant, which is the one node that is definitely *not*
//! enclosing it. An index built while the XML tree is still in hand
//! has real extents and the band does not exist. A test below pins
//! that case specifically, since it is the one observation that
//! distinguishes this design from the cheaper one.
//!
//! # Which elements are indexed
//!
//! Every element carrying a readable `sce:provenance`, not only the
//! elements the IR happens to have a node for. §2.1.2's rationale is
//! that "annotating a region annotates everything the region governs",
//! and a region is a span of the author's document — the fact that SCE
//! lowers `<state>` to a node and `<param>` to a field of one is an
//! internal matter that must not decide what an author's annotation
//! covers. This is the one place where the index deliberately answers
//! for more than [`crate::requirements_report`] reports: that report
//! enumerates *IR nodes* carrying annotations, which is a different
//! question with a different consumer.
//!
//! # Coordinates
//!
//! Positions here are the ones the parse recorded: rows and columns in
//! the **expanded** document, the same space
//! [`crate::model::State::source_location`] uses. A caller holding
//! authored coordinates — anything that has already been through
//! [`crate::model::AuthoredPositions::resolve`] or
//! `remap_post_expansion` — is in a different space and must resolve
//! before remapping, not after. Documents with no preprocessor
//! directives make the two spaces identical, which is precisely why
//! this has to be said out loud: the mistake is invisible in every
//! test that does not use `<xi:include>` or `<sce:use>`.

use crate::forge::error::SourceLocation;
use crate::provenance::SpecProvenance;

/// A row/column position in the expanded document, ordered the way a
/// reader scans: by row, then by column within the row.
///
/// A tuple rather than a struct because that ordering is exactly
/// `(u32, u32)`'s derived one, and restating it as a hand-written
/// `Ord` would be a second place for it to be wrong.
type Pos = (u32, u32);

/// One anchored element's extent and the anchors it declared.
#[derive(Debug, Clone)]
struct AnchoredRegion {
    /// First byte of `<`, as a position. Inclusive.
    start: Pos,
    /// One past the last byte of the element's closing `>`, as a
    /// position. Exclusive — an element that ends where the next one
    /// begins must not claim the next one's first character.
    end: Pos,
    /// Verbatim, in document order, as the author wrote them. SCE
    /// never infers an anchor (RFC §4.3), so this is a copy and not a
    /// derivation.
    anchors: Vec<SpecProvenance>,
}

/// The anchored regions of one parsed document, queryable by position.
///
/// Built once per parse and carried on the model beside
/// [`crate::model::AuthoredPositions`], for the same reason that one
/// is: it is knowledge only the parse has, and every consumer of it
/// runs after the XML tree is gone.
///
/// [`Default`] is an empty index over a document with no label, which
/// answers "nothing encloses that" to every query. That is the honest
/// answer for a model assembled in memory rather than parsed: no
/// document was read, so no anchor was written down.
#[derive(Debug, Clone, Default)]
pub struct AnchorIndex {
    /// The document these positions index into, in the artifact
    /// spelling (a basename — see `parser::artifact_label`). Held so a
    /// location naming some other document cannot be answered with
    /// this document's rows.
    label: String,
    /// In document order, which is also ascending by `start`. Nothing
    /// depends on the ordering; it falls out of the walk.
    regions: Vec<AnchoredRegion>,
}

impl AnchorIndex {
    /// Index every element of `root`'s document that carries a
    /// readable `sce:provenance`.
    ///
    /// `label` is the document label the parse is threading —
    /// whichever spelling it holds. The index normalises it the way
    /// every other artifact-facing consumer does, so a caller that has
    /// the full path and a caller that has the basename agree.
    ///
    /// # Unreadable anchors
    ///
    /// An element whose `sce:provenance` does not parse is skipped
    /// rather than rejected here. That is not a swallowed error: the
    /// parse that produced the model already rejected every such
    /// element it visited (`validation/provenance-malformed`,
    /// `validation/provenance-duplicate`), so a document that reaches
    /// this function has none among them. What can survive is an
    /// element the IR has no node for and therefore never read — and
    /// for that one, skipping is what the contract demands rather than
    /// a concession: an anchor that could not be read is not an
    /// anchor, and attaching a half-read one would make a record
    /// assert the thing SCE is refusing.
    pub fn build(root: roxmltree::Node<'_, '_>, label: &str) -> Self {
        let doc = root.document();
        let mut regions = Vec::new();
        for node in root.descendants().filter(|n| n.is_element()) {
            let anchors = match crate::parser::collect_sce_provenance(
                &node,
                || node.tag_name().name().to_string(),
                label,
            ) {
                Ok(anchors) if !anchors.is_empty() => anchors,
                // Nothing declared, or nothing readable. Both are
                // "this region is not anchored"; see above for why
                // the second is not an error to raise from here.
                _ => continue,
            };
            let range = node.range();
            let start = doc.text_pos_at(range.start);
            let end = doc.text_pos_at(range.end);
            regions.push(AnchoredRegion {
                start: (start.row, start.col),
                end: (end.row, end.col),
                anchors,
            });
        }
        Self {
            label: crate::parser::artifact_label(label),
            regions,
        }
    }

    /// The anchors of the innermost anchored region enclosing `loc`,
    /// or an empty slice when nothing encloses it.
    ///
    /// Returning the empty slice for "nothing encloses it" is the
    /// contract's single meaning of an absent `spec_provenance`, so
    /// this signature deliberately has no third answer to offer. A
    /// caller cannot tell an unanchored location from an unanswerable
    /// one, and per §2.1.2 must not need to.
    ///
    /// # What "innermost" costs to compute
    ///
    /// Nothing, given real extents. XML elements nest properly, so any
    /// two regions enclosing one position are themselves nested, and
    /// the innermost is simply the one that starts last. No depth is
    /// recorded and none is needed — a depth counter would be a second
    /// encoding of a fact the positions already carry, free to drift
    /// from them.
    ///
    /// The scan is linear over anchored regions. A sorted index with a
    /// binary search would buy nothing measurable on a path that runs
    /// once per rejected document, and would cost an ordering
    /// invariant that a future builder could quietly break.
    pub fn enclosing(&self, loc: &SourceLocation) -> &[SpecProvenance] {
        // A location in another document has rows in another
        // numbering. Answering it with this document's regions would
        // be a confident wrong anchor, which is worse than none —
        // `SpecProvenance` is what a reviewer opens a specification
        // at.
        //
        // The comparison is over the artifact spelling, because one
        // document reaches this with two: a diagnostic names the path
        // a consumer opens, an artifact names the basename (see
        // `parser::artifact_label`). That leaves the caveat
        // [`SourceLocation::file`] already states — two inputs sharing
        // a basename are not distinguished — so the precondition is
        // that the caller holds the model that parsed the document its
        // location names. Which is the only way a caller gets here: an
        // index is reachable through the model it was built with.
        if crate::parser::artifact_label(&loc.file) != self.label {
            return &[];
        }
        // A record with no row names no position, so no region
        // encloses it. This is the "different question" the Item 8
        // roster registers separately: what a diagnostic that carries
        // no location at all should say is not answerable by a
        // positional lookup, and inventing an answer here would put
        // a second meaning behind an empty field.
        let Some(row) = loc.line else {
            return &[];
        };
        // A row without a column is a whole-row coordinate (XSD
        // errors carry one). Reading it as the row's first column
        // makes an element that starts mid-row not enclose it, so the
        // answer walks outward to the region that does — the enclosing
        // one, which is true, rather than the precise one, which is
        // unknown. `AuthoredPositions::call_site_on` collapses to row
        // granularity the same way and for the same reason: under-
        // report rather than mis-report.
        let at: Pos = (row, loc.col.unwrap_or(1));
        self.regions
            .iter()
            .filter(|r| r.start <= at && at < r.end)
            .max_by_key(|r| r.start)
            .map(|r| r.anchors.as_slice())
            .unwrap_or(&[])
    }

    /// Give a rejection the anchors enclosing the location it already
    /// names, unless it brought its own.
    ///
    /// ⚠ Callers go through [`crate::model::SCXMLModel::with_enclosing_anchor`]
    /// rather than here. This method trusts that `err.location` is in
    /// the same coordinate space as the index, and only the model
    /// knows whether that is true — see that method for the check and
    /// why it cannot be left to the call site.
    ///
    /// This is the whole wiring surface. A stage that holds the model
    /// calls it **once, where its errors leave it** — not at each
    /// raise site. The RFC's §2.2 is explicit that widening by adding
    /// call sites is correct behaviour with the wrong structure: every
    /// site is another place to forget, and forgetting is silent. One
    /// call at a boundary cannot be forgotten by a rejection added
    /// inside that boundary next year, which is the property being
    /// bought.
    ///
    /// # Why an already-filled record is left alone
    ///
    /// The four parser sites thread the anchors of the node they are
    /// looking at, and they run where this lookup cannot: mid-parse,
    /// with the model half-built. Those are the exact node's anchors,
    /// which is at least as precise as the innermost enclosing one and
    /// is sometimes strictly more so. Overwriting them with a lookup
    /// would trade a better answer for a uniform one. So the two
    /// regimes compose rather than compete, exactly as RFC §4.1 says:
    /// manual where the model does not exist yet, resolved everywhere
    /// after.
    pub fn enrich<E>(
        &self,
        err: crate::forge::error::Located<E>,
    ) -> crate::forge::error::Located<E> {
        if !err.spec_provenance.is_empty() {
            return err;
        }
        let anchors = self.enclosing(&err.location);
        if anchors.is_empty() {
            return err;
        }
        err.with_spec_provenance(anchors.to_vec())
    }

    /// How many anchored regions the document declared.
    ///
    /// Exists for the guard that has to distinguish "the lookup
    /// answered empty" from "the lookup had nothing to answer from" —
    /// a test asserting the former against an index built from a
    /// document with no anchors at all passes without measuring
    /// anything.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Whether the document declared no anchors at all.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SCXMLModel;

    const DOC: &str = "anchors.scxml";

    fn parse(scxml: &str) -> SCXMLModel {
        crate::parser::SCXMLParser::new()
            .parse_string(scxml, DOC)
            .expect("fixture parses")
    }

    fn at(line: u32, col: u32) -> SourceLocation {
        SourceLocation {
            file: DOC.to_string(),
            line: Some(line),
            col: Some(col),
        }
    }

    fn doc_ids(anchors: &[SpecProvenance]) -> Vec<&str> {
        anchors.iter().map(|a| a.doc_id.as_str()).collect()
    }

    /// Rows are 1-based and the fixtures are written so that the row
    /// of every element is the row this helper names — asserted here
    /// once so the position arithmetic in each test is checked rather
    /// than assumed. A fixture edited without renumbering its
    /// assertions would otherwise keep passing while testing a
    /// different element.
    fn row_of(scxml: &str, needle: &str) -> u32 {
        let idx = scxml.find(needle).expect("fixture contains the element");
        u32::try_from(scxml[..idx].matches('\n').count() + 1).expect("fixture is small")
    }

    /// The anchor is on the enclosing `<state>` and the location is on
    /// a `<transition>` inside it. This is the case §2.1.2 chooses the
    /// word "enclosing" for, and before Item 8 it answered nothing.
    #[test]
    fn an_ancestors_anchor_answers_for_a_child_element() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0">
  <state id="s0" sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>"#;
        let model = parse(scxml);
        let index = &model.anchor_index;
        assert_eq!(index.len(), 1, "the fixture anchors exactly one region");

        let transition_row = row_of(scxml, "<transition");
        assert_eq!(
            doc_ids(index.enclosing(&at(transition_row, 5))),
            vec!["OEM-DIAG-SPEC"],
            "a complaint about the transition is inside a state anchored \
             at 3.4.2, and that is the paragraph a reviewer opens",
        );
    }

    /// Both the state and the transition inside it are anchored. The
    /// answer is the transition's — "innermost", not "outermost" and
    /// not both.
    #[test]
    fn the_innermost_of_two_nested_anchors_wins() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0">
  <state id="s0" sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
    <transition event="go" target="s1"
                sce:provenance="OEM-TIMING-REQ@B#7.1"/>
  </state>
  <final id="s1"/>
</scxml>"#;
        let model = parse(scxml);
        let index = &model.anchor_index;
        assert_eq!(index.len(), 2, "the fixture anchors two nested regions");

        let transition_row = row_of(scxml, "<transition");
        assert_eq!(
            doc_ids(index.enclosing(&at(transition_row, 5))),
            vec!["OEM-TIMING-REQ"],
            "the transition declares its own anchor, so the state's is \
             the enclosing one and not the innermost one",
        );
        let state_row = row_of(scxml, "<state id=\"s0\"");
        assert_eq!(
            doc_ids(index.enclosing(&at(state_row, 3))),
            vec!["OEM-DIAG-SPEC"],
            "the state's own row is outside the transition's extent",
        );
    }

    /// The same regions with every extent reconstructed the way a
    /// lookup keyed on the IR would have to reconstruct it: a node's
    /// extent runs to the start of the next recorded node, because
    /// where a node *ends* is the one thing the IR does not record.
    ///
    /// `after` is the start of the first element following the last
    /// anchored region — a real position read out of the fixture
    /// rather than an open end. That choice is the whole reason this
    /// helper is trustworthy: an open end would make the
    /// reconstruction cover *more*, which is the direction that lets
    /// the comparison below pass without measuring anything. The
    /// tightest faithful reconstruction is the one that has to be
    /// beaten.
    fn as_if_reconstructed_from_following_starts(index: &AnchorIndex, after: Pos) -> AnchorIndex {
        let mut regions = index.regions.clone();
        for i in 0..regions.len() {
            regions[i].end = regions.get(i + 1).map_or(after, |next| next.start);
        }
        AnchorIndex {
            label: index.label.clone(),
            regions,
        }
    }

    /// The band that separates a real extent from one reconstructed
    /// out of the starts that follow: the parent's closing tag.
    ///
    /// It sits after the last anchored descendant and before whatever
    /// comes next, so a reconstruction attributes it to that
    /// descendant — the one element that has already ended. With the
    /// element's true range the row belongs to the state alone. This
    /// is why the index is built while the XML tree is in hand.
    ///
    /// Both halves are asserted, and the second is the one that keeps
    /// the first honest. The first states the contract. The second
    /// states that this fixture can still tell the two designs apart,
    /// which is a precondition of the first meaning anything: flatten
    /// the document so the band disappears and the contract assertion
    /// keeps passing over a document that cannot fail it. Measured
    /// 2026-09-11 by building the index with reconstructed extents —
    /// this test answered `OEM-TIMING-REQ`, the child that had already
    /// ended.
    #[test]
    fn a_closing_tag_row_belongs_to_the_parent_not_its_last_child() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0">
  <state id="s0" sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
    <transition event="go" target="s1"
                sce:provenance="OEM-TIMING-REQ@B#7.1"/>
  </state>
  <final id="s1"/>
</scxml>"#;
        let model = parse(scxml);
        let closing_row = row_of(scxml, "</state>");
        assert_eq!(
            doc_ids(model.anchor_index.enclosing(&at(closing_row, 3))),
            vec!["OEM-DIAG-SPEC"],
            "the transition ended on the row above, so the state is the \
             innermost region still enclosing its own closing tag",
        );

        let next_element = (row_of(scxml, "<final"), 3);
        let reconstructed =
            as_if_reconstructed_from_following_starts(&model.anchor_index, next_element);
        assert_eq!(
            doc_ids(reconstructed.enclosing(&at(closing_row, 3))),
            vec!["OEM-TIMING-REQ"],
            "this fixture no longer distinguishes real extents from \
             reconstructed ones, so the assertion above is measuring \
             nothing. Restore a row that lies after the last anchored \
             descendant ends and before its parent does.",
        );
    }

    /// An anchor on the `<scxml>` root encloses every location in the
    /// document, including one in a sibling subtree that declares
    /// nothing. Document-level annotation is the coarsest case of the
    /// same containment rule, not a separate feature.
    #[test]
    fn a_root_anchor_encloses_an_unannotated_subtree() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0"
       sce:provenance="OEM-DIAG-SPEC@D#1">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>"#;
        let model = parse(scxml);
        let transition_row = row_of(scxml, "<transition");
        assert_eq!(
            doc_ids(model.anchor_index.enclosing(&at(transition_row, 5))),
            vec!["OEM-DIAG-SPEC"],
        );
    }

    /// A location no anchored region encloses answers empty — from an
    /// index that is not itself empty, so the assertion measures the
    /// lookup rather than the absence of anything to look up.
    #[test]
    fn a_location_outside_every_anchored_region_answers_empty() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0">
  <state id="s0" sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
    <transition event="go" target="s1"/>
  </state>
  <state id="s1">
    <transition event="back" target="s0"/>
  </state>
</scxml>"#;
        let model = parse(scxml);
        let index = &model.anchor_index;
        assert!(
            !index.is_empty(),
            "the document anchors s0, so an empty answer below is the \
             lookup's verdict and not an empty index",
        );
        let sibling_row = row_of(scxml, "<transition event=\"back\"");
        assert!(
            index.enclosing(&at(sibling_row, 5)).is_empty(),
            "s1 is a sibling of the anchored state, not inside it",
        );
    }

    /// An anchor on an element the IR has no node of its own for. The
    /// author annotated a region; what SCE lowers that region to is
    /// not the author's concern.
    #[test]
    fn an_anchor_on_a_non_ir_element_still_encloses_its_children() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0"
       datamodel="ecmascript">
  <datamodel sce:provenance="OEM-DIAG-SPEC@D#5.1">
    <data id="threshold" expr="12"/>
  </datamodel>
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>"#;
        let model = parse(scxml);
        let data_row = row_of(scxml, "<data id=");
        assert_eq!(
            doc_ids(model.anchor_index.enclosing(&at(data_row, 5))),
            vec!["OEM-DIAG-SPEC"],
            "`<datamodel>` is not an IR node, and the annotation on it \
             still governs the rows it spans",
        );
    }

    /// A row with no column is answered at the row's first column, so
    /// an element starting mid-row does not claim it. The enclosing
    /// region answers instead: less precise, still true.
    #[test]
    fn a_line_only_location_answers_with_the_enclosing_region() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0">
  <state id="s0" sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
    <transition event="go" target="s1" sce:provenance="OEM-TIMING-REQ@B#7.1"/>
  </state>
  <final id="s1"/>
</scxml>"#;
        let model = parse(scxml);
        let transition_row = row_of(scxml, "<transition");
        let line_only = SourceLocation {
            file: DOC.to_string(),
            line: Some(transition_row),
            col: None,
        };
        assert_eq!(
            doc_ids(model.anchor_index.enclosing(&line_only)),
            vec!["OEM-DIAG-SPEC"],
            "column 1 of that row is inside the state and before the \
             transition begins, so the state is the honest answer",
        );
    }

    /// A record naming a different document is not answered from this
    /// document's rows. The two spellings of one document's label —
    /// the path a consumer opens and the basename an artifact carries
    /// — are still one document.
    #[test]
    fn a_location_in_another_document_answers_empty() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" initial="s0">
  <state id="s0" sce:provenance="OEM-DIAG-SPEC@D#3.4.2:112">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>"#;
        let model = parse(scxml);
        let index = &model.anchor_index;
        let transition_row = row_of(scxml, "<transition");

        assert!(
            index
                .enclosing(&SourceLocation {
                    file: "some/other.scxml".to_string(),
                    line: Some(transition_row),
                    col: Some(5),
                })
                .is_empty(),
            "another document's row {transition_row} is not this \
             document's row {transition_row}",
        );
        assert_eq!(
            doc_ids(index.enclosing(&SourceLocation {
                file: format!("sub/dir/{DOC}"),
                line: Some(transition_row),
                col: Some(5),
            })),
            vec!["OEM-DIAG-SPEC"],
            "a path and its basename name the same document",
        );
    }

    /// A model nobody parsed carries an index that answers nothing,
    /// rather than one that cannot be asked.
    #[test]
    fn an_unparsed_model_answers_empty_without_panicking() {
        let model = SCXMLModel::default();
        assert!(model.anchor_index.is_empty());
        assert!(model.anchor_index.enclosing(&at(1, 1)).is_empty());
    }
}
