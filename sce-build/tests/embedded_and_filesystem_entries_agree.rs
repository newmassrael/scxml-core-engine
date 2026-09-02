//! The two codegen entry points must emit the same bytes.
//!
//! `sce-codegen` compiles from a path and reads templates off the
//! filesystem (`compile_scxml_lang_typed`). The browser build compiles a
//! string against templates `build.rs` embedded into the binary
//! (`compile_from_string_lang` + `template_registry::embedded_templates_for`).
//! Two entries, two template sources, one promise: the artefact a user
//! downloads from the visualizer is the artefact the CLI would have
//! written.
//!
//! Nothing checked that promise. `embedded_set_matches_the_filesystem_set`
//! proves the two template SETS are equal, which is necessary and not
//! sufficient — equal inputs still permit different outputs when the
//! routes differ, and they do differ: one resolves includes against a
//! directory, the other against a map held in memory. The registry's own
//! landing note filed exactly this as open work: *"진입점이 다르다 …
//! 출력 바이트 비교는 아직 없다."*
//!
//! Two differences are declared rather than drift, and both are excluded
//! deliberately.
//!
//! `GeneratedOutput::deps` is documented as "Empty for `from_string` /
//! in-memory routes", so asserting on it would fail for a reason that has
//! nothing to do with the bytes anybody ships.
//!
//! The PROVENANCE lines are the second, and this test found them on its
//! first run: the filesystem route writes `// From: /tmp/.tmpXXXX/reg.scxml`
//! and `// SCE-MAP: reg.scxml:2`, the in-memory route writes
//! `// From: unknown.scxml` and `// SCE-MAP: reg:2`. The in-memory entry
//! takes a NAME, not a path — it has no way to say where a string came
//! from — so these cannot agree and should not be made to. Everything else
//! matched byte for byte on that first run, which is the answer the debt
//! was asking for.
//!
//! An exclusion is where a comparison goes to die, so the exclusion is
//! measured rather than trusted: both sides must drop the SAME NUMBER of
//! lines, and what survives has to stay above a floor. A normaliser that
//! quietly erased half the file would fail both.

use std::fs;

use sce_build::template_registry::{embedded_templates_for, SUPPORTED_LANGUAGES};
use sce_build::{compile_from_string_lang, compile_scxml_lang_typed, find_template_dir_for};

/// Minimal machine that still reaches the templates which import the
/// shared macros: `state_machine`, `entry_exit_actions` (via `onentry`)
/// and `process_transition` (via a targeted transition). Same shape the
/// registry's own render tests use, for the same reason — a fixture that
/// renders only the outermost template would compare two copies of
/// nothing much and pass whatever the include machinery did.
const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a" name="reg">
  <state id="a">
    <onentry><log expr="'entered a'"/></onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// The stem both routes must agree on: the filesystem entry derives output
/// names from the path, the in-memory entry is told the name outright.
const STEM: &str = "reg";

/// Lines naming where the input came from. Matched on the marker rather
/// than on comment syntax, because six languages spell a comment six ways
/// and the marker is the same in all of them.
///
/// Each entry here was found by RUNNING this, one language at a time,
/// never by guessing: C++ produced `From:`, `SCE-MAP:` and `#line `;
/// Kotlin produced `Source:` (a path on one route, empty on the other);
/// Go produced `//line `, which is its own line directive and not the C
/// one. The backends spell provenance differently, and each spelling
/// carries the input's name into the artefact — a name the two entries
/// cannot agree on, because one splits a path and the other is handed a
/// bare string (see the call site).
///
/// `#line ` and `//line ` are listed separately on purpose. Collapsing
/// them to `line ` would match prose in any comment that happens to use
/// the word, and a marker that matches prose deletes evidence.
///
/// This is not a licence to widen the list. Every marker added here is a
/// line the comparison stops making, which is why the floors below count
/// what was dropped against what was kept.
const PROVENANCE_MARKERS: [&str; 5] = ["From:", "SCE-MAP:", "#line ", "//line ", "Source:"];

/// Drop the provenance lines, and say how many were dropped so the caller
/// can check the two sides dropped the same amount.
fn without_provenance(body: &str) -> (Vec<&str>, usize) {
    let mut kept = Vec::new();
    let mut dropped = 0usize;
    for line in body.lines() {
        if PROVENANCE_MARKERS.iter().any(|m| line.contains(m)) {
            dropped += 1;
        } else {
            kept.push(line);
        }
    }
    (kept, dropped)
}

#[test]
fn both_entries_emit_the_same_bytes_for_every_language() {
    let dir = tempfile::tempdir().expect("a temp dir for the fixture");
    let scxml_path = dir.path().join(format!("{STEM}.scxml"));
    fs::write(&scxml_path, FIXTURE).expect("the fixture is writable");
    let path_str = scxml_path.to_str().expect("utf-8 temp path");

    // Floors first, and they are the point rather than ceremony. Every
    // assertion below is an equality between two generated sets, and two
    // EMPTY sets are equal — so a registry that resolved nothing, a
    // language list that shrank to zero, or a route that started returning
    // an empty `files` would all read as agreement. Counting what was
    // actually compared is what keeps this from retiring itself.
    assert!(
        !SUPPORTED_LANGUAGES.is_empty(),
        "no languages to compare — the registry's language list is empty"
    );
    let mut languages_compared = 0usize;
    let mut files_compared = 0usize;
    let mut bytes_compared = 0usize;

    for &language in SUPPORTED_LANGUAGES {
        let template_dir = find_template_dir_for(language);
        let from_fs = compile_scxml_lang_typed(path_str, &template_dir, language)
            .unwrap_or_else(|e| panic!("{language:?}: filesystem entry failed: {e}"));

        let embedded = embedded_templates_for(language);
        assert!(
            !embedded.is_empty(),
            "{language:?}: the embedded registry resolved no templates, so the \
             in-memory route below would be comparing against nothing"
        );
        // The stem, and the caller cannot do better. Measured both ways:
        // the filesystem route splits its path, putting the FILE NAME in
        // the sourcemap (`#line 2 "reg.scxml"`) and the STEM in the output
        // filenames (`reg_sm.h`). The in-memory route is handed one string
        // and uses it for both, unsplit. So passing "reg" matches the
        // filenames and leaves the sourcemap reading "reg"; passing
        // "reg.scxml" matches the sourcemap and produces
        // `reg.scxml_sm.h`. There is no argument that satisfies both —
        // which is the shape of the difference, not a bad choice here.
        let from_memory = compile_from_string_lang(FIXTURE, STEM, &embedded, language)
            .unwrap_or_else(|e| panic!("{language:?}: in-memory entry failed: {e}"));

        assert!(
            !from_fs.files.is_empty(),
            "{language:?}: the filesystem entry emitted no file at all"
        );

        // Names first: a mismatch here is a different failure from a
        // mismatch in content, and reporting it as "byte N differs" would
        // send the reader looking in the wrong place.
        let fs_names: Vec<&str> = from_fs.files.iter().map(|(n, _)| n.as_str()).collect();
        let mem_names: Vec<&str> = from_memory.files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            fs_names, mem_names,
            "{language:?}: the two entries emitted different file sets"
        );

        for ((fs_name, fs_body), (_, mem_body)) in
            from_fs.files.iter().zip(from_memory.files.iter())
        {
            let (fs_lines, fs_dropped) = without_provenance(fs_body);
            let (mem_lines, mem_dropped) = without_provenance(mem_body);

            // The exclusion has to be symmetric. If one route stopped
            // emitting a provenance line the counts diverge here, and that
            // is a finding rather than something to normalise away.
            assert_eq!(
                fs_dropped, mem_dropped,
                "{language:?}/{fs_name}: the two entries dropped different numbers \
                 of provenance lines ({fs_dropped} vs {mem_dropped}) — the exclusion \
                 is hiding a real difference"
            );
            // And it has to be small. Erasing the file would make any two
            // outputs agree, so what remains carries the verdict.
            assert!(
                fs_lines.len() > fs_dropped,
                "{language:?}/{fs_name}: {fs_dropped} line(s) excluded against only \
                 {} kept — the comparison below would prove almost nothing",
                fs_lines.len()
            );

            // Report the FIRST differing line rather than two whole files.
            // A generated header is hundreds of lines; `assert_eq!` on the
            // pair prints both in full and the reader has to diff them by
            // eye, which is how a real difference gets skimmed past.
            if fs_lines != mem_lines {
                match fs_lines
                    .iter()
                    .zip(mem_lines.iter())
                    .position(|(a, b)| a != b)
                {
                    Some(i) => panic!(
                        "{language:?}/{fs_name}: line {} differs between the CLI and \
                         the browser build\n  filesystem: {:?}\n  in-memory : {:?}",
                        i + 1,
                        fs_lines[i],
                        mem_lines[i]
                    ),
                    None => panic!(
                        "{language:?}/{fs_name}: one entry emitted {} line(s), the \
                         other {} — a prefix matched and then one of them stopped",
                        fs_lines.len(),
                        mem_lines.len()
                    ),
                }
            }
            files_compared += 1;
            bytes_compared += fs_lines.iter().map(|l| l.len()).sum::<usize>();
        }
        languages_compared += 1;
    }

    assert_eq!(
        languages_compared,
        SUPPORTED_LANGUAGES.len(),
        "not every declared language was compared"
    );
    assert!(
        files_compared >= SUPPORTED_LANGUAGES.len(),
        "only {files_compared} file(s) compared across {languages_compared} language(s) — \
         at least one per language is the floor"
    );
    assert!(
        bytes_compared > 0,
        "every compared file was empty, so the equality above proved nothing"
    );
}
