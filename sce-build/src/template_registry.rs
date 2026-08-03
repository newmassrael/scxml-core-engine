// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Compile-time registry of the Jinja2 template tree.
//!
//! `build.rs` derives [`EMBEDDED_TEMPLATES`] from
//! `tools/codegen/templates/`, so the in-memory template path carries no
//! hand-maintained list of what that tree contains. Callers with a
//! filesystem keep using [`crate::generator::load_templates`]; callers
//! without one (WASM) use [`embedded_templates_for`]. Both name
//! templates identically, which is what lets the two sources be compared
//! rather than merely assumed equal.
//!
//! This module is deliberately *not* behind the `wasm` feature. The
//! registry's only previous consumer was, which meant no default build
//! ever compiled it and no test ever rendered from it — a template
//! missing from the list was invisible to every gate the project had.

use crate::generator::Language;

include!(concat!(env!("OUT_DIR"), "/embedded_templates.rs"));

/// Cross-backend macro directory, shared by every language and
/// therefore addressed by the same name from every scope.
const SHARED_MACRO_PREFIX: &str = "_macros/";

/// The templates visible to `language`, named as that language's
/// templates address them.
///
/// This mirrors [`crate::generator::load_templates`] exactly, and the
/// mirroring is the point — the two must agree for a given tree or the
/// native and WASM generators diverge:
///
/// - A language whose subdir is `""` (C++, C11) sees the whole tree
///   under full relative names, because the filesystem loader walks
///   from the root for those languages.
/// - Any other language sees its own subdir with the prefix stripped,
///   plus `_macros/` unstripped, because the filesystem loader walks
///   the language directory and then re-walks `_macros/` with the tree
///   root as its base.
pub fn embedded_templates_for(language: Language) -> Vec<(&'static str, &'static str)> {
    let subdir = language.template_subdir();
    if subdir.is_empty() {
        return EMBEDDED_TEMPLATES.to_vec();
    }
    let prefix = format!("{subdir}/");
    EMBEDDED_TEMPLATES
        .iter()
        .filter_map(|&(name, content)| {
            if let Some(scoped) = name.strip_prefix(&prefix) {
                Some((scoped, content))
            } else if name.starts_with(SHARED_MACRO_PREFIX) {
                Some((name, content))
            } else {
                None
            }
        })
        .collect()
}

/// Every language the embedded registry can serve.
///
/// Callers that present a language menu should derive it from this
/// rather than restating the list; the WASM visualizer previously
/// restated it and offered a Go button that the dispatcher rejected.
pub const SUPPORTED_LANGUAGES: &[Language] = &[
    Language::Cpp,
    Language::C11,
    Language::Rust,
    Language::Kotlin,
    Language::Go,
    Language::Python,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal machine that still reaches the templates which import the
    /// shared macros: `state_machine`, `entry_exit_actions` (via
    /// `onentry`) and `process_transition` (via a targeted transition).
    const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a" name="reg">
  <state id="a">
    <onentry><log expr="'entered a'"/></onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

    /// The registry is non-empty and every language resolves a set.
    ///
    /// A silently empty registry would turn every generation into a
    /// "template not found" at render time, which is the failure this
    /// module exists to prevent.
    #[test]
    fn every_language_resolves_a_non_empty_template_set() {
        assert!(
            !EMBEDDED_TEMPLATES.is_empty(),
            "embedded template registry is empty — build.rs did not walk the tree"
        );
        for &language in SUPPORTED_LANGUAGES {
            let set = embedded_templates_for(language);
            assert!(
                !set.is_empty(),
                "{language:?} resolves no templates from the embedded registry"
            );
        }
    }

    /// Every `{% import %}` / `{% include %}` / `{% extends %}` target
    /// names a template present in the same set.
    ///
    /// minijinja binds those at render time, so an unresolvable target
    /// costs nothing at compile time and surfaces only when a render
    /// happens to take that branch. This walks the edges directly, which
    /// covers branches no fixture reaches — `mesh/cpp/parallel_final`
    /// sits behind `partition_context_present`, and the render tests
    /// below would never touch it.
    ///
    /// Targets assembled from an expression cannot be resolved to a
    /// single name, so those are checked by prefix instead — see
    /// [`Reference`]. The render tests remain the ground truth.
    ///
    /// Only templates the language *owns* are checked as referrers.
    /// C++ and C11 scope to the tree root, so their environment also
    /// holds every other backend's subdirectory; those templates name
    /// their own siblings unqualified (`go/state_machine.go.jinja2`
    /// includes `license_header.go.jinja2`) and resolve only under the
    /// scope that strips their prefix. They are unreachable from a C++
    /// render, so their presence in that environment is slack rather
    /// than breakage — but it is why referrers are filtered here.
    #[test]
    fn every_template_reference_resolves_within_its_language_set() {
        for &language in SUPPORTED_LANGUAGES {
            let set = embedded_templates_for(language);
            let names: std::collections::HashSet<&str> =
                set.iter().map(|&(name, _)| name).collect();
            for &(name, content) in set.iter().filter(|&&(name, _)| owns(language, name)) {
                for reference in template_references(content) {
                    match reference {
                        Reference::Literal(target) => assert!(
                            names.contains(target.as_str()),
                            "{language:?}: {name} references {target}, \
                             which is absent from that language's template set"
                        ),
                        Reference::DynamicPrefix(prefix) => assert!(
                            names.iter().any(|n| n.starts_with(&prefix)),
                            "{language:?}: {name} builds a template name under {prefix}, \
                             but that language's template set has nothing there"
                        ),
                    }
                }
            }
        }
    }

    /// Whether `name`, as it appears in `language`'s template set, is a
    /// template that language can render.
    ///
    /// Every name is owned except another backend's subdirectory, which
    /// only the root-scoped languages ever see.
    fn owns(language: Language, name: &str) -> bool {
        !SUPPORTED_LANGUAGES
            .iter()
            .map(|other| other.template_subdir())
            .filter(|subdir| !subdir.is_empty() && *subdir != language.template_subdir())
            .any(|subdir| name.starts_with(&format!("{subdir}/")))
    }

    /// A resolvable target of an `import` / `include` / `extends` tag.
    ///
    /// `actions/foreach.jinja2` dispatches on the action kind with
    /// `{% include 'actions/' + body_action.type + '.jinja2' %}`, so not
    /// every target is a name known before render. For those, the
    /// literal head is still checked — it is what makes the difference
    /// between "this directory is missing from the set" and "this one
    /// action kind is missing", and the former is the failure mode this
    /// whole module exists to catch.
    #[derive(Debug)]
    enum Reference {
        Literal(String),
        DynamicPrefix(String),
    }

    /// Extract the target of every `import` / `include` / `extends` tag
    /// in a template body.
    fn template_references(content: &str) -> Vec<Reference> {
        let mut found = Vec::new();
        for (index, _) in content.match_indices("{%") {
            let tag = &content[index..];
            let Some(end) = tag.find("%}") else { continue };
            let tag = &tag[..end];
            let keyword = tag
                .trim_start_matches("{%")
                .trim_start_matches('-')
                .trim_start();
            if !["import ", "include ", "extends "]
                .iter()
                .any(|k| keyword.starts_with(k))
            {
                continue;
            }
            // Both quote styles are legal minijinja; take whichever
            // opens first so a target containing the other is intact.
            let quote = match (tag.find('\''), tag.find('"')) {
                (Some(single), Some(double)) => {
                    if single < double {
                        '\''
                    } else {
                        '"'
                    }
                }
                (Some(_), None) => '\'',
                (None, Some(_)) => '"',
                (None, None) => continue,
            };
            let mut parts = tag.splitn(3, quote);
            parts.next();
            let Some(target) = parts.next() else { continue };
            // A concatenation operator directly after the closing quote
            // means the literal is a fragment, not the whole name.
            let concatenated = parts
                .next()
                .map(|rest| {
                    let rest = rest.trim_start();
                    rest.starts_with('+') || rest.starts_with('~')
                })
                .unwrap_or(false);
            found.push(if concatenated {
                Reference::DynamicPrefix(target.to_string())
            } else {
                Reference::Literal(target.to_string())
            });
        }
        found
    }

    /// The registry actually renders, for every language it advertises.
    ///
    /// This is the regression guard for the defect that motivated the
    /// module: `state_machine.jinja2` imported `_macros/sce_map_marker`
    /// while the hand-written WASM list omitted it, so every browser
    /// generation failed with "template not found" while every native
    /// test stayed green.
    #[test]
    fn embedded_registry_renders_for_every_language() {
        for &language in SUPPORTED_LANGUAGES {
            let templates = embedded_templates_for(language);
            let borrowed: Vec<(&str, &str)> = templates.clone();
            crate::compile_from_string_lang(FIXTURE, "reg", &borrowed, language).unwrap_or_else(
                |e| panic!("{language:?} failed to render from the embedded registry: {e}"),
            );
        }
    }

    /// The embedded set and the filesystem set agree, name for name.
    ///
    /// Equality here is what makes the two sources interchangeable; if
    /// they drift, native output and WASM output drift with them.
    #[test]
    fn embedded_set_matches_the_filesystem_set() {
        for &language in SUPPORTED_LANGUAGES {
            let dir = crate::find_template_dir_for(language);
            let mut env = crate::generator::new_env();
            crate::generator::load_templates(&mut env, &dir).expect("filesystem templates load");
            let mut from_disk: Vec<String> =
                env.templates().map(|(name, _)| name.to_string()).collect();
            let mut from_registry: Vec<String> = embedded_templates_for(language)
                .iter()
                .map(|&(name, _)| name.to_string())
                .collect();
            from_disk.sort();
            from_registry.sort();
            assert_eq!(
                from_disk, from_registry,
                "{language:?}: embedded registry and filesystem tree disagree"
            );
        }
    }
}
