// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a depfile declares must not depend on where the checkout lives.
//
// `Language::foreign_template_prefixes` is documented as returning path
// prefixes "relative to the template tree root", and the depfile writer
// used them to drop the other backends' templates. It applied them to
// the *absolute* path of each template, component by component. Every
// component of the checkout prefix therefore participated in the match,
// so a tree under a directory named after any backend — `/home/go/…`,
// `/srv/c/…`, a `rust/` workspace holding several repos — filtered out
// every template the render reads and left the depfile empty.
//
// Measured against the binary before the fix, with the template tree
// reached through `…/go/templates`:
//
//   -l rust, canonical path  →  21 templates declared
//   -l rust, under `go/`     →   0 templates declared
//
// Zero is the exact shape `codegen_depfile_content.rs` was built to
// reject ("the depfile names no template at all"): the build reuses a
// stale artefact after a template edit and reports success. That gate
// cannot see this one, because it renders from the canonical path — the
// defect is invisible from any tree whose prefix happens to be innocent,
// which is why it sat latent rather than failing loudly.
//
// The property asserted here is the invariant, not the one reproduction:
// relocating the template tree under a path built from every
// backend-significant directory name must leave the declared template
// set byte-identical to the canonical run's. Names are compared relative
// to the template root, since the absolute prefixes differ by
// construction.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::template_registry::SUPPORTED_LANGUAGES;
use tempfile::TempDir;

/// Lower bound on templates declared by a canonical run, read off a run
/// rather than guessed (the smallest measured was 21, on `rust`
/// statechart). Comparing two empty sets succeeds, so without a floor a
/// generator that declared nothing anywhere would pass this gate.
const MIN_DECLARED_TEMPLATES: usize = 15;

/// One probe per (backend, pipeline). Pinned exactly so a scenario that
/// stops being generated fails here rather than quietly shrinking the
/// matrix.
const EXPECTED_PROBES: usize = 12;

fn codegen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sce-codegen")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn template_root() -> PathBuf {
    repo_root().join("tools").join("codegen").join("templates")
}

/// Directory names that carry meaning *inside* the template tree, and so
/// must carry none outside it.
///
/// Derived from the language registry rather than listed, for the reason
/// `foreign_template_prefixes` is: a seventh backend adds a directory
/// name, and a hand-kept list here would leave that name untested while
/// the gate still read as full coverage. Both scopes contribute — the
/// statechart tree's `rust/`, `go/`, `c/` … and the forge tree's
/// `forge/<lang>/`, which additionally names `cpp`.
fn backend_directory_names() -> Vec<&'static str> {
    let mut names: BTreeSet<&'static str> = BTreeSet::new();
    for language in SUPPORTED_LANGUAGES {
        if let Some(owned) = language.template_owned_subdir() {
            names.insert(owned);
        }
        names.insert(language.forge_template_subdir());
    }
    names.into_iter().collect()
}

/// A checkout prefix built from every one of those names at once.
///
/// Nesting them rather than probing one at a time is both cheaper and
/// stronger: a filter that leaks any single name empties the depfile, so
/// one relocated run per backend covers the whole cross product. The
/// tree itself is reached by symlink — the defect is in how the path is
/// spelled, and copying 2.9 MB of templates twelve times would prove the
/// same thing slower.
fn hostile_template_root(work: &Path) -> PathBuf {
    let mut nest = work.to_path_buf();
    for name in backend_directory_names() {
        nest = nest.join(name);
    }
    std::fs::create_dir_all(&nest).expect("nested directory is creatable");
    let link = nest.join("templates");
    std::os::unix::fs::symlink(template_root(), &link).expect("symlink is creatable");
    link
}

/// Which pipeline a scenario routes through. The two arms derive their
/// template scope from different roots (`find_template_dir_for` versus
/// `find_template_base()/forge/<lang>`), so both need a probe.
#[derive(Clone, Copy)]
enum Pipeline {
    Statechart,
    Forge,
}

impl Pipeline {
    fn label(self) -> &'static str {
        match self {
            Pipeline::Statechart => "statechart",
            Pipeline::Forge => "forge",
        }
    }
}

const STATECHART: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="relocation_probe" initial="work" datamodel="ecmascript">
  <datamodel>
    <data id="counter" expr="0"/>
  </datamodel>
  <state id="work">
    <onentry>
      <log label="enter" expr="'work'"/>
      <assign location="counter" expr="counter + 1"/>
      <raise event="advance"/>
    </onentry>
    <transition event="advance" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

const FORGE_CODEC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="relocation_codec" version="1.0">
  <datamodel>
    <sce:field id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#;

fn write_source(dir: &Path, pipeline: Pipeline) -> PathBuf {
    let (name, body) = match pipeline {
        Pipeline::Statechart => ("relocation_probe.scxml", STATECHART),
        Pipeline::Forge => ("relocation_codec.scxml", FORGE_CODEC),
    };
    let doc = dir.join(name);
    std::fs::write(&doc, body).expect("fixture is writable");
    doc
}

/// Run `sce-codegen generate` against `templates`, writing a depfile.
///
/// `SOURCE_DATE_EPOCH` is pinned so the `generated-at` stamp cannot make
/// two runs differ for an unrelated reason; `--go-module-prefix` is
/// required by the Go route and ignored elsewhere.
fn generate(
    doc: &Path,
    out: &Path,
    lang: &str,
    templates: &Path,
    depfile: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(out).expect("output directory is creatable");
    let output = Command::new(codegen_bin())
        .arg("generate")
        .arg(doc)
        .arg("-o")
        .arg(out)
        .arg("-l")
        .arg(lang)
        .arg("--go-module-prefix")
        .arg("example.com/relocation_probe")
        .arg("--write-deps")
        .arg(depfile)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("SCE_TEMPLATE_DIR", templates)
        .output()
        .expect("sce-codegen is runnable");
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "exit {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

/// Templates named by a depfile, relative to the template root they were
/// rendered from — the only form in which a canonical run and a
/// relocated one are comparable.
///
/// A prerequisite that is a `.jinja2` outside `root` is reported as its
/// full path rather than dropped: silently ignoring it would let a
/// regression that started naming templates from somewhere else read as
/// unchanged.
fn declared_templates(depfile: &Path, root: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(depfile)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", depfile.display()));
    // Colon-space, not a bare colon: the targets are absolute paths.
    let (_, rhs) = text
        .split_once(": ")
        .unwrap_or_else(|| panic!("{} has a `target: prereqs` shape", depfile.display()));
    rhs.split(|c: char| c.is_whitespace() || c == '\\')
        .filter(|t| !t.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("jinja2"))
        .map(|p| {
            p.strip_prefix(root)
                .map(|rel| rel.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| p.display().to_string())
        })
        .collect()
}

#[test]
fn a_depfile_declares_the_same_templates_wherever_the_checkout_lives() {
    let work = TempDir::new().expect("tempdir");
    let canonical_root = template_root();
    let relocated_root = hostile_template_root(work.path());

    let mut violations: Vec<String> = Vec::new();
    let mut probes = 0usize;

    for language in SUPPORTED_LANGUAGES {
        for pipeline in [Pipeline::Statechart, Pipeline::Forge] {
            probes += 1;
            let lang = language.canonical_name();
            let case = format!("{lang}/{}", pipeline.label());

            let case_dir = work
                .path()
                .join("cases")
                .join(format!("{}_{}", lang, pipeline.label()));
            let src = case_dir.join("src");
            std::fs::create_dir_all(&src).expect("source directory is creatable");
            let doc = write_source(&src, pipeline);

            let canonical_dep = case_dir.join("canonical.d");
            if let Err(e) = generate(
                &doc,
                &case_dir.join("canonical"),
                lang,
                &canonical_root,
                &canonical_dep,
            ) {
                violations.push(format!("{case}: canonical generation failed\n{e}"));
                continue;
            }
            let canonical = declared_templates(&canonical_dep, &canonical_root);

            if canonical.len() < MIN_DECLARED_TEMPLATES {
                violations.push(format!(
                    "{case}: the canonical run declared only {} template(s), floor \
                     {MIN_DECLARED_TEMPLATES} — the comparison below would be between two \
                     near-empty sets and would pass without testing anything",
                    canonical.len(),
                ));
                continue;
            }

            let relocated_dep = case_dir.join("relocated.d");
            if let Err(e) = generate(
                &doc,
                &case_dir.join("relocated"),
                lang,
                &relocated_root,
                &relocated_dep,
            ) {
                violations.push(format!(
                    "{case}: generation from the relocated template tree failed\n{e}"
                ));
                continue;
            }
            let relocated = declared_templates(&relocated_dep, &relocated_root);

            if relocated == canonical {
                continue;
            }

            let dropped: Vec<&str> = canonical
                .difference(&relocated)
                .map(String::as_str)
                .collect();
            let added: Vec<&str> = relocated
                .difference(&canonical)
                .map(String::as_str)
                .collect();
            violations.push(format!(
                "{case}: moving the template tree to {} changed what the depfile declares \
                 — {} of {} template(s) dropped, {} added. A prefix component of the \
                 checkout is being read as a template-tree directory, so on such a tree \
                 every edit to a dropped template ships a stale artefact and the build \
                 reports success.\n  dropped: {dropped:?}\n  added:   {added:?}",
                relocated_root.display(),
                dropped.len(),
                canonical.len(),
                added.len(),
            ));
        }
    }

    assert_eq!(
        probes, EXPECTED_PROBES,
        "ran {probes} probes, expected {EXPECTED_PROBES} — the matrix shrank, so a green \
         result covers less than it claims",
    );
    assert!(
        violations.is_empty(),
        "depfile contents depend on where the checkout lives:\n\n{}",
        violations.join("\n\n"),
    );
}

/// The nesting the probe above relies on actually contains the names it
/// is meant to contain.
///
/// Without this, a `backend_directory_names()` that returned nothing —
/// a registry refactor, an accessor renamed — would build a harmless
/// path, and the probe would compare two identical runs and pass while
/// testing the opposite of what it claims.
#[test]
fn the_relocation_prefix_carries_every_backend_directory_name() {
    let names = backend_directory_names();
    assert!(
        names.len() >= SUPPORTED_LANGUAGES.len(),
        "derived {} backend directory name(s) from {} languages — the registry accessors \
         stopped reporting, so the relocation probe would run against an innocent path",
        names.len(),
        SUPPORTED_LANGUAGES.len(),
    );

    let work = TempDir::new().expect("tempdir");
    let root = hostile_template_root(work.path());
    let components: BTreeSet<String> = root
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    for name in &names {
        assert!(
            components.contains(*name),
            "the relocated template root {} has no component named `{name}`, so no probe \
             exercises that backend's directory name",
            root.display(),
        );
    }
    assert!(
        root.join(
            Language::Rust
                .template_owned_subdir()
                .expect("rust owns a subdir")
        )
        .is_dir(),
        "the relocated root {} does not resolve to a template tree",
        root.display(),
    );
}
