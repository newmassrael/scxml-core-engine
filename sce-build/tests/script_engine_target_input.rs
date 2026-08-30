// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The target engine is a codegen INPUT, and a backend that cannot honour it
//! refuses.
//!
//! `datamodel="ecmascript"` is a claim about a language; Lua is not that
//! language, so a backend running the datamodel on Lua must translate, at
//! build time or at run time. Four backends lower at build time and two hand
//! the engine the author's source — and that split used to be a property of
//! the templates alone, with no way for a caller to ask for the other side.
//! `--script-engine` is that ask (`docs/SCE_LUA_TRANSLATION_SEAM.md`).
//!
//! What is held here:
//!
//!  1. **A run that does not ask is unchanged.** The default is each
//!     backend's existing derived answer, so the flag's arrival moved no
//!     artifact.
//!  2. **The pair is emitted by ONE filter or not at all.** A template site
//!     that assembled `ScriptSource::lua("…", "…")` by hand would be one edit
//!     away from lowering one half and not the other, and every diagnostic
//!     the artifact raised would then name a language the author never wrote.
//!  3. **A half-migrated backend refuses.** This is the case with teeth: the
//!     seam document states that splitting a subset is not a smaller version
//!     of the change, because the engine would receive Lua from some sites and
//!     ECMAScript from others *in one session*, with no diagnostic saying so.
//!     The refusal is derived from a count taken off the template tree, so it
//!     lifts by itself when the last site moves — the shape the mesh-rpc
//!     refusal uses.
//!  4. **Each refusal names its own reason.** A backend asked for Lua is
//!     part-way through a migration this can count; one asked for ECMAScript
//!     has no source-emitting arm at all. Naming the wrong one sends a reader
//!     to the wrong repair — which the first cut of this feature did.

use sce_build::generator::{Language, ScriptEngineTarget};

/// Every backend supports the target it already emits for, so adding the
/// flag cannot have taken an artifact away from anyone.
#[test]
fn every_backend_supports_its_own_default() {
    for language in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let default = language.default_script_engine_target();
        assert!(
            language.supports_script_engine_target(default),
            "{} refuses the engine it emits for by default ({})",
            language.canonical_name(),
            default.wire_name()
        );
    }
}

/// The default is the answer the manifest has always reported, spelled two
/// ways that must not drift.
#[test]
fn the_default_target_and_the_manifest_field_are_one_answer() {
    for language in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        assert_eq!(
            language.default_script_engine_target().wire_name(),
            language.script_engine_language(),
            "{} reports one engine language on the manifest and another as its \
             codegen default",
            language.canonical_name()
        );
    }
}

/// The flag's vocabulary is the wire's, so a caller cannot spell the
/// selection one way and read it back another.
#[test]
fn the_flag_accepts_exactly_the_wire_vocabulary() {
    for spelling in sce_build::manifest::SCRIPT_ENGINE_LANGUAGES {
        let parsed = ScriptEngineTarget::parse(spelling)
            .unwrap_or_else(|| panic!("the wire admits '{spelling}' and the flag does not"));
        assert_eq!(parsed.wire_name(), *spelling);
    }
    assert!(ScriptEngineTarget::parse("quickjs").is_none());
    assert!(ScriptEngineTarget::parse("").is_none());
}

/// `ScriptEngineTarget::ALL` is the wire vocabulary, not a second copy of it.
///
/// The constant exists so a site that must sweep every engine language — the
/// Kotlin conformance gate's population check is the first — can ask the enum
/// instead of restating the two. A constant that could fall behind would make
/// those sweeps quietly narrower than they read, which on this seam means an
/// emittable artifact no lane runs.
///
/// Held to `SCRIPT_ENGINE_LANGUAGES` rather than to a literal here, because
/// that list is already pinned to the manifest schema's `enum`: a third
/// target cannot reach the wire without reaching this constant, and cannot be
/// added here without the schema admitting it.
#[test]
fn the_target_population_is_the_wire_vocabulary() {
    use std::collections::BTreeSet;

    let from_wire: BTreeSet<&str> = sce_build::manifest::SCRIPT_ENGINE_LANGUAGES
        .iter()
        .copied()
        .collect();
    let from_all: BTreeSet<&str> = ScriptEngineTarget::ALL
        .iter()
        .map(|target| target.wire_name())
        .collect();
    assert_eq!(
        from_wire, from_all,
        "ScriptEngineTarget::ALL and the wire's script-engine vocabulary \
         disagree. A target on the wire and missing from ALL is one that every \
         sweep written against ALL silently skips."
    );
}

/// The four lowering backends cannot emit the author's source, and say so
/// without pretending a migration is in progress.
#[test]
fn a_lowering_backend_refuses_the_source_target() {
    for language in [
        Language::Rust,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        assert_eq!(
            language.default_script_engine_target(),
            ScriptEngineTarget::Lua,
            "{} was expected to lower at build time",
            language.canonical_name()
        );
        assert!(
            !language.supports_script_engine_target(ScriptEngineTarget::EcmaScript),
            "{} claims it can emit the author's source; no template arm does that",
            language.canonical_name()
        );
    }
}

/// C++ hands its engine the author's source today, and refuses the Lua target
/// for as long as any site still does.
///
/// This case is deliberately written against the COUNT rather than against a
/// fixed expectation of "refuses": when the migration finishes the count goes
/// to zero, the refusal lifts, and this case follows it instead of having to
/// be rewritten to stay true.
#[test]
fn cpp_refuses_the_lua_target_while_sites_remain() {
    assert_eq!(
        Language::Cpp.default_script_engine_target(),
        ScriptEngineTarget::EcmaScript
    );

    let remaining = Language::Cpp.unmigrated_expression_sites();
    let unknown = Language::Cpp.unclassified_expression_sites();
    // BOTH lists. Asking only about `remaining` would have passed on
    // 2026-08-28 with 0 unmigrated and 9 unadjudicated sites — the escape
    // hatch defeating the gate that owns it.
    assert_eq!(
        Language::Cpp.supports_script_engine_target(ScriptEngineTarget::Lua),
        remaining.is_empty() && unknown.is_empty(),
        "the Lua target is offered exactly when NO site is left behind and NO \
         site is unadjudicated — {} unmigrated:\n  {}\n{} unclassified:\n  {}",
        remaining.len(),
        remaining.join("\n  "),
        unknown.len(),
        unknown.join("\n  ")
    );
}

/// The migration count is a measurement, and a measurement that reads nothing
/// is not a passing one.
///
/// A floor, not an equality: moving a site must not have to touch this file.
/// But a scan that silently stopped finding sites — a renamed field, a
/// template tree that moved — would otherwise report the migration COMPLETE
/// and hand the Lua target to a backend that never moved a line. That is the
/// "empty sweep reads as a pass" failure, and it is the one this guards.
#[test]
fn the_migration_scan_is_actually_reading_the_templates() {
    let remaining = Language::Cpp.unmigrated_expression_sites();
    if remaining.is_empty() {
        // ⚠ The independent witness, and it has to be independent. Asking
        // `supports_script_engine_target` here — as the first cut of this case
        // did — asks the SAME scan a second time: break the scanner and both
        // answers say "migration complete", and the case passes while handing
        // the Lua target to a backend that never moved a line. Measured
        // 2026-08-28: that break left this case green.
        let migrated = Language::Cpp.migrated_expression_sites();
        assert!(
            !migrated.is_empty(),
            "no site is unmigrated AND no site routes through the pair filter, \
             which is what a scanner reading nothing looks like — not a finished \
             migration"
        );
        return;
    }
    assert!(
        remaining.len() >= 20,
        "the scan found only {} unmigrated C++ site(s), which is far below the \
         29 measured on 2026-08-28 and reads as a scan that stopped working \
         rather than a migration that made progress:\n  {}",
        remaining.len(),
        remaining.join("\n  ")
    );
    // Every entry names its template, so a reader can go straight to it.
    for site in &remaining {
        assert!(
            site.contains(".jinja2:"),
            "site entry '{site}' does not name a template"
        );
    }
}

/// Render one interpolation through the pair filter under `target`.
fn render_through_pair_filter(target: ScriptEngineTarget, template: &str) -> String {
    let mut env = minijinja::Environment::new();
    let scope = std::sync::Arc::new(sce_build::ecmascript::DocumentScope::installed());
    sce_build::filters::register_cpp_filters_for_engine(&mut env, &scope, target);
    env.add_template("t", template).expect("template compiles");
    env.get_template("t")
        .expect("template is registered")
        .render(minijinja::context! {})
        .expect("render succeeds")
}

/// One filter emits BOTH halves, or neither — the site never assembles them.
///
/// Under the Lua target the two halves differ, and differ in the way the seam
/// exists to preserve: `arr[0]` evaluates as the lowered 1-based index while
/// the diagnostic still names what the author wrote. A filter that emitted one
/// string would have to pick which of those two to lose.
#[test]
fn the_pair_filter_emits_both_halves_together() {
    let lowered = render_through_pair_filter(
        ScriptEngineTarget::Lua,
        r#"{{ "arr[0]" | to_script_source_expr }}"#,
    );
    assert!(
        lowered.starts_with("::SCE::ScriptSource::lua("),
        "the Lua target did not emit the two-argument form: {lowered}"
    );
    assert!(
        lowered.ends_with(r#""arr[0]")"#),
        "the second argument is not the author's text: {lowered}"
    );
    assert!(
        !lowered.contains(r#"lua("arr[0]", "arr[0]")"#),
        "the evaluated half was never lowered — both arguments are the author's \
         text, which is the shape a one-string seam would produce: {lowered}"
    );

    // The other target is the author's text, said once. Emitting the
    // two-argument form here would claim a lowering that did not happen.
    let source = render_through_pair_filter(
        ScriptEngineTarget::EcmaScript,
        r#"{{ "arr[0]" | to_script_source_expr }}"#,
    );
    assert_eq!(source, r#"::SCE::ScriptSource::ecmascript("arr[0]")"#);
}

/// A guard takes the truthiness lowering an expression does not.
///
/// §scxml-5.9 truthiness is where the runtime rewriters' ECMA-262 divergences
/// concentrate, so the guard spelling is the one that must not quietly become
/// the expression spelling.
#[test]
fn the_guard_filter_lowers_a_guard_not_a_bare_expression() {
    let guard = render_through_pair_filter(
        ScriptEngineTarget::Lua,
        r#"{{ "Var1" | to_script_source_guard }}"#,
    );
    let expr = render_through_pair_filter(
        ScriptEngineTarget::Lua,
        r#"{{ "Var1" | to_script_source_expr }}"#,
    );
    assert!(guard.starts_with("::SCE::ScriptSource::lua("));
    assert_ne!(
        guard, expr,
        "a guard and an expression lowered identically, so the truthiness \
         wrapper §scxml-5.9 needs is not being applied: {guard}"
    );
    assert!(
        guard.ends_with(r#""Var1")"#),
        "the guard lost the author's text from its second half: {guard}"
    );
}

/// The C++ migration is complete, and "complete" is three facts, not one.
///
/// A gate that asserted only "the Lua target is supported" could be satisfied
/// by a scanner that had stopped finding anything. So it asks all three: no
/// site left behind, no site unjudged, and — the independent one — sites that
/// demonstrably route through the pair filter.
#[test]
fn the_cpp_migration_is_complete_and_the_lua_target_is_offered() {
    let remaining = Language::Cpp.unmigrated_expression_sites();
    let unknown = Language::Cpp.unclassified_expression_sites();
    let migrated = Language::Cpp.migrated_expression_sites();

    assert!(
        remaining.is_empty(),
        "{} site(s) still hand the engine the author's text:\n  {}",
        remaining.len(),
        remaining.join("\n  ")
    );
    assert!(
        unknown.is_empty(),
        "{} site(s) reach a destination nobody has judged. Each needs one \
         decision: add the callee to ENGINE_ENTRY_POINTS and migrate the site, \
         or to INERT_DESTINATIONS with the evidence:\n  {}",
        unknown.len(),
        unknown.join("\n  ")
    );
    assert!(
        migrated.len() >= 15,
        "only {} C++ site(s) route through the pair filter, which reads as a \
         scanner that stopped working rather than a finished migration",
        migrated.len()
    );
    assert!(
        Language::Cpp.supports_script_engine_target(ScriptEngineTarget::Lua),
        "every site is accounted for and the Lua target is still refused"
    );
}

/// `forge/` falls inside C++'s template ownership and outside this question.
///
/// Ownership answers "whose templates are these"; this scan answers "which
/// text reaches a script engine". Forge renders `if ({{ tr.cond }})` as native
/// C++ and names no engine entry point at all, so counting its expressions
/// would hold the refusal shut over text that was never on the wrong side of
/// the seam.
#[test]
fn the_migration_scan_leaves_the_forge_tree_alone() {
    let remaining = Language::Cpp.unmigrated_expression_sites();
    let forge: Vec<&String> = remaining
        .iter()
        .filter(|site| site.starts_with("forge/"))
        .collect();
    assert!(
        forge.is_empty(),
        "the scan counted {} forge template site(s) as script-engine sites; forge \
         expressions are emitted as native code:\n  {:?}",
        forge.len(),
        forge
    );
}

/// Kotlin crossed the seam on 2026-08-29, and the same two questions are asked
/// of it here.
///
/// Not a copy of the C++ case above with a name changed. The two backends
/// reach the seam by different routes — C++ through a `::SCE::ScriptSource`
/// its templates spell, Kotlin through `com.sce.runtime.ScriptSource` its own
/// runtime carries — and a scan that worked for one and not the other is what
/// this file's history is made of. A backend that HAS crossed and still
/// refuses is a refusal nobody can explain; one that has NOT and accepts is
/// the mixed artifact.
///
/// ⚠ What this case can and cannot see. The equality below is a BICONDITIONAL
/// against a value derived from the same two lists, so it holds however the
/// scan answers — it guards the `target == default` shortcut inside
/// `supports_script_engine_target`, and the default assertion above it, and
/// not the scan's reach. The positive claim, which is what a lost scan shape
/// makes red, is
/// `the_kotlin_migration_is_complete_and_the_lua_target_is_offered` below.
#[test]
fn kotlin_offers_the_lua_target_exactly_when_no_site_is_left_behind() {
    assert_eq!(
        Language::Kotlin.default_script_engine_target(),
        ScriptEngineTarget::Lua,
        "Kotlin's default artifact is LOWERED. It answered `ecmascript` until \
         2026-08-30, and the reason it gave was a host fact rather than a \
         template one: its hosts take the engine by injection and the two this \
         tree ships — `W3CTestBase.DEFAULT_ENGINE` and the Spring starter's \
         `scxmlScriptEngine` bean — both named Rhino, which refuses Lua. Those \
         two moved to the Lua engine in the same commit as this line, which is \
         what makes the pairing hold; `scripts/gates/w3c-kotlin.sh` is what \
         keeps them from drifting apart afterwards."
    );

    let remaining = Language::Kotlin.unmigrated_expression_sites();
    let unknown = Language::Kotlin.unclassified_expression_sites();
    assert_eq!(
        Language::Kotlin.supports_script_engine_target(ScriptEngineTarget::Lua),
        remaining.is_empty() && unknown.is_empty(),
        "the Lua target is offered exactly when NO site is left behind and NO \
         site is unadjudicated — {} unmigrated:\n  {}\n{} unclassified:\n  {}",
        remaining.len(),
        remaining.join("\n  "),
        unknown.len(),
        unknown.join("\n  ")
    );

    // The independent witness, for the reason the C++ case gives: "no site is
    // unmigrated" and "the scan read nothing" are the same answer from that
    // scan alone.
    let migrated = Language::Kotlin.migrated_expression_sites();
    assert!(
        !migrated.is_empty(),
        "no Kotlin site is unmigrated AND none routes through the pair filter, \
         which is what a scanner reading nothing looks like — not a finished \
         migration"
    );
}

/// Kotlin's migration is complete, asserted POSITIVELY.
///
/// The twin of `the_cpp_migration_is_complete_and_the_lua_target_is_offered`,
/// and it had to be written because the biconditional above cannot replace it.
/// `supports_script_engine_target(Lua)` is DEFINED as "both scan lists are
/// empty", so an assertion that the two agree holds however the scan answers:
/// see the site and both sides go false, miss it and both go true. That check
/// earns its place guarding the `target == default` shortcut, not the scan.
///
/// Measured 2026-08-30: the mutation case *"the callee sits further away than
/// the window reached"* — which restores an unmigrated Kotlin `<send>` site
/// whose callee sits above its argument, the shape the scan lost once already
/// — was SURVIVED on CI for exactly this reason, while its two C++ siblings
/// were CAUGHT by the positive C++ assertion. A backend with a positive
/// witness was measured; the one without it was not.
#[test]
fn the_kotlin_migration_is_complete_and_the_lua_target_is_offered() {
    let remaining = Language::Kotlin.unmigrated_expression_sites();
    let unknown = Language::Kotlin.unclassified_expression_sites();
    let migrated = Language::Kotlin.migrated_expression_sites();

    assert!(
        remaining.is_empty(),
        "{} Kotlin site(s) still hand the engine the author's text:\n  {}",
        remaining.len(),
        remaining.join("\n  ")
    );
    assert!(
        unknown.is_empty(),
        "{} Kotlin site(s) reach a destination nobody has judged. Each needs \
         one decision: add the callee to ENGINE_ENTRY_POINTS and migrate the \
         site, or to INERT_DESTINATIONS with the evidence:\n  {}",
        unknown.len(),
        unknown.join("\n  ")
    );
    // A floor, not a count: 30 was measured on 2026-08-30 and sites move, so
    // what is written down is the level below which "finished migration" and
    // "scan that reads almost nothing" stop being distinguishable. Same shape
    // and same reason as the C++ floor above it.
    assert!(
        migrated.len() >= 20,
        "only {} Kotlin site(s) route through the pair filter, which reads as \
         a scanner that stopped working rather than a finished migration",
        migrated.len()
    );
    assert!(
        Language::Kotlin.supports_script_engine_target(ScriptEngineTarget::Lua),
        "every Kotlin site is accounted for and the Lua target is still refused"
    );
}

/// The scan reaches a `{% set %}` binding, on both backends that have one.
///
/// A POSITIVE assertion, and it has to be one. Losing the alias walk makes
/// both scan lists SHORTER, so every "no sites remain" case beside it stays
/// green while the refusal they feed starts offering the Lua target to a
/// backend that launders the author's text through a name. That is not
/// hypothetical: measured 2026-08-29, `entry_exit_actions.jinja2` bound
/// `<donedata>`'s content to a name and emitted it into
/// `DoneDataHelper::evaluateContent`, and the artifact carried
/// `ScriptSource::lua(…)` on one line and the author's ECMAScript on another
/// while this file reported the migration finished.
///
/// Derived rather than listed: what it asserts is "at least one migrated site
/// is a binding", so moving which binding that is does not touch this file.
#[test]
fn the_scan_reaches_expressions_bound_by_a_set_statement() {
    for lang in [Language::Cpp, Language::Kotlin] {
        let bindings: Vec<String> = lang
            .migrated_expression_sites()
            .into_iter()
            .filter(|site| site.contains("{% set "))
            .collect();
        assert!(
            !bindings.is_empty(),
            "{}'s migrated sites include no `{{% set %}}` binding. Both backends \
             bind the author's text to a name and emit it somewhere else — C++ \
             for `<donedata>`'s content, Kotlin for a `<send>`'s — so an empty \
             answer here means the scan stopped following the alias, and every \
             site it stops seeing drops out of the refusal it feeds.",
            lang.canonical_name()
        );
    }
}

/// Every backend is classified by its ARMS, and the classification is total.
///
/// The default this file pins for each backend is now one of three answers,
/// and which one it is depends on how many languages the templates can emit:
///
/// * lowers unconditionally → Lua, because nothing else is emittable;
/// * cannot be asked for Lua → ECMAScript, for the same reason;
/// * both → a POLICY, made in `Language::two_armed_default`.
///
/// ⚠ The vacuity check is the point of the two counters. A predicate that
/// silently answered "one-armed" for everything would leave the policy arm
/// unreached and this case green — the shape
/// `an_oracle_that_restates_its_predicates_definition_is_vacuous` names. So
/// BOTH populations are asserted non-empty: today four backends are one-armed
/// and two carry both arms, and a reading that collapses them is red here
/// before it is anything else.
#[test]
fn every_backends_default_is_the_one_its_arms_allow() {
    let mut one_armed = 0usize;
    let mut two_armed = 0usize;

    for language in Language::ALL.iter().copied() {
        let lowers = language.lowers_expressions_at_build_time();
        let can_lower = language.can_lower_expressions_on_request();
        let default = language.default_script_engine_target();

        match (lowers, can_lower) {
            (true, _) => {
                one_armed += 1;
                assert_eq!(
                    default,
                    ScriptEngineTarget::Lua,
                    "{} lowers unconditionally, so Lua is the only artifact it can \
                     produce and the default cannot be anything else",
                    language.canonical_name()
                );
                assert!(
                    !language.supports_script_engine_target(ScriptEngineTarget::EcmaScript),
                    "{} lowers unconditionally and still claims a source-emitting arm",
                    language.canonical_name()
                );
            }
            (false, false) => {
                one_armed += 1;
                assert_eq!(
                    default,
                    ScriptEngineTarget::EcmaScript,
                    "{} has no lowered arm, so the author's text is the only artifact \
                     it can produce",
                    language.canonical_name()
                );
            }
            (false, true) => {
                two_armed += 1;
                assert!(
                    ScriptEngineTarget::ALL.contains(&default),
                    "{} carries both arms and its policy answered a target that is not \
                     on the wire",
                    language.canonical_name()
                );
                // Reaching here already proves `two_armed_default` did not take
                // its `unreachable!` arm for this backend — that arm panics.
                assert!(
                    language.supports_script_engine_target(ScriptEngineTarget::Lua)
                        && language.supports_script_engine_target(ScriptEngineTarget::EcmaScript),
                    "{} was classified as two-armed and refuses one of the two",
                    language.canonical_name()
                );
            }
        }
    }

    assert!(
        one_armed > 0 && two_armed > 0,
        "the arm classification collapsed: {one_armed} one-armed and {two_armed} \
         two-armed backend(s). Both populations are non-empty in this tree, so a \
         zero on either side means the predicates stopped telling them apart and \
         every assertion above is being made about one bucket."
    );
}

/// The two-armed policy has no catch-all arm.
///
/// ⚠ This is the escape hatch that would defeat everything above. A `_ =>` in
/// `two_armed_default` would answer for a backend that LATER grows a second
/// arm — turning a forced answer into an unexamined policy with nothing red —
/// and no behavioural test can see it, because on today's tree the catch-all
/// and the enumeration return the same values for the same backends. The only
/// witness is the source, so the source is what this reads.
///
/// Read as CODE, not as text: the rationale above `two_armed_default` talks
/// about `_` arms, and a scan that matched its own documentation is a mistake
/// this seam has already made once (`reach_of`, 2026-08-30).
#[test]
fn the_two_armed_policy_names_every_backend() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generator.rs"),
    )
    .expect("the generator source is readable from its own crate");

    let start = source
        .find("fn two_armed_default(self) -> ScriptEngineTarget {")
        .expect("`two_armed_default` is where the two-armed policy is made");
    let body = &source[start..];
    let end = body
        .find("\n    }\n")
        .expect("the function is brace-balanced");
    let body: String = body[..end]
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !body.contains("_ =>"),
        "`two_armed_default` grew a catch-all arm. Every backend must be named \
         there: a backend that falls through gets another backend's policy \
         without anyone choosing it, and nothing else in this file can tell.\n{body}"
    );
    for language in Language::ALL.iter().copied() {
        assert!(
            body.contains(&format!("Language::{language:?}")),
            "`two_armed_default` does not name `Language::{language:?}`. Every \
             backend is named — the one-armed ones by an `unreachable!` that says \
             what growing a second arm would mean — so that adding a backend, or \
             moving one across the seam, is a compile error rather than a silent \
             default.\n{body}"
        );
    }
}
