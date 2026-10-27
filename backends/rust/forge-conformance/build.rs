// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// build.rs for the cross-language numerical conformance harness (Rust half).
//
// Code generation runs in-process via sce_build, into OUT_DIR, in the steps
// numbered below:
//
//   - every fixture listed in tests/forge/conformance/fixtures.json, compiled
//     by compile_forge_with_imports() (Steps 1-2);
//
//   - the per-fixture test harness, rendered from
//     tools/codegen/templates/forge/rust/conformance/harness.rs.jinja2 into
//     the file `conformance::harness_filename(Language::Rust)` names (Step 3);
//
//   - the fixtures the round-trip tests include that the catalog does not
//     list (Steps 4-5).
//
// Each integration test under tests/ `include!`s what it needs from OUT_DIR,
// so no committed Rust test code exists for the numerical harness — the
// single source of truth is the fixture catalog and the conformance template.
//
// This script belongs to this crate and not to `sce-forge-runtime`, whose
// tests it once served. A build script runs for every consumer of its
// package, so there it compiled the whole generator and regenerated this
// catalog for every crate that depended on the runtime — each throwaway
// crate `forge_conformance` compiles a generated codec into, every
// consumer's build — none of which read a byte of it. Here it runs only
// when these tests are built.

use sce_build::{
    compile_forge_with_imports,
    conformance::{self, Manifest},
    generator::Language,
    load_forge_source, DocumentLabel, ForgeCompileOptions,
};
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set by cargo");
    let resource_dir = Path::new(&manifest_dir)
        .join("../../../tests/forge/resources")
        .canonicalize()
        .expect("canonicalize tests/forge/resources");
    let manifest_path = Path::new(&manifest_dir)
        .join("../../../tests/forge/conformance/fixtures.json")
        .canonicalize()
        .expect("canonicalize fixtures.json");
    let template_base = Path::new(&manifest_dir)
        .join("../../../tools/codegen/templates")
        .canonicalize()
        .expect("canonicalize tools/codegen/templates");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", resource_dir.display());
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        template_base.join("forge/rust").display()
    );

    // Step 1: fixture catalog — single source of truth for which SCXML files
    // participate in the conformance harness across all 5 languages.
    let manifest =
        Manifest::load(&manifest_path).unwrap_or_else(|e| panic!("load conformance manifest: {e}"));

    // Step 2: generate each fixture's Rust code into OUT_DIR.
    //
    // With `views`: a view is additive (ADR 0049), so everything a fixture
    // generated without it is still here, and compiling the catalog with the
    // option on is what shows that the view of every codec shape it holds is
    // code that compiles. The tests that read a view are
    // `forge_view_round_trip.rs`.
    //
    // With `sources` (ADR 0058) too, for the same reason: the interface is
    // additive, and compiling the catalog with it on shows that the interface of
    // every codec shape it holds is code that compiles. `forge_source_round_trip`
    // reads what Step 7 records here.
    let options = ForgeCompileOptions {
        views: true,
        sources: true,
        ..ForgeCompileOptions::default()
    };
    let mut sweeps: Vec<SourceSweep> = Vec::new();
    let mut codec_modules: Vec<String> = Vec::new();
    for fixture in &manifest.fixtures {
        // A fixture Rust refuses by name is not scheduled for it, and the harness
        // rendered in Step 3 does not ask for its code: the one answer the
        // harness and every other language's build read (a codec whose lines are
        // records waits on each backend, docs/adr/0014).
        if !conformance::lang_supports_fixture(fixture, Language::Rust, &resource_dir)
            .unwrap_or_else(|e| panic!("schedule {}: {e}", fixture.name))
        {
            continue;
        }
        // Where the manifest says — `<name>.scxml` here, or a standard
        // document it names by `sce:std/...` — asked of the one answer
        // every language's build uses.
        let scxml_path = fixture.document_path(&resource_dir);
        let base_dir = scxml_path
            .parent()
            .map_or_else(|| resource_dir.clone(), std::path::Path::to_path_buf);

        // Read + expand. Handing raw bytes to the compiler generates a
        // fixture with its templated nodes missing, and says nothing.
        //
        // The label stays `symmetric(fixture.name)` rather than being
        // derived from the path: `conformance.rs` compiles these same
        // fixtures under that label, and the label reaches the SCE-MAP
        // markers in the emitted code. Two producers of one fixture must
        // not disagree about what to call it.
        let loaded = load_forge_source(&scxml_path, &[])
            .unwrap_or_else(|e| panic!("expand {}: {e}", scxml_path.display()));

        // Template and XInclude targets are inputs to this build; without
        // them a fixture's template can be edited and the generated code
        // stays stale with nothing reporting it.
        for dep in &loaded.deps {
            println!("cargo:rerun-if-changed={}", dep.display());
        }

        let output = compile_forge_with_imports(
            loaded.text(),
            DocumentLabel::symmetric(&fixture.name),
            Language::Rust,
            &base_dir,
            &options,
        )
        .unwrap_or_else(|e| panic!("sce-build codegen failed for {}: {e}", fixture.name));

        // Step 7 input: a codec with a source interface is swept against its own
        // `encode`, from the frames its document holds as test vectors.
        let source_trait = format!(
            "pub trait {}Source",
            sce_build::filters::to_pascal_case(fixture.name.clone())
        );
        let has_source = output
            .files
            .iter()
            .any(|(_, code)| code.contains(&source_trait));
        if let Some(mut sweep) = codec_sweep(&fixture.name, loaded.text()) {
            if has_source && sweep.plain {
                (sweep.view, sweep.owned) = forms(&output.files, &fixture.name);
                sweeps.push(sweep);
            }
        }
        // Every file the catalog generates is a module of the sweep, whatever it
        // is: a codec names the enums and the codecs it imports as siblings. A
        // codec's test sidecar is the exception; it is a test of its own and
        // names its codec by `use super::*`.
        codec_modules.extend(output.files.iter().filter_map(|(filename, _)| {
            filename
                .strip_suffix(".rs")
                .filter(|stem| !stem.ends_with("_test"))
                .map(str::to_string)
        }));

        for (filename, code) in output.files {
            let target = Path::new(&out_dir).join(&filename);
            // The numerical-conformance harness `include!()`s each fixture
            // inside a parent `mod {}`, where a leading `#![doc = "SCE-MAP:
            // ..."]` inner attribute is illegal — strip it (same lossless
            // treatment Step 4 applies; the trace info survives as the
            // sibling `// SCE-MAP:` comment line).
            std::fs::write(&target, strip_inner_doc(&code))
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
    }

    // Step 3: render the cross-language test harness from the shared template.
    let harness =
        conformance::render_harness(&manifest, Language::Rust, &template_base, &resource_dir)
            .unwrap_or_else(|e| panic!("render conformance harness: {e}"));
    let harness_path = Path::new(&out_dir).join(conformance::harness_filename(Language::Rust));
    std::fs::write(&harness_path, harness.source)
        .unwrap_or_else(|e| panic!("write {}: {e}", harness_path.display()));

    // Step 4: RFC variant-default-uniformity Rust runtime round-
    // trip — generate the 3 default-marker fixtures outside the
    // conformance manifest (they're not part of the numerical-conformance
    // catalog; they exist solely to prove the Default-emission contract
    // at runtime, not to assert numerical oracle values). The three
    // sources are placed under a shared parent module so the outer
    // codec's `use super::codec_default_marker_arm_*` references resolve.
    //
    // The generated sources are post-processed to strip the leading
    // `#![doc = "SCE-MAP: ..."]` inner attribute so the round-trip
    // integration test can `include!()` them inside parent `mod {}`
    // blocks (inner attributes are not permitted at that injection
    // position). The same trace info is preserved as a regular
    // `// SCE-MAP:` comment on the following line of the generated
    // source, so the stripped attribute carries no unique information.
    generate_stripped_fixtures(
        &[
            "codec_default_marker_arm_a",
            "codec_default_marker_arm_b",
            "codec_variant_default_marker",
        ],
        &resource_dir,
        &out_dir,
        &options,
    );

    // Step 5: owned->borrowed projection round-trip. The *fallible*
    // `try_as_borrowed` path (`try_project_bounded` over a bounded list,
    // plus embed / present-if / list of a fallible body) has no
    // test-vector sidecar — that gate rejects repeat / tlv-chain /
    // present-if codecs — so generate the nested-fallible codecs (and
    // their locator leaf) here for an explicit runtime round-trip +
    // overflow assertion in `forge_nested_round_trip.rs`.
    generate_stripped_fixtures(
        &[
            "codec_zenoh_locator",
            "codec_nested_body",
            "codec_nested_parent",
        ],
        &resource_dir,
        &out_dir,
        &options,
    );

    // Step 6: queue kind (RFC §synth-5-P). The generator's output for a
    // queue is a module that names a runtime queue type over the element
    // codec's struct; until it is compiled, nothing shows that the names it
    // writes are the names the runtime has. One queue per bounded algorithm
    // row, sharing one element codec under a parent module so the queues'
    // `use super::queue_conformance_event` resolves.
    generate_stripped_fixtures(
        &[
            "queue_conformance_event",
            "queue_conformance_spsc",
            "queue_conformance_scq",
            "queue_conformance_node",
            "queue_conformance_intrusive",
            "queue_conformance_intrusive_many",
            "queue_conformance_segmented",
            "queue_conformance_segmented_many",
        ],
        &resource_dir,
        &out_dir,
        &options,
    );

    // Step 7: the sweeps of `forge_source_round_trip`, one for each codec whose
    // generated file holds a source interface (docs/adr/0058). The catalog's
    // codecs are swept, and so are the codecs below, which are in no catalog:
    // they hold a shape whose interface nothing else compiles (the bits of a
    // carrier an embedded variant's arm fills).
    let source_fixtures = Path::new(&manifest_dir).join("fixtures");
    println!("cargo:rerun-if-changed={}", source_fixtures.display());
    for fixture in generate_stripped_fixtures(
        &["codec_source_leaf", "codec_source_parent_tag"],
        &source_fixtures,
        &out_dir,
        &options,
    ) {
        let source_trait = format!(
            "pub trait {}Source",
            sce_build::filters::to_pascal_case(fixture.name.clone())
        );
        let has_source = fixture
            .files
            .iter()
            .any(|(_, code)| code.contains(&source_trait));
        codec_modules.extend(fixture.files.iter().filter_map(|(filename, _)| {
            filename
                .strip_suffix(".rs")
                .filter(|stem| !stem.ends_with("_test"))
                .map(str::to_string)
        }));
        if let Some(mut sweep) = codec_sweep(&fixture.name, &fixture.text) {
            if has_source && sweep.plain {
                (sweep.view, sweep.owned) = forms(&fixture.files, &fixture.name);
                sweeps.push(sweep);
            }
        }
    }
    codec_modules.sort();
    codec_modules.dedup();
    write_source_sweeps(&out_dir, &codec_modules, &sweeps);
}

/// Frames a codec's document gives as its wire oracle in a comment, for a codec
/// that cannot carry test vectors (the grammar holds only always-present
/// fields), and whose frames a random draw does not reach: a length that must
/// match the body that follows it.
const DOCUMENTED_FRAMES: &[(&str, &[&[u8]])] = &[
    (
        "codec_zenoh_source_info_ext",
        &[&[0x04, 0x00, 0xAB, 0x2A, 0x07]],
    ),
    (
        "codec_tlv_chain_basic",
        &[&[0x01, 0x10, 0x03, 0x41, 0x42, 0x43, 0x20, 0x01, 0x58]],
    ),
    ("codec_repeat_basic", &[&[0x02, 0x00, 0x01, 0x00, 0x02]]),
    // A map with its four required entries (a 16-byte id, a text, a number and
    // a byte string), and the same with both optional entries (urgent, a
    // deadline) after them.
    (
        "codec_cbor_map",
        &[
            &[
                0xA4, 0x00, 0x50, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x01, 0x62, 0x61, 0x62, 0x03, 0x01, 0x05, 0x41, 0xAA,
            ],
            &[
                0xA6, 0x00, 0x50, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x01, 0x62, 0x61, 0x62, 0x03, 0x01, 0x05, 0x41, 0xAA,
                0x07, 0xF5, 0x0C, 0x18, 0x2A,
            ],
        ],
    ),
    // The mesh envelope: its six required entries (a 16-byte id, two texts, two
    // enum values and a byte string), and the same with a subject and a
    // deadline after them.
    (
        "envelope",
        &[
            &[
                0xA6, 0x00, 0x50, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x01, 0x62, 0x61, 0x62, 0x02, 0x62, 0x65, 0x76, 0x03,
                0x01, 0x04, 0x01, 0x05, 0x41, 0xAA,
            ],
            &[
                0xA8, 0x00, 0x50, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x01, 0x62, 0x61, 0x62, 0x02, 0x62, 0x65, 0x76, 0x03,
                0x01, 0x04, 0x01, 0x05, 0x41, 0xAA, 0x06, 0x61, 0x73, 0x0C, 0x18, 0x2A,
            ],
        ],
    ),
    // Components of content lines. Enums by the text of a variant, a list of
    // enum lines, lines that are records with a required and an optional
    // parameter, a TEXT with an escape.
    (
        "codec_content_line_enums",
        &[
            b"BEGIN:VEVENT\r\nUID:u1\r\nSTATUS:COMPLETED\r\nCLASS:PRIVATE\r\n\
              X-HISTORY:NEEDS-ACTION\r\nX-HISTORY:IN-PROCESS\r\n\
              ATTENDEE;ROLE=CHAIR;PARTSTAT=COMPLETED:mailto:a@x\r\n\
              ATTENDEE;ROLE=NON-PARTICIPANT:b\r\nX-STEP;NOTE=hi:IN-PROCESS\r\n\
              COMMENT:a\\, b\r\nEND:VEVENT\r\n",
        ],
    ),
    // Required and optional strings with parameters, a list of lines, numbers and
    // a boolean, a TEXT with an escape.
    (
        "codec_content_line_event",
        &[
            b"BEGIN:VEVENT\r\nUID:e1\r\nDTSTART;TZID=Asia/Seoul:20261008T090000\r\n\
              SUMMARY:Hello\\, world\r\nORGANIZER;CN=Kim:mailto:k@x\r\n\
              EXDATE:20261009\r\nEXDATE:20261010\r\nSEQUENCE:3\r\nPRIORITY:-2\r\n\
              X-ALL-DAY:TRUE\r\nEND:VEVENT\r\n",
        ],
    ),
    // Records with parameters, a record whose line holds a list of values with a
    // parameter, one line of a TEXT list with an escaped separator, and a list
    // cut at a semicolon.
    (
        "codec_content_line_records",
        &[
            b"BEGIN:VEVENT\r\nUID:r1\r\nATTENDEE;CN=Lee;ROLE=CHAIR:mailto:l@x\r\n\
              ATTENDEE;ROLE=OPT:m\r\nEXDATE;TZID=UTC:20261001,20261002\r\n\
              EXDATE:20261003\r\nCATEGORIES:a\\,b,c\r\nGEO:37.5;127.0\r\nEND:VEVENT\r\n",
        ],
    ),
    // Required lists: two lines, a list of values, and two records.
    (
        "codec_content_line_required_lists",
        &[
            b"BEGIN:VEVENT\r\nUID:u\r\nTAG:t1\r\nTAG:t2\r\nCATS:a,b\r\nREC;KIND=k:v1\r\n\
              REC:v2\r\nEND:VEVENT\r\n",
        ],
    ),
    // A chain a later field is gated on. The first frame holds an entry of kind
    // 7 and none of kind 9, so the priority and the checksum are on the wire;
    // the second is wide and holds a 9, so the checksum is not.
    (
        "codec_chain_has_tagged",
        &[
            &[0x00, 0x07, 0x00, 0x55, 0x11, 0x12, 0x34],
            &[0x01, 0x09, 0x01, 0xAA, 0x03, 0x00, 0xBB, 0x22],
        ],
    ),
    // The marker extension (0x12) in the chain brings the slice count in place
    // of the payload; without the chain the payload is on the wire.
    (
        "codec_chain_has_marker",
        &[&[0x80, 0x12, 0x00], &[0x00, 0x01, 0xAA]],
    ),
    // An entry that follows the marker (0x4) has the long value; the others the
    // short one. The second frame has the entry's tag too.
    (
        "codec_chain_prev_carrier",
        &[
            &[0x00, 0x84, 0x11, 0x01, 0x22, 0x33],
            &[0x01, 0x84, 0x11, 0x77, 0x01, 0x22, 0x33, 0x77],
        ],
    ),
    // Two entries: the first says another follows, the second does not. The
    // second frame gives the first entry a body.
    (
        "codec_zenoh_ext_envelope",
        &[&[0x00, 0x80, 0x01], &[0x00, 0xA0, 0x05, 0x01]],
    ),
];

/// What `forge_source_round_trip` sweeps one codec with.
struct SourceSweep {
    /// The fixture's name, which is the module the codec is included as.
    name: String,
    /// The frames the document holds as test vectors: the seeds of the sweep.
    seeds: Vec<Vec<u8>>,
    /// Whether its `decode` and `encode` can be driven plainly: a codec that
    /// takes a flag-input or a tag from its caller is driven by its parent.
    plain: bool,
    /// Whether the codec has a view of its own (not the builder under another
    /// name), and an owned form: each is swept as a source of the same value.
    view: bool,
    owned: bool,
}

/// Whether the generated `files` of the codec `name` hold a view type of its own
/// and an owned type.
fn forms(files: &[(String, String)], name: &str) -> (bool, bool) {
    let ty = sce_build::filters::to_pascal_case(name.to_string());
    let (view, owned) = (
        format!("pub struct {ty}View"),
        format!("pub struct {ty}Owned"),
    );
    (
        files.iter().any(|(_, code)| code.contains(&view)),
        files.iter().any(|(_, code)| code.contains(&owned)),
    )
}

/// The sweep of the codec `name` read from `text`, or `None` when the document
/// is no codec.
fn codec_sweep(name: &str, text: &str) -> Option<SourceSweep> {
    use sce_build::forge::model::ForgeDocument;
    let parsed =
        sce_build::forge::parser::parse_forge_with_imports(text, DocumentLabel::symmetric(name))
            .unwrap_or_else(|e| panic!("parse {name}: {e}"))?;
    let ForgeDocument::Codec(m) = &parsed.document else {
        return None;
    };
    let caller_tag = m
        .variant
        .as_ref()
        .is_some_and(|v| v.tag_field.is_none() && v.peek_byte.is_none());
    let mut seeds: Vec<Vec<u8>> = m.test_vectors.iter().map(|v| v.hex.clone()).collect();
    for (codec, frames) in DOCUMENTED_FRAMES {
        if *codec == name {
            seeds.extend(frames.iter().map(|frame| frame.to_vec()));
        }
    }
    Some(SourceSweep {
        name: name.to_string(),
        seeds,
        plain: m.flag_inputs.is_empty() && !caller_tag,
        view: false,
        owned: false,
    })
}

/// Write the sweeps: the codecs as modules, then one `sweep_codec!` for each
/// codec that has a source interface.
fn write_source_sweeps(out_dir: &str, modules: &[String], sweeps: &[SourceSweep]) {
    let mut text = String::from("// Written by build.rs (Step 7).\npub mod codecs {\n");
    for name in modules {
        text.push_str(&format!(
            "    pub mod {name} {{\n        include!(concat!(env!(\"OUT_DIR\"), \"/{name}.rs\"));\n    }}\n"
        ));
    }
    text.push_str("}\n\n");
    for sweep in sweeps {
        let ty = sce_build::filters::to_pascal_case(sweep.name.clone());
        // The bytes a one-byte change of a seed is drawn from: the bytes the seeds
        // hold, and the values a flag or a length takes at its edges.
        let mut pool: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03, 0x80, 0x81, 0xff];
        for seed in &sweep.seeds {
            for &b in seed {
                if !pool.contains(&b) {
                    pool.push(b);
                }
            }
        }
        pool.sort_unstable();
        let seeds: Vec<String> = sweep.seeds.iter().map(|s| format!("vec!{s:?}")).collect();
        let view = if sweep.view {
            format!("codecs::{}::{ty}View", sweep.name)
        } else {
            String::new()
        };
        let owned = if sweep.owned {
            format!("codecs::{}::{ty}Owned", sweep.name)
        } else {
            String::new()
        };
        text.push_str(&format!(
            "sweep_codec!({name}, codecs::{name}::{ty}, &{pool:?}, vec![{seeds}], [{view}], [{owned}]);\n",
            name = sweep.name,
            seeds = seeds.join(", ")
        ));
    }
    let target = Path::new(out_dir).join("source_sweeps.rs");
    std::fs::write(&target, text).unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
}

/// Generate each named forge SCXML to Rust into `OUT_DIR`, stripping the
/// inner attributes so the result can be `include!()`-ed inside a parent
/// `mod {}` by the round-trip integration tests.
fn generate_stripped_fixtures(
    names: &[&str],
    resource_dir: &Path,
    out_dir: &str,
    options: &ForgeCompileOptions,
) -> Vec<StrippedFixture> {
    let mut generated = Vec::new();
    for name in names {
        let scxml_path = resource_dir.join(format!("{name}.scxml"));
        let loaded = load_forge_source(&scxml_path, &[])
            .unwrap_or_else(|e| panic!("expand {}: {e}", scxml_path.display()));
        for dep in &loaded.deps {
            println!("cargo:rerun-if-changed={}", dep.display());
        }
        let output = compile_forge_with_imports(
            loaded.text(),
            DocumentLabel::symmetric(name),
            Language::Rust,
            resource_dir,
            options,
        )
        .unwrap_or_else(|e| panic!("sce-build codegen failed for {name}: {e}"));
        for (filename, code) in &output.files {
            let target = Path::new(out_dir).join(filename);
            std::fs::write(&target, strip_inner_attributes(code))
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
        generated.push(StrippedFixture {
            name: (*name).to_string(),
            text: loaded.text().to_string(),
            files: output.files,
        });
    }
    generated
}

/// What `generate_stripped_fixtures` generated for one fixture: its document,
/// and every file the fixture and the codecs it imports came to.
struct StrippedFixture {
    name: String,
    text: String,
    files: Vec<(String, String)>,
}

/// Drop every inner attribute line (`#![doc = ...]`, and the `#![allow(...)]`
/// a module-shaped template such as the queue's opens with), for the same
/// reason `strip_inner_doc` drops the doc one: none is legal at an
/// `include!` injection point. The including `mod` carries the `allow`s
/// instead, as outer attributes.
fn strip_inner_attributes(code: &str) -> String {
    code.lines()
        .filter(|line| !line.trim_start().starts_with("#!["))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Drop the leading `#![doc = "SCE-MAP: ..."]` inner attribute so the
/// generated fixture can be `include!()`-ed inside a parent `mod {}` (inner
/// attributes are illegal at an `include!` injection point). Lossless — the
/// same trace lands on the sibling `// SCE-MAP:` comment line.
fn strip_inner_doc(code: &str) -> String {
    code.lines()
        .filter(|line| !line.trim_start().starts_with("#![doc"))
        .collect::<Vec<_>>()
        .join("\n")
}
