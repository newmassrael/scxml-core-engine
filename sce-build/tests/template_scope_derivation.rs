// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which templates belong to which backend must be derived from the
// language registry, never written out by hand.
//
// The distinction that makes this necessary: `template_subdir` is a
// *loader scope* — C11 returns the tree root so it can reach the shared
// `license_header.jinja2` — while `template_owned_subdir` says whose
// templates live where. Asking the loader scope "who owns `c/`?" gets
// no answer, so every backend claims it.
//
// The defect this guards against already happened. The depfile writer
// kept its own `rust`/`kotlin`/`go` exclusion list; `python/` and `c/`
// were added to the tree later and nothing updated it. Measured
// consequence, from real depfiles:
//
//   C++ build  → 18 templates it cannot render listed as inputs
//   C11 build  → 65 (it took no filter at all), so editing one Rust
//                template regenerated all 270 C11 outputs
//
// Wrong dependencies in this direction cost build time rather than
// correctness, which is why it went unnoticed: nothing fails, the build
// is just needlessly slower. The opposite direction — a missing
// dependency — is the silent-stale-artefact defect that
// `codegen_depfile_coverage.rs` guards.
//
// A backend's templates live in two places, not one: `<lang>/` for the
// statechart templates and `forge/<lang>/` for the forge ones. The
// filter matches path *components*, so both are covered by the same
// prefix without either being named twice. That was measured rather
// than reasoned about — the fix dropped 78 templates from the C++
// depfile where 18 were predicted, and the extra 60 turned out to be
// `forge/c/` and `forge/python/`, correctly excluded. `forge/cpp/`
// stayed.
//
// After deriving the set: 130 → 52 template deps for a C++ build,
// 266 → 90 for C11.

use sce_build::generator::Language;
use sce_build::template_registry::SUPPORTED_LANGUAGES;

/// Every backend's owned subdirectory, as the tree actually holds them.
/// Written out here on purpose: this is the assertion, and deriving it
/// from the same function under test would assert nothing.
const EXPECTED_OWNED: &[(Language, Option<&str>)] = &[
    (Language::Rust, Some("rust")),
    (Language::Kotlin, Some("kotlin")),
    (Language::Go, Some("go")),
    (Language::Python, Some("python")),
    (Language::C11, Some("c")),
    // C++ templates sit at the tree root, so no path prefix identifies
    // them. That is why a C11 build still lists the root C++ templates
    // among its inputs — stated rather than silently tolerated.
    (Language::Cpp, None),
];

#[test]
fn owned_subdirectories_match_the_template_tree() {
    for (lang, expected) in EXPECTED_OWNED {
        assert_eq!(
            lang.template_owned_subdir(),
            *expected,
            "{lang:?} owns a different subdirectory than the template tree has"
        );
    }
    assert_eq!(
        EXPECTED_OWNED.len(),
        SUPPORTED_LANGUAGES.len(),
        "a backend was added to SUPPORTED_LANGUAGES without stating which \
         template subdirectory it owns — until it is listed here, its templates \
         are excluded from nobody's dependency filtering and every other \
         backend rebuilds when they change"
    );
}

/// The exclusion set every backend applies must name every *other*
/// backend's directory, and never its own.
#[test]
fn foreign_prefixes_cover_every_other_backend() {
    for &lang in SUPPORTED_LANGUAGES {
        let foreign = lang.foreign_template_prefixes();

        if let Some(own) = lang.template_owned_subdir() {
            assert!(
                !foreign.contains(&own),
                "{lang:?} excludes its own template directory {own:?} — its \
                 generated output would declare no dependency on the templates \
                 that render it"
            );
        }

        for &other in SUPPORTED_LANGUAGES {
            if other == lang {
                continue;
            }
            let Some(other_dir) = other.template_owned_subdir() else {
                continue;
            };
            // C++ and C11 both load from the root, so a directory one of
            // them owns is exactly what the other must exclude.
            if Some(other_dir) == lang.template_owned_subdir() {
                continue;
            }
            assert!(
                foreign.contains(&other_dir),
                "{lang:?} does not exclude {other:?}'s templates ({other_dir:?}) — \
                 a {other:?} template edit will regenerate every {lang:?} output"
            );
        }
    }
}

/// No backend list may be hand-kept in the depfile writer.
///
/// A source scan, so it names what it looked at rather than trusting a
/// silent pass: the point is that the exclusion is computed, and a
/// literal set of language names reappearing there is the regression.
#[test]
fn depfile_writer_holds_no_hand_kept_language_list() {
    let src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/bin/sce_codegen.rs"),
    )
    .expect("sce_codegen.rs is readable");

    let start = src
        .find("fn write_depfile(")
        .expect("write_depfile still exists — if it was renamed, retarget this gate");
    let body = &src[start..];
    let end = body.find("\nfn ").unwrap_or(body.len());
    let body = &body[..end];

    assert!(
        body.contains("foreign_template_prefixes"),
        "write_depfile no longer derives its exclusion set from the language \
         registry; a hand-kept list here is what let `python/` and `c/` go \
         unfiltered after they were added"
    );

    // Two or more backend names as string literals is the shape of the
    // list that drifted. One may legitimately appear in prose.
    let literal_names = SUPPORTED_LANGUAGES
        .iter()
        .filter_map(|l| l.template_owned_subdir())
        .filter(|dir| body.contains(&format!("\"{dir}\"")))
        .collect::<Vec<_>>();
    assert!(
        literal_names.len() < 2,
        "write_depfile names backend directories literally ({literal_names:?}) — \
         derive them from `foreign_template_prefixes` so a new backend cannot be \
         forgotten"
    );
}
