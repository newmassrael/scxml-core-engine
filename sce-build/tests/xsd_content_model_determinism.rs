// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// XSD 1.0 content-model determinism guard for the forge schemas.
//
// `sce-codegen` validates every input SCXML against `schemas/sce-forge.xsd`
// (which `xs:import`s `sce-forge-ext.xsd`) through libxml2. libxml2 compiles
// the whole schema graph up front, so a non-deterministic content model is a
// *schema-compile* failure that breaks validation for ALL inputs, not a
// per-document diagnostic.
//
// XSD 1.0's Unique Particle Attribution (UPA) rule forbids a content model
// where an element could match more than one particle without lookahead. The
// classic trap in these schemas is a named `sce:*` element particle sharing a
// compositor with an `xs:any namespace="##any"` (or `"##targetNamespace"`)
// wildcard: the wildcard also matches the target namespace, so the named
// particle and the wildcard overlap. Lenient processors (libxml2 <= 2.11)
// silently accept it; strict ones (libxml2 >= 2.15) reject it at compile
// time, which would take down codegen on the next host upgrade.
//
// The local CI host's libxml2 cannot be relied on to catch this (it varies by
// distro and is often lenient), so this guard is host-independent: it parses
// the schema text structurally and asserts the invariant directly.
//
// Design rule enforced: a target-namespace-overlapping element wildcard
// (`##any` / `##targetNamespace`) must be the ONLY element particle in its
// compositor. Structural ordering of `sce:*` particles (e.g. "params precede
// body") is owned by the parser/expander SSOT (`sce-build/src/template.rs`,
// `forge::parser`), never re-encoded in the XSD — so a lone wildcard is the
// correct, deterministic shape for every forge body element.

use std::path::{Path, PathBuf};

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

/// Repo root = the crate manifest dir's parent (`sce-build/..`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// True for an `xs:any` whose `namespace` overlaps the schema target
/// namespace (`##any` matches every namespace; `##targetNamespace` matches it
/// explicitly). `##other` and an explicit foreign namespace list do not
/// overlap a target-namespace `sce:*` particle, so they are UPA-safe.
fn wildcard_overlaps_target(node: &roxmltree::Node) -> bool {
    if !(node.tag_name().namespace() == Some(XSD_NS) && node.tag_name().name() == "any") {
        return false;
    }
    // Absent `namespace` defaults to `##any` per the XSD spec.
    matches!(
        node.attribute("namespace").unwrap_or("##any"),
        "##any" | "##targetNamespace"
    )
}

/// True for a named element particle (`xs:element` carrying `name=` or
/// `ref=`). A bare structural compositor is not a named particle.
fn is_named_element_particle(node: &roxmltree::Node) -> bool {
    node.tag_name().namespace() == Some(XSD_NS)
        && node.tag_name().name() == "element"
        && (node.has_attribute("name") || node.has_attribute("ref"))
}

/// Collect "compositor sequence shares a target-overlapping wildcard with a
/// named element particle" violations across one schema file, as human
/// readable `"<file>:<line> ..."` strings.
fn violations(rel: &str) -> Vec<String> {
    let text = read(rel);
    let doc = roxmltree::Document::parse(&text)
        .unwrap_or_else(|e| panic!("{rel} is not well-formed XML: {e}"));

    let mut out = Vec::new();
    for compositor in doc.descendants().filter(|n| {
        n.tag_name().namespace() == Some(XSD_NS)
            && matches!(n.tag_name().name(), "sequence" | "choice" | "all")
    }) {
        let particles: Vec<roxmltree::Node> =
            compositor.children().filter(|n| n.is_element()).collect();
        let has_named = particles.iter().any(is_named_element_particle);
        for p in &particles {
            if wildcard_overlaps_target(p) && has_named {
                let line = text[..p.range().start].lines().count();
                out.push(format!(
                    "{rel}:{line} — `xs:any namespace=\"{}\"` shares a compositor \
                     with a named element particle (XSD 1.0 UPA violation)",
                    p.attribute("namespace").unwrap_or("##any")
                ));
            }
        }
    }
    out
}

/// Every forge schema content model must be UPA-deterministic so libxml2
/// (any version) can compile it. See module header for the failure mode.
#[test]
fn forge_schemas_have_no_wildcard_named_particle_overlap() {
    let mut all = Vec::new();
    for rel in ["schemas/sce-forge.xsd", "schemas/sce-forge-ext.xsd"] {
        all.extend(violations(rel));
    }
    assert!(
        all.is_empty(),
        "XSD 1.0 UPA violation(s) — strict libxml2 (>= 2.15) will reject the \
         schema at compile time, breaking ALL codegen. Move the `sce:*` \
         ordering to the parser SSOT and leave a lone wildcard:\n  {}",
        all.join("\n  ")
    );
}

/// Sanity: the guard's particle detector actually fires. A hand-built schema
/// fragment with the exact antipattern must be flagged, so a future refactor
/// of the detector cannot silently turn the guard into a no-op.
#[test]
fn guard_detects_a_known_antipattern_fragment() {
    let fragment = r###"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="bad">
        <xs:sequence>
          <xs:element name="p" minOccurs="0" maxOccurs="unbounded"/>
          <xs:any namespace="##any" processContents="lax"
                  minOccurs="0" maxOccurs="unbounded"/>
        </xs:sequence>
      </xs:complexType>
    </xs:schema>"###;
    let doc = roxmltree::Document::parse(fragment).expect("fragment parses");
    let seq = doc
        .descendants()
        .find(|n| n.tag_name().name() == "sequence")
        .expect("has sequence");
    let particles: Vec<roxmltree::Node> = seq.children().filter(|n| n.is_element()).collect();
    let has_named = particles.iter().any(is_named_element_particle);
    let flagged = particles
        .iter()
        .any(|p| wildcard_overlaps_target(p) && has_named);
    assert!(flagged, "guard failed to flag the known UPA antipattern");
}
