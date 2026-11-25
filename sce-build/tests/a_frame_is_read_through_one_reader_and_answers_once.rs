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
    // the arms it records the refusal and is the arm the verdict discards. The
    // view's body is its `read_with`, which `read` hands no hook (docs/adr/0136).
    let code = rust_of("codec_variant_peek_basic");
    let recorded = ["read", "read_into", "read_with"]
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
/// refusal of its own: the list's walk names no `Result`. It reads with the hook
/// it is handed (docs/adr/0136), which the scan of a chain hands it.
#[test]
fn a_walk_reads_its_entries_with_the_total_read() {
    let code = rust_of("codec_tlv_chain_basic");
    assert!(
        code.contains(
            "fn read_with<'b, H: ::sce_forge_runtime::codec::ChainHook>(_r: &mut SceReader<'b>, "
        ),
        "the walk does not read through the reader:\n{code}"
    );
    assert!(
        !code.contains("-> Result<Self::Entry<'b>, CodecError>"),
        "the walk answers a refusal per entry:\n{code}"
    );
}

/// Codecs whose views scan a repeat: one to a count and one to the end of its
/// frame.
const SCANNED: [&str; 2] = ["codec_repeat_basic", "codec_until_eof_basic"];

/// Codecs whose views scan a chain: a chain, the chain of a frame's extensions
/// closed by an entry flag, and a chain whose entries gate the fields that
/// follow it.
const CHAINS_SCANNED: [&str; 3] = [
    "codec_tlv_chain_basic",
    "codec_zenoh_ext_envelope",
    "codec_chain_has_tagged",
];

/// A view's scan counts an entry only where it read it whole. An entry whose
/// read recorded a refusal is the zeros a total read returns, and the walk of
/// the list stops at it, so a count that held it would be one the walk never
/// reaches: the gates a decode derives from the list's entries after the
/// refusal would walk a list that disagrees with its count (docs/adr/0061 §3).
/// A repeat's scan counts by that rule in its own lines; a chain's is the
/// runtime's `walk_chain` (docs/adr/0136), which counts by the same rule
/// (`chain::tests::a_scan_counts_an_entry_only_where_its_read_was_whole`), and
/// prints no count of its own.
#[test]
fn a_scan_counts_the_entries_it_read_whole() {
    for name in SCANNED.into_iter().chain(CHAINS_SCANNED) {
        let code = rust_of(name);
        assert!(
            code.contains("ListView::new(_rest, _count, "),
            "{name}'s view scans no list:\n{code}"
        );
        let counts: Vec<&str> = code
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("_count +="))
            .collect();
        if CHAINS_SCANNED.contains(&name) {
            assert!(
                counts.is_empty()
                    && code.contains("let _count = ::sce_forge_runtime::codec::walk_chain::<"),
                "{name}'s chain is not scanned by the runtime's count:\n{code}"
            );
            continue;
        }
        assert!(!counts.is_empty(), "{name}'s scan counts no entry:\n{code}");
        for count in counts {
            assert_eq!(
                count, "_count += u32::from(!cursor.failed());",
                "{name}'s scan counts an entry whose read was refused:\n{code}"
            );
        }
    }
}

/// A body a length bounds is read from those bytes by a function of the
/// reader. A view's body that takes nothing else is that function, its own
/// `read`, named: a closure that only passes the reader on is one Clippy
/// refuses (`redundant_closure`), and every other closure adds something, the
/// destination it fills, the `Some` of a gated body or the flags it hands on.
#[test]
fn a_body_a_length_bounds_is_read_by_its_own_read() {
    let code = rust_of("codec_zenoh_decl_ext_keyexpr");
    assert!(
        code.contains(
            "cursor.within(total_length as usize, CodecZenohDeclExtKeyexprInnerView::read);"
        ),
        "the view does not read its bounded body by the body's own read:\n{code}"
    );
    for line in code.lines().filter(|line| line.contains("|_inner| ")) {
        let (_, body) = line
            .split_once("|_inner| ")
            .expect("the line holds the closure");
        let forwards = body
            .strip_suffix(");")
            .and_then(|call| call.strip_suffix("(_inner)"))
            .is_some_and(|path| !path.contains('('));
        assert!(
            !forwards,
            "a closure only passes its reader on: {}",
            line.trim()
        );
    }
}

/// Codecs whose decode reads a length a sibling carries: one carried as itself
/// plus one (`sce:length-arith="-1"`) and read streaming, one carried as
/// itself minus one (`"+1"`) and sliced at a fixed offset, one whose `+1` is
/// applied to the flag of a carrier, and a 32-bit length sliced at a fixed
/// offset.
const LENGTHS: [&str; 4] = [
    "codec_declared_bounds",
    "codec_scout_zid_body",
    "codec_zenoh_hello",
    "codec_length_ref_uint32_le",
];

/// A length the check refuses is still read on through the reader
/// (docs/adr/0128), so what the decode makes of it is total over every value
/// the carrier holds: the offset is applied checked, and a slice at a fixed
/// offset is the frame past the offset, then that many bytes of it, with no sum
/// of the two. Arithmetic that a refused length can carry past its type would
/// panic in a build that checks it, on a frame that is only to be refused.
#[test]
fn a_length_is_offset_and_sliced_with_no_arithmetic_that_can_overflow() {
    for name in LENGTHS {
        let code = rust_of(name);
        let reads: Vec<&str> = ["read", "read_into"]
            .into_iter()
            .flat_map(|fun| bodies(&code, fun))
            .collect();
        assert!(!reads.is_empty(), "{name} has no read");
        for body in reads {
            let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
            for unchecked in [
                "(_len as usize) - ",
                "(_len as usize) + ",
                "try_from(_len - ",
                "as usize - ",
                "as usize + ",
            ] {
                assert!(
                    !flat.contains(unchecked),
                    "{name} offsets a length unchecked (`{unchecked}`):\n{body}"
                );
            }
            for (at, _) in flat.match_indices("raw.get(") {
                let range = &flat[at + "raw.get(".len()..];
                let start = range.trim_start_matches(|c: char| c.is_ascii_digit());
                assert!(
                    start.starts_with("..)"),
                    "{name} slices by a sum of an offset and a length:\n{body}"
                );
            }
        }
    }
    let bounds = rust_of("codec_declared_bounds");
    assert!(
        bounds.contains("checked_sub(1).ok_or(CodecError::LengthOutOfRange)"),
        "the tag's length is not taken one below what it carries, checked:\n{bounds}"
    );
    let zid = rust_of("codec_scout_zid_body");
    assert!(
        zid.contains("raw.get(1..).zip((zid_len_m1 as usize).checked_add(1))"),
        "the zid is not sliced past its offset by a length one above what it carries:\n{zid}"
    );
}
