// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Which backends each `--language` route serves.
//
// A route's language menu gets spoken three times — in the flag's help,
// in the refusal a caller who named something else receives, and in the
// `replace_one_of` candidates that refusal carries on the JSON wire —
// and all three had drifted from the dispatcher and from each other:
//
//   * `generate-w3c --help` named `c11`, which the dispatcher answers
//     with exit 20, and omitted `python`, which generates 202 fixtures.
//   * `generate-conformance --help` named five languages and called
//     itself the "single source of truth for all 5 languages" while its
//     dispatcher emitted a 188 KB C harness for a sixth.
//   * `generate` and `orchestrate` both omitted `python`, which works,
//     even though `check`'s own help promises "`check -l X` and
//     `generate -l X` always agree" and correctly lists all six.
//   * every refusal, on every route, ended in the sentence
//     "Use rust, cpp, kotlin, or go." — a four-language set that was
//     already stale twice over, and rode onto the wire as the
//     machine-readable `fix.candidates` an external consumer repairs
//     from.
//
// [`Language::ALL`] and [`crate::template_registry::SUPPORTED_LANGUAGES`]
// each already carry a doc comment telling callers to derive the menu
// rather than restate it — the visualiser learned it by offering a Go
// button the dispatcher rejected. This module is where a *route* does
// the same, since the answer is per-route: what `generate` serves and
// what `generate-integration` serves are different sets, and a single
// global list cannot say so.
//
// This table declares; the dispatchers in `sce_codegen` implement. That
// leaves exactly one way for them to disagree, and
// `sce-build/tests/cli_language_surface.rs` closes it by running the
// real binary once per (route, language) pair and comparing what it
// accepts against what its own `--help` says.

use crate::generator::Language;

/// A `sce-codegen` subcommand that takes `--language`, together with the
/// backends it serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageRoute {
    /// `generate` — one document, one backend.
    Generate,
    /// `check` — validate without writing; `--language` is repeatable
    /// and selects which backends the document is judged against.
    Check,
    /// `orchestrate` — multi-document generate with a cross-doc registry.
    Orchestrate,
    /// `generate-w3c` — batch W3C statechart conformance suite.
    GenerateW3c,
    /// `generate-integration` — batch integration fixtures for the
    /// backends that keep a regenerated tree.
    GenerateIntegration,
    /// `generate-conformance` — cross-language numerical harness.
    GenerateConformance,
    /// `list-fixtures` — `--language` is a filter over the listing
    /// rather than an output target, but it parses the same names and so
    /// owes callers the same menu.
    ListFixtures,
}

impl LanguageRoute {
    /// Every route that takes `--language`.
    ///
    /// A new one lands here and the surface test picks it up; a route
    /// that forgets to would be caught by that test's lower bound only
    /// if the count moved, so the test also derives its own worklist
    /// from `--help` and fails on a subcommand it finds there and not
    /// here.
    pub const ALL: &'static [LanguageRoute] = &[
        LanguageRoute::Generate,
        LanguageRoute::Check,
        LanguageRoute::Orchestrate,
        LanguageRoute::GenerateW3c,
        LanguageRoute::GenerateIntegration,
        LanguageRoute::GenerateConformance,
        LanguageRoute::ListFixtures,
    ];

    /// The subcommand name, spelled as a caller types it.
    pub fn subcommand(self) -> &'static str {
        match self {
            LanguageRoute::Generate => "generate",
            LanguageRoute::Check => "check",
            LanguageRoute::Orchestrate => "orchestrate",
            LanguageRoute::GenerateW3c => "generate-w3c",
            LanguageRoute::GenerateIntegration => "generate-integration",
            LanguageRoute::GenerateConformance => "generate-conformance",
            LanguageRoute::ListFixtures => "list-fixtures",
        }
    }

    /// The backends this route serves, in [`Language::ALL`] order.
    ///
    /// Routes that serve every backend say so by naming `Language::ALL`,
    /// not by repeating six variants: a seventh backend then reaches
    /// them without an edit, and only the routes that genuinely restrict
    /// their set carry a list to keep current.
    pub fn languages(self) -> &'static [Language] {
        match self {
            LanguageRoute::Generate
            | LanguageRoute::Check
            | LanguageRoute::Orchestrate
            | LanguageRoute::GenerateConformance
            | LanguageRoute::ListFixtures => Language::ALL,
            // No C11 statechart emitter exists for the batch suite, so
            // it has nothing to render for that backend. Single-document
            // `generate -l c11` is a different emitter and does work —
            // which is why the refusal below has to say so rather than
            // leave the caller reading a five-name menu.
            LanguageRoute::GenerateW3c => &[
                Language::Rust,
                Language::Cpp,
                Language::Kotlin,
                Language::Go,
                Language::Python,
            ],
            // C++ and C11 integration fixtures emit at CMake build time
            // through `sce_generate_static_integration_test`, so there is
            // no committed tree for this subcommand to regenerate.
            LanguageRoute::GenerateIntegration => &[
                Language::Rust,
                Language::Kotlin,
                Language::Go,
                Language::Python,
            ],
        }
    }

    /// Whether this route serves `language`.
    pub fn serves(self, language: Language) -> bool {
        self.languages().contains(&language)
    }

    /// Why the backends this route does not serve are absent, phrased to
    /// follow the menu in a refusal or in help.
    ///
    /// `None` means the route serves every backend, so there is nothing
    /// to explain — and a route that grows a restriction without a
    /// reason is a route whose refusal tells the caller to go away
    /// without saying where.
    pub fn exclusion_reason(self) -> Option<&'static str> {
        match self {
            LanguageRoute::GenerateW3c => Some(
                "C11 is absent because no C11 W3C statechart emitter exists yet (RFC §5.J.1); \
                 single-document `generate -l c11` uses a different emitter and does work",
            ),
            LanguageRoute::GenerateIntegration => Some(
                "cpp and c11 are absent because their integration fixtures emit at CMake build \
                 time through `sce_generate_static_integration_test`, leaving no committed tree \
                 for this subcommand to regenerate",
            ),
            _ => None,
        }
    }

    /// The menu as prose: `rust, cpp, kotlin, go, python`.
    ///
    /// Spelled with [`Language::canonical_name`] so every name printed
    /// here is one [`std::str::FromStr`] takes back — a menu offering an
    /// alias the parser happens to accept today would be a second
    /// spelling contract.
    pub fn menu(self) -> String {
        self.languages()
            .iter()
            .map(|l| l.canonical_name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The same menu as the `replace_one_of` candidate list carried by
    /// `cli/unknown-language` on the JSON wire.
    ///
    /// Per-route rather than global on purpose: offering `c11` to a
    /// caller whose `generate-w3c` invocation just failed would be a
    /// repair that fails again.
    pub fn candidates(self) -> Vec<String> {
        self.languages()
            .iter()
            .map(|l| l.canonical_name().to_string())
            .collect()
    }

    /// The `--language` flag's one-line help (`-h`) for this route.
    ///
    /// `lead` is the route-specific phrase for what the flag selects
    /// here; the menu is appended from this table so it cannot be
    /// restated wrongly. Formatted rather than written as a doc comment
    /// for the reason `w3c_registry_flag_help` is: a doc comment makes
    /// the same fact into a second sentence, and that is what drifted.
    ///
    /// Both help forms carry the menu, because `-h` is where most
    /// callers meet it — a summary that dropped the menu to stay short
    /// would send them to `--help` to learn something the summary used
    /// to tell them.
    pub fn flag_summary(self, lead: &str) -> String {
        format!("{lead} ({}).", self.menu())
    }

    /// The `--language` flag's long help (`--help`) for this route: the
    /// summary, plus why the backends it does not serve are absent.
    pub fn flag_help(self, lead: &str) -> String {
        match self.exclusion_reason() {
            Some(reason) => format!("{}\n\n{reason}.", self.flag_summary(lead)),
            None => self.flag_summary(lead),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menu names have to round-trip: a caller who copies one out of
    /// help or out of `fix.candidates` and hands it back must be
    /// accepted. This is the property that makes the menu a repair
    /// instruction rather than a description.
    #[test]
    fn every_menu_name_parses_back_to_its_language() {
        for &route in LanguageRoute::ALL {
            for &language in route.languages() {
                let name = language.canonical_name();
                let parsed: Language = name.parse().unwrap_or_else(|e| {
                    panic!(
                        "{}: menu name {name:?} does not parse: {e}",
                        route.subcommand()
                    )
                });
                assert_eq!(
                    parsed,
                    language,
                    "{}: menu name {name:?} parses to a different language",
                    route.subcommand()
                );
            }
        }
    }

    /// A route that serves a strict subset owes the caller a reason, and
    /// a route that serves everything has nothing to explain. Without
    /// this, a future restriction lands as a bare refusal.
    #[test]
    fn restricted_routes_explain_the_restriction() {
        for &route in LanguageRoute::ALL {
            let restricted = route.languages().len() < Language::ALL.len();
            assert_eq!(
                restricted,
                route.exclusion_reason().is_some(),
                "{}: serves {} of {} backends but exclusion_reason() is {:?}",
                route.subcommand(),
                route.languages().len(),
                Language::ALL.len(),
                route.exclusion_reason(),
            );
        }
    }

    /// Every route's menu is non-empty and free of duplicates, and its
    /// order is `Language::ALL`'s. Order is not cosmetic: two routes
    /// printing the same set in different orders reads as two different
    /// sets to anyone comparing help output.
    #[test]
    fn menus_are_canonical_subsets() {
        for &route in LanguageRoute::ALL {
            let langs = route.languages();
            assert!(!langs.is_empty(), "{}: empty menu", route.subcommand());
            let expected: Vec<Language> = Language::ALL
                .iter()
                .copied()
                .filter(|l| langs.contains(l))
                .collect();
            assert_eq!(
                langs,
                expected.as_slice(),
                "{}: menu is not Language::ALL order (duplicates or reordering)",
                route.subcommand()
            );
        }
    }

    /// Both help forms have to carry the menu verbatim, because the
    /// surface test parses help output to learn what a route claims. If
    /// either dropped it the gate would be reading something else — and
    /// `-h` is the form most callers see.
    #[test]
    fn both_help_forms_carry_the_menu() {
        for &route in LanguageRoute::ALL {
            for (form, text) in [
                ("flag_summary", route.flag_summary("Target language")),
                ("flag_help", route.flag_help("Target language")),
            ] {
                assert!(
                    text.contains(&route.menu()),
                    "{}: {form} does not contain its own menu: {text}",
                    route.subcommand()
                );
            }
        }
    }
}
