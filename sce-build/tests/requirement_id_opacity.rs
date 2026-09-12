//! HOLE-2 — `sce:req` tokens are opaque, and this is what says so.
//!
//! `docs/SCE_ACCEPTED_SUBSET.md` §2.10 publishes the contract:
//!
//! > Tokens are opaque to SCE (no shape enforcement — IR generators
//! > own the semantic layer).
//!
//! Until this file existed that sentence was a promise with nothing
//! behind it, and the tree disagreed with itself: `provenance.rs`
//! carried a `RequirementId::validate` enforcing a shape — letter or
//! underscore first, then letters, digits, `.`, `-`, `_`, `:` — that
//! **no production code called**. Two statements of one rule, one of
//! them published and one of them wrong, with nothing to make them
//! meet.
//!
//! # Why the validator was deleted rather than called
//!
//! The hole was registered as a question — XML `NMTOKEN` or XML
//! `Name`? — because the function's summary claimed the first while
//! its code implemented the second (an `NMTOKEN` may begin with a
//! digit; a `Name` may not). Measured against the tree, the question
//! has a third answer: **neither**, because the published contract
//! enforces no shape at all and the parser already behaves that way.
//!
//! ⚠ The cost of getting this wrong was not hypothetical. ISO 13400-2
//! numbers its requirements `3.DoIP-152`, which begins with a digit,
//! so calling the validator as written would have refused a real
//! standard's own spelling — and the committed fixtures under
//! `tests/fixtures/requirement_closure/` are built entirely from ids
//! of that shape, which is why wiring it up would have turned this
//! repository's own ATOMIC-D artefacts red on the first run.
//!
//! # What this file guards
//!
//! Not "does the parser run". It drives deliberately hostile-looking
//! but legal id spellings through the real parse path and requires
//! every one to arrive **verbatim**. If anybody re-adds shape
//! enforcement — or normalisation — this goes red and names §2.10.

use std::path::PathBuf;
use std::process::Command;

use sce_build::parser::SCXMLParser;

/// Id spellings that a shape rule would plausibly reject, each with
/// the reason it is here. None is invented for the test: every shape
/// is one a real requirements catalogue uses.
const SHAPES: &[(&str, &str)] = &[
    ("3.DoIP-152", "ISO 13400-2's own spelling: leading digit"),
    ("REQ_AB_12345", "underscores, the shape the old rule liked"),
    ("REQ-CD-67890", "hyphens"),
    (
        "ns:req.1",
        "a colon and a dot, as a namespaced catalogue writes",
    ),
    ("_underscore_start", "leading underscore"),
    ("12345", "digits only — a bare catalogue number"),
    ("a", "one character"),
    ("REQ/1", "a slash, which the deleted rule refused outright"),
    ("REQ#9", "a hash, likewise"),
    ("SWRS_ADAS.DRV-2A_0147", "a mixed real-world OEM shape"),
];

/// Every shape reaches the IR, unaltered.
///
/// ⚠ The floor is the point. Without it a document that parsed to
/// zero annotations would pass this file silently, which is the exact
/// shape of green this repository keeps being caught by — and the
/// assertion on the joined value is what catches normalisation, which
/// a per-id `contains` check would not.
#[test]
fn every_id_shape_survives_the_parse_verbatim() {
    let ids: Vec<&str> = SHAPES.iter().map(|(id, _)| *id).collect();
    let doc = format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                  xmlns:sce="http://sce.dev/ext"
                  version="1.0" name="opacity" initial="s0">
             <state id="s0" sce:req="{}">
               <transition event="go" target="s1"/>
             </state>
             <state id="s1"/>
           </scxml>"#,
        ids.join(" "),
    );

    let model = SCXMLParser::new()
        .parse_string(&doc, "opacity")
        .unwrap_or_else(|e| {
            panic!(
                "SCE refused an `sce:req` id shape. §2.10 of \
                 docs/SCE_ACCEPTED_SUBSET.md publishes these tokens as \
                 OPAQUE — if shape enforcement is genuinely wanted, the \
                 contract has to change first, and note that it would \
                 reject ISO 13400-2's own `3.DoIP-152`: {:?}",
                e.error,
            )
        });

    let state = model
        .states
        .get("s0")
        .expect("the annotated state is in the model");
    let seen: Vec<&str> = state.req.iter().map(|id| id.0.as_str()).collect();

    for (id, why) in SHAPES {
        assert!(
            seen.contains(id),
            "`{id}` ({why}) did not reach the IR. Either it was refused \
             or it was rewritten; §2.10 allows neither. Saw: {seen:?}",
        );
    }
    assert_eq!(
        seen, ids,
        "the ids reached the IR but not verbatim and in order. An id is \
         compared against a manifest as written, so normalising two \
         spellings together silently merges two requirements",
    );

    println!(
        "HOLE-2: {} id shape(s) survived the parse verbatim",
        seen.len()
    );
    assert!(
        seen.len() >= 10,
        "checked {} shape(s); below ten this stops covering the \
         characters a shape rule would reject first — a leading digit, \
         a slash, a hash, a bare number",
        seen.len(),
    );
}

/// The same claim at the wire, which is what a consumer reads.
///
/// ⚠ The library check above proves the ids reach the IR. It does not
/// prove they reach the REPORT — `sce-codegen requirements` is a
/// separate layer, and a sort, a dedupe or a normalisation added there
/// would leave the model intact while changing every consumer's input.
/// This repository has twice had a check that was green at the library
/// and blind at the wire, so the wire gets its own assertion rather
/// than an argument that it must follow.
#[test]
fn the_command_reports_every_id_shape_verbatim() {
    let ids: Vec<&str> = SHAPES.iter().map(|(id, _)| *id).collect();
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = dir.path().join("opacity.scxml");
    std::fs::write(
        &doc,
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                      xmlns:sce="http://sce.dev/ext"
                      version="1.0" name="opacity" initial="s0">
                 <state id="s0" sce:req="{}"/>
               </scxml>"#,
            ids.join(" "),
        ),
    )
    .expect("write fixture");

    let output = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .arg("requirements")
        .arg(&doc)
        .output()
        .expect("sce-codegen runs");
    assert!(
        output.status.success(),
        "sce-codegen requirements failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");

    // Exact array, in order: anything that reorders, dedupes or
    // rewrites a token fails here, and none of those would be caught
    // by asking whether each id appears somewhere.
    let expected = format!(
        "\"requirement_ids\":[{}]",
        ids.iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(","),
    );
    assert!(
        stdout.contains(&expected),
        "the report did not carry the ids verbatim and in order.\n\
         expected substring: {expected}\nstdout: {stdout}",
    );

    println!(
        "HOLE-2: {} id shape(s) survived to the report verbatim",
        ids.len()
    );
    assert!(
        ids.len() >= 10,
        "checked {} shape(s) at the wire; the floor is the same one the \
         parse-level check carries, for the same reason",
        ids.len(),
    );
}

/// The one rule §2.10 *does* state, kept so deleting the shape rule is
/// not read as deleting all of them.
///
/// Duplicates are refused — not because of an id's shape, but because
/// a repeated id on one node masks a missing annotation downstream.
#[test]
fn a_duplicate_id_on_one_node_is_still_refused() {
    let doc = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" name="dup" initial="s0">
                   <state id="s0" sce:req="3.DoIP-152 3.DoIP-152"/>
                 </scxml>"#;
    let parsed = SCXMLParser::new().parse_string(doc, "dup");
    let err = match parsed {
        Err(e) => e,
        Ok(_) => panic!("a duplicate requirement id on one node must be refused"),
    };
    let rendered = format!("{:?}", err.error);
    assert!(
        rendered.contains("DuplicateRequirementId") || rendered.contains("duplicate"),
        "the refusal must be the duplicate rule, not a shape rule; got: {rendered}",
    );
    println!("HOLE-2: the duplicate rule still fires, on an ISO-shaped id");
}
