// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Rust codec reads a frame through one reader and answers once
//! (docs/adr/0128).
//!
//! Every read of a generated decode is total and records a refusal in the
//! frame's reader (`SceReader`), so no statement of a decode body propagates an
//! error or returns one: the one verdict is the reader's, said by
//! `decode_into` once the frame is read (`SceCursor::read_frame`). A nested
//! body is read through the same reader, so a frame has one verdict however
//! deep it nests.
//!
//! A frame is read into place without a whole default written first where the
//! form allows it: a view is a value of a few words built once from what its
//! reads return, so no arm of a view is made holding a default body; the eager
//! form installs an arm, or makes an optional list present, only where its
//! destination does not hold it already.
//!
//! What the decode does, frame by frame, is held by the round trips the
//! conformance crate runs over the committed goldens.

use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// Codecs of each shape a decode body takes: a fixed frame, a variant with a
/// default arm, a list, a gated chain, a gated list of a variant's envelope.
const CODECS: [&str; 6] = [
    "codec_simple_frame",
    "codec_zenoh_ext_entry",
    "codec_tlv_chain_basic",
    "codec_chain_depth_gated",
    "codec_zenoh_push",
    "codec_variant_peek_basic",
];

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The Rust the codec `name` generates, with its views, every file joined.
fn rust_of(name: &str) -> String {
    let path = resource_dir().join(format!("{name}.scxml"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    sce_build::compile_forge_with_imports(
        &source,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    )
    .unwrap_or_else(|e| panic!("{name} generates for Rust: {e}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// The bodies of every method `name` the code declares, from its signature to
/// its closing brace.
fn bodies<'c>(code: &'c str, name: &str) -> Vec<&'c str> {
    let head = format!("    pub fn {name}");
    code.match_indices(&head)
        .filter(|(at, _)| {
            // `read` is not `read_into`.
            let after = &code[at + head.len()..];
            after.starts_with('(') || after.starts_with('<')
        })
        .map(|(at, _)| {
            let body = &code[at..];
            let end = body
                .find("\n    }\n")
                .unwrap_or_else(|| panic!("`{name}` closes"));
            &body[..end]
        })
        .collect()
}

/// The one verdict: `decode_into` reads the frame through a reader and answers
/// what the reader says, and no body it reads through answers on its own.
#[test]
fn a_decode_answers_its_frames_verdict_once() {
    for name in CODECS {
        let code = rust_of(name);
        let wrappers = bodies(&code, "decode_into");
        assert!(!wrappers.is_empty(), "{name} has no decode_into");
        for wrapper in wrappers {
            let statements: Vec<&str> = wrapper
                .lines()
                .skip(1)
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            assert_eq!(
                statements.len(),
                1,
                "{name}'s decode_into is more than the frame's read:\n{wrapper}"
            );
            assert!(
                statements[0].starts_with("cursor.read_frame(|cursor| "),
                "{name}'s decode_into does not read through one reader:\n{wrapper}"
            );
        }
        let reads: Vec<&str> = ["read", "read_into"]
            .into_iter()
            .flat_map(|fun| bodies(&code, fun))
            .collect();
        assert!(!reads.is_empty(), "{name} has no read");
        for body in reads {
            for answer in [")?", "]?"] {
                assert!(
                    !body.contains(answer),
                    "{name} answers a verdict inside its read (`{answer}`):\n{body}"
                );
            }
            // A list's fill answers its capacity to the list it fills, which the
            // reader then holds; no other refusal is returned.
            for (at, _) in body.match_indices("return Err(") {
                assert!(
                    body[at..].starts_with("return Err(CodecError::CapacityExceeded)"),
                    "{name} returns a refusal the reader should hold:\n{body}"
                );
            }
        }
    }
}

/// No arm of a view is made holding a default body: each arm is its body's view
/// as the read returns it. No optional list or body is made present with a
/// whole default written over what the destination holds, and an eager arm is
/// installed holding a default only where the destination holds another arm.
#[test]
fn a_frame_is_read_into_place_without_a_default_written_first() {
    for name in CODECS {
        let code = rust_of(name);
        let reads: Vec<&str> = ["read", "read_into"]
            .into_iter()
            .flat_map(|fun| bodies(&code, fun))
            .collect();
        for body in reads {
            assert!(
                !body.contains(".insert("),
                "{name} makes a field present by writing a whole value over it:\n{body}"
            );
            let lines: Vec<&str> = body.lines().map(str::trim).collect();
            for (at, line) in lines.iter().enumerate() {
                let installs_default = line.contains("::default()")
                    && (line.starts_with("_out.body = ") || line.contains("ViewVariant::"));
                if !installs_default {
                    continue;
                }
                assert!(
                    !line.contains("ViewVariant::") || line.ends_with("ViewVariant::default()"),
                    "{name}'s view makes an arm of a default body:\n{body}"
                );
                if line.starts_with("_out.body = ") {
                    let before = at.checked_sub(1).map(|b| lines[b]).unwrap_or_default();
                    assert!(
                        before.starts_with("if !matches!(_out.body, ") || before == "} else {",
                        "{name} installs an arm whatever the destination holds:\n{body}"
                    );
                }
            }
        }
        // A view, and an eager value of a fixed shape, is built as a value: its
        // read fills no destination field by field. The eager form that builds
        // its lists in place reads into `_out` (`read_into`), and its `read` is
        // that read into a value of its own.
        for body in bodies(&code, "read") {
            if body.contains("Self::read_into(") {
                continue;
            }
            assert!(
                !body.contains("_out."),
                "{name} fills a destination it should build once:\n{body}"
            );
        }
    }
}

/// Codecs whose decode has a branch no frame takes, a contradiction of its own
/// gates, which are read off the same frame, and what each records there: a
/// chain whose memberships are gates, read in place and read again; a gated
/// list or body read where the arm its gates installed holds it; two lists of
/// one count paired by value; a variant whose arms cover its tag's domain.
const CONTRADICTIONS: [(&str, &[&str]); 7] = [
    (
        "codec_chain_has_envelope",
        &["cursor.fail(CodecError::PresentIfMismatch)"],
    ),
    (
        "codec_chain_has_marker",
        &["cursor.fail(CodecError::PresentIfMismatch)"],
    ),
    (
        "codec_chain_has_tagged",
        &["cursor.fail(CodecError::PresentIfMismatch); break;"],
    ),
    (
        "codec_repeat_unconditional_count",
        &["cursor.fail(CodecError::PresentIfMismatch);"],
    ),
    (
        "codec_zenoh_value_zbuf",
        &["cursor.fail(CodecError::PresentIfMismatch);"],
    ),
    (
        "codec_repeat_flag_aligned_group",
        &[
            "cursor.settle(",
            ".and_then(|_s| ::sce_forge_runtime::codec::Aligned::new(links, _s)",
        ],
    ),
    (
        "codec_variant_peek_basic",
        &["_ => cursor.fail(CodecError::UndeclaredEnumValue),"],
    ),
];

/// A branch of a decode its own gates leave no frame to take records the
/// contradiction in the frame's reader, as every other refusal is recorded
/// (docs/adr/0128): no read propagates it with `?` or returns it, and none
/// panics on it, so a bare-metal image carries no panic path for a frame that
/// cannot arrive. Each codec's reads hold what it records there, read with
/// their whitespace as one space.
#[test]
fn a_contradiction_of_the_gates_is_recorded_in_the_reader() {
    for (name, records) in CONTRADICTIONS {
        let code = rust_of(name);
        let reads: Vec<&str> = ["read", "read_into"]
            .into_iter()
            .flat_map(|fun| bodies(&code, fun))
            .collect();
        assert!(!reads.is_empty(), "{name} has no read");
        for body in &reads {
            for answer in [")?", "]?"] {
                assert!(
                    !body.contains(answer),
                    "{name} answers a verdict inside its read (`{answer}`):\n{body}"
                );
            }
            for (at, _) in body.match_indices("return Err(") {
                assert!(
                    body[at..].starts_with("return Err(CodecError::CapacityExceeded)"),
                    "{name} returns a refusal the reader should hold:\n{body}"
                );
            }
            for panic in ["unreachable!(", "panic!(", ".expect(", ".unwrap()"] {
                assert!(
                    !body.contains(panic),
                    "{name} panics on a frame its gates contradict (`{panic}`):\n{body}"
                );
            }
        }
        // Read with its whitespace as one space, so a record is found however
        // the statement around it is laid out.
        let flat: Vec<String> = reads
            .iter()
            .map(|body| body.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        for record in records {
            assert!(
                flat.iter().any(|body| body.contains(record)),
                "{name} does not record its contradiction as `{record}`:\n{}",
                reads.join("\n")
            );
        }
    }
    // A view of a variant reads its arm as a value, so where the tag is outside
    // the arms it records the refusal and is the arm the verdict discards.
    let code = rust_of("codec_variant_peek_basic");
    let recorded = ["read", "read_into"]
        .into_iter()
        .flat_map(|fun| bodies(&code, fun))
        .filter(|body| body.contains("cursor.fail(CodecError::UndeclaredEnumValue)"))
        .count();
    assert!(
        recorded >= 2,
        "codec_variant_peek_basic records an undeclared tag in fewer than its eager \
         and its view read:\n{code}"
    );
}

/// A walk reads each entry with the view's total read, inlined, and tests no
/// refusal of its own: the list's walk names no `Result`.
#[test]
fn a_walk_reads_its_entries_with_the_total_read() {
    let code = rust_of("codec_tlv_chain_basic");
    assert!(
        code.contains("fn read<'b>(_r: &mut SceReader<'b>, "),
        "the walk does not read through the reader:\n{code}"
    );
    assert!(
        !code.contains("-> Result<Self::Entry<'b>, CodecError>"),
        "the walk answers a refusal per entry:\n{code}"
    );
}
