//! How each backend spells an enum variant — the single owner.
//!
//! ⚠ WHY THIS MODULE EXISTS. The spelling used to live inline in
//! `render_enum`'s `match`, and while that was its only consumer an
//! inline match was right. It stopped being right the moment a SECOND
//! consumer appeared: an expression in another document referring to a
//! variant (`Fuel.DSL`) must produce the identifier the declaration
//! emitted, in every backend. Re-deriving six rules at the second site
//! is how "the enum declares `Error` and the reference says `ERROR`"
//! becomes a compile failure that appears in one lane and not the rest.
//!
//! ⚠⚠ THE SPELLINGS ARE NOT ARBITRARY, AND THE DIVERGENCE IS REAL.
//! Measured by generating the same enum for every backend — one source
//! variant `error` on an enum named `Result`:
//!
//! ```text
//! cpp    Error         rust    Error
//! python ERROR         kotlin  ERROR
//! go     ResultError   c11     RESULT_ERROR
//! ```
//!
//! C has no namespace, so its members are global and must carry the
//! type name. Go's exported constants are package-level PascalCase and
//! conventionally prefixed by their type. Neither is a defect; both are
//! what the language requires.
//!
//! ⚠⚠⚠ AND AN ALL-CAPS SOURCE NAME HIDES ALL OF IT. With a variant
//! literally named `DSL`, four of the six agree by accident. A reader
//! who checks only such a fixture concludes "C11 is the odd one out",
//! generalises the other five into one rule, and that rule breaks on the
//! first lowercase variant. That misreading happened here, which is why
//! `an_all_caps_source_name_hides_the_divergence` below exists.
//!
//! ⭐ THE INVARIANT THAT MAKES DRIFT IMPOSSIBLE. A reference is the
//! declaration plus qualification, so [`variant_ref`] is defined to
//! CONTAIN [`variant_ident`] verbatim, and a test asserts it across
//! `Language::ALL`. A seventh backend cannot pass that test while
//! spelling a reference differently from its declaration — which is the
//! only way this pair can break.

use crate::filters;
use crate::generator::Language;

/// `InterConfigFuelType` → `INTER_CONFIG_FUEL_TYPE`. Private so the two
/// public functions stay the only way a caller learns a spelling.
fn to_upper_snake(s: &str) -> String {
    filters::to_snake_case(s.to_string()).to_uppercase()
}

/// The identifier a backend emits for `variant` IN THE ENUM'S OWN
/// DECLARATION. `enum_name` is the document's `name`, unconverted.
///
/// This is the definition. Everything else derives from it.
pub fn variant_ident(lang: Language, enum_name: &str, variant: &str) -> String {
    match lang {
        // C++ / Rust scope members under the type, so the member itself
        // is a plain PascalCase identifier.
        Language::Cpp | Language::Rust => filters::to_pascal_case(variant.to_string()),
        // Kotlin / Python also scope members under the type; their
        // convention for a constant member is SCREAMING_SNAKE.
        Language::Kotlin | Language::Python => to_upper_snake(variant),
        // Go has no member scoping: an exported constant is a
        // package-level PascalCase identifier, prefixed by its type.
        Language::Go => format!(
            "{}{}",
            filters::to_pascal_case(enum_name.to_string()),
            filters::to_pascal_case(variant.to_string())
        ),
        // C has no namespace mechanism at all — members land in the
        // global scope, so the type name is part of the identifier.
        Language::C11 => format!("{}_{}", to_upper_snake(enum_name), to_upper_snake(variant)),
    }
}

/// How an expression in ANOTHER document refers to that variant.
///
/// `qualified_type` is the importing side's
/// `ImportContext::enum_qualified_type`, already language-correct
/// (`SCE::Generated::X::X`, `x::X`, `x.X`, `X_t`, the bare class name for
/// Kotlin). The only thing added here is the JOIN — and for two backends
/// there is nothing to join, because the identifier is already globally
/// unique or the type is already in scope.
///
/// ⚠ The result CONTAINS [`variant_ident`] verbatim. That is not an
/// incidental property: it is the contract
/// `a_reference_always_contains_the_declared_identifier` pins, and it is
/// what stops a reference from drifting away from the declaration it has
/// to match.
pub fn variant_ref(lang: Language, qualified_type: &str, enum_name: &str, variant: &str) -> String {
    let ident = variant_ident(lang, enum_name, variant);
    match lang {
        Language::Cpp | Language::Rust => format!("{qualified_type}::{ident}"),
        // Kotlin's generated import is a wildcard, so `qualified_type`
        // IS the bare class name — joining it is correct and joining
        // anything more would produce `X.X.ERROR`.
        Language::Python | Language::Kotlin => format!("{qualified_type}.{ident}"),
        // Go: `qualified_type` is `pkg.Type` but the constant is
        // `pkg.TypeVariant`, so only the package part qualifies —
        // `ident` already carries the type name.
        Language::Go => match qualified_type.split_once('.') {
            Some((pkg, _)) => format!("{pkg}.{ident}"),
            None => ident,
        },
        // C11: the identifier is already globally unique by construction
        // (it carries the type name) and `X_t` is a typedef, not a scope
        // — there is nothing to qualify it with.
        Language::C11 => ident,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// ⭐ THE LOAD-BEARING TEST. A reference must spell the variant
    /// exactly as the declaration does, or the emitted code names an
    /// identifier that was never declared — per backend, so most lanes
    /// can stay green while one fails.
    ///
    /// Iterating `Language::ALL` rather than listing the six is what
    /// makes a seventh backend covered the day it is added.
    #[test]
    fn a_reference_always_contains_the_declared_identifier() {
        for &lang in Language::ALL {
            for (enum_name, variant) in [
                ("Result", "error"),
                ("Result", "ok"),
                ("InterConfigFuelType", "DSL"),
                ("inter_config_fuel_type", "message_timeout"),
                ("X", "a"),
            ] {
                let ident = variant_ident(lang, enum_name, variant);
                assert!(
                    !ident.is_empty(),
                    "{lang:?}: empty identifier for {enum_name}.{variant}"
                );
                let qualified = match lang {
                    Language::Cpp => format!("SCE::Generated::{enum_name}::{enum_name}"),
                    Language::C11 => format!("{enum_name}_t"),
                    Language::Kotlin => enum_name.to_string(),
                    _ => format!("m.{enum_name}"),
                };
                let reference = variant_ref(lang, &qualified, enum_name, variant);
                assert!(
                    reference.contains(&ident),
                    "{lang:?}: reference {reference:?} does not contain the declared \
                     identifier {ident:?} for {enum_name}.{variant}",
                );
            }
        }
    }

    /// The spellings this tree emits, read off generated output rather
    /// than assumed. Pinned so a refactor that "tidies" them has to say
    /// so out loud.
    #[test]
    fn the_declared_spellings_are_the_ones_each_language_requires() {
        for (lang, expected) in [
            (Language::Cpp, "Error"),
            (Language::Rust, "Error"),
            (Language::Python, "ERROR"),
            (Language::Kotlin, "ERROR"),
            (Language::Go, "ResultError"),
            (Language::C11, "RESULT_ERROR"),
        ] {
            assert_eq!(
                variant_ident(lang, "Result", "error"),
                expected,
                "{lang:?} spells `Result.error` differently than measured",
            );
        }
    }

    /// ⚠ The misreading this module was extracted to prevent. With an
    /// ALL-CAPS source name several backends agree by accident; the
    /// divergence only shows on a lowercase one. A fixture chosen from
    /// the caps case teaches the wrong rule.
    #[test]
    fn an_all_caps_source_name_hides_the_divergence() {
        let distinct = |v: &str| -> usize {
            Language::ALL
                .iter()
                .map(|&l| variant_ident(l, "Result", v))
                .collect::<BTreeSet<_>>()
                .len()
        };
        assert!(
            distinct("DSL") < distinct("dsl"),
            "the all-caps name no longer hides fewer distinct spellings \
             ({} vs {}) — if the backends genuinely converged, this \
             module's rationale needs rewriting rather than this \
             assertion relaxing",
            distinct("DSL"),
            distinct("dsl"),
        );
    }

    /// Go qualifies by PACKAGE, not by type — the type is already inside
    /// the identifier. Joining the whole `pkg.Type` would emit
    /// `pkg.Type.ResultError`, which is not a Go expression.
    #[test]
    fn go_qualifies_by_package_because_the_type_is_in_the_identifier() {
        let r = variant_ref(Language::Go, "result.Result", "Result", "error");
        assert_eq!(r, "result.ResultError");
    }

    /// C11 has nothing to qualify with: the typedef is not a scope.
    #[test]
    fn c11_has_no_qualifier_because_a_typedef_is_not_a_scope() {
        let r = variant_ref(Language::C11, "Result_t", "Result", "error");
        assert_eq!(r, "RESULT_ERROR");
    }
}
