// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Multi-language code generator — renders minijinja templates from SCXMLModel.
// Supports Rust, C++, and Kotlin code generation.

use crate::ecmascript::DocumentScope;
use crate::filters;
use crate::forge::error::GenerateError;
use crate::forge::symbol_mangling;
use crate::model::SCXMLModel;
use minijinja::Environment;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The declarations the ECMAScript filters resolve authored names
/// against, for one document.
///
/// Built here, at the point where a backend's template environment is
/// assembled, because that is the one place every backend passes
/// through — the four that lower the datamodel to Lua share the scope
/// with [`crate::ecmascript_acceptance`], and a `check` that disagreed
/// with a `generate` about which names exist would put two answers on
/// one document.
fn document_scope(model: &SCXMLModel) -> Arc<DocumentScope> {
    Arc::new(DocumentScope::from_model(model))
}

/// Create a minijinja Environment with Python Jinja2 compatibility enabled.
pub(crate) fn new_env<'a>() -> Environment<'a> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    // Python Jinja2 compatibility:
    // 1. dict.items(), str.strip(), str.startswith(), etc.
    env.set_unknown_method_callback(minijinja_contrib::pycompat::unknown_method_callback);
    // 2. Undefined attributes propagate as undefined (Chainable) instead of
    //    silently becoming "" (Lenient). This catches template typos while still
    //    allowing optional attribute chains like `model.foo.bar` to work.
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
    register_symbol_artifact_global(&mut env);
    env
}

/// Publish the artifact vocabulary the SCE-MAP markers name.
///
/// Registered as a template global rather than pushed into each backend's
/// render context so all six backends, the forge per-kind templates, and the
/// mesh transport template (which builds its own `Environment`) read one copy
/// — and so no template ever spells an artifact label as a literal, which
/// would fork the vocabulary the sourcemap keys off.
pub(crate) fn register_symbol_artifact_global(env: &mut Environment<'_>) {
    env.add_global(
        "sce_artifact",
        minijinja::Value::from_iter([
            ("machine", symbol_mangling::ARTIFACT_MACHINE),
            ("state_body", symbol_mangling::ARTIFACT_STATE_BODY),
            ("forge_body", symbol_mangling::ARTIFACT_FORGE_BODY),
        ]),
    );
}

/// Which language the generated artifact hands its script engine.
///
/// The seam's codegen input. `datamodel="ecmascript"` is a claim about a
/// language and Lua is not that language, so a backend running the datamodel
/// on Lua has to translate — and there are exactly two places to do it,
/// build time or run time (`docs/SCE_LUA_TRANSLATION_SEAM.md`). This says
/// which one THIS run chose, and the choice is a codegen input rather than a
/// property of the backend because C++ and Kotlin genuinely have both.
///
/// The spellings are the manifest's `script_engine_language` vocabulary, so
/// the value a host reads and the value the run selected cannot become two
/// names for one answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptEngineTarget {
    /// Emit the author's ECMAScript and let the engine deal with it —
    /// natively, or through an input adapter it owns.
    EcmaScript,
    /// Emit Lua the build-time frontend produced. The artifact can then only
    /// run on a Lua engine, which is the whole cost of this choice.
    Lua,
}

impl ScriptEngineTarget {
    /// The wire spelling, shared with `script_engine_language`.
    pub fn wire_name(self) -> &'static str {
        match self {
            ScriptEngineTarget::EcmaScript => crate::manifest::SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT,
            ScriptEngineTarget::Lua => crate::manifest::SCRIPT_ENGINE_LANGUAGE_LUA,
        }
    }

    /// Parse the flag value. Accepts exactly the wire vocabulary, so a
    /// caller cannot spell the selection one way and read it back another.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            crate::manifest::SCRIPT_ENGINE_LANGUAGE_ECMASCRIPT => {
                Some(ScriptEngineTarget::EcmaScript)
            }
            crate::manifest::SCRIPT_ENGINE_LANGUAGE_LUA => Some(ScriptEngineTarget::Lua),
            _ => None,
        }
    }
}

/// Target language for code generation.
///
/// `C11` is the embedded MCU backend per SCE Protocol-Synthesis RFC §synth-5-J-1.
/// Enum membership lets every dispatch site handle the C11 case
/// explicitly rather than silently routing C11 through a more
/// permissive arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Cpp,
    Kotlin,
    Go,
    Python,
    C11,
}

impl Language {
    /// Every backend, in the canonical order used wherever a run spans
    /// all of them (the `check` sweep, the codegen-matrix coverage
    /// tests, the manifest schema's language enum).
    ///
    /// A seventh backend lands here once and every all-backends site
    /// widens with it. The alternative — each site restating the six —
    /// is how a new backend ends up covered by some of them and
    /// silently skipped by the rest.
    pub const ALL: &'static [Language] = &[
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
    ];

    /// Subdirectory of `tools/codegen/templates/` holding this
    /// language's templates, or `""` when they live at the tree root.
    ///
    /// Single source of truth for template scoping: the filesystem
    /// loader joins it onto the tree root, and
    /// [`crate::template_registry::embedded_templates_for`] strips it from the
    /// embedded registry's names. Both derive the same scope from this
    /// one match, so a new backend cannot be reachable from one source
    /// and not the other.
    ///
    /// C++ and C11 return the root: every backend shares
    /// `license_header.jinja2` there, while the C11 statechart templates
    /// live under `c/`, so both layers have to be visible in one pass.
    pub fn template_subdir(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Cpp => "",
            Language::Kotlin => "kotlin",
            Language::Go => "go",
            Language::Python => "python",
            Language::C11 => "",
        }
    }

    /// Subdirectory this language's own templates live in, as opposed to
    /// the scope it loads.
    ///
    /// The two differ, and conflating them is a live defect rather than
    /// a hypothetical: [`Self::template_subdir`] is a *loader scope*, so
    /// C11 returns the root to reach the shared `license_header.jinja2`
    /// even though its statechart templates are under `c/`. Anything
    /// asking "whose templates are these?" — dependency filtering,
    /// registry ownership — needs this instead, and asking the loader
    /// scope leaves `c/` owned by nobody, so every language claims it.
    ///
    /// `None` means the language's templates sit at the tree root, which
    /// only C++ does. Root-level templates are consequently not
    /// attributable to one backend by path alone; separating them is a
    /// larger change than this axis, and until it happens a C11 build
    /// still lists the root C++ templates among its inputs.
    pub fn template_owned_subdir(self) -> Option<&'static str> {
        match self {
            Language::Rust => Some("rust"),
            Language::Cpp => None,
            Language::Kotlin => Some("kotlin"),
            Language::Go => Some("go"),
            Language::Python => Some("python"),
            Language::C11 => Some("c"),
        }
    }

    /// Directory under `templates/forge/` holding this language's forge
    /// templates — the scope `forge::generator::generate_*` loads.
    ///
    /// Separate from [`Self::template_subdir`] because the forge tree
    /// names every backend, including C++, which has no directory of its
    /// own in the statechart tree. `C11` is `c` there as it is here.
    ///
    /// The `generate_*` functions still spell their own path inline,
    /// each being single-language. That duplication is bounded by
    /// `codegen_depfile_content`, and measured rather than assumed to
    /// be: renaming `forge/kotlin` to `forge/kt` and pointing only the
    /// loader at it still renders — the gate turns red anyway, because
    /// the scope named here no longer resolves and the depfile comes out
    /// empty. A divergence that renders is exactly the silent kind.
    pub fn forge_template_subdir(self) -> &'static str {
        self.feature_tree_subdir()
    }

    /// The per-language directory name a *feature* template tree uses.
    ///
    /// `templates/forge/` and `templates/mesh/` both address backends by
    /// this name, and they have to: a feature arm exists for a language
    /// exactly when `<tree>/<name>/` does, so a second spelling would
    /// make one tree's coverage unreadable from the other's shape.
    /// Distinct from [`Self::template_subdir`], which is a *loader
    /// scope* and answers the root for C++.
    fn feature_tree_subdir(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Cpp => "cpp",
            Language::Kotlin => "kotlin",
            Language::Go => "go",
            Language::Python => "python",
            Language::C11 => "c",
        }
    }

    /// Path prefixes belonging to *other* backends, relative to the
    /// template tree root.
    ///
    /// Derived from the language registry rather than written out, so a
    /// new backend cannot be forgotten here. It was: the depfile writer
    /// carried a hand-kept `rust`/`kotlin`/`go` list, and when `python/`
    /// and `c/` were added nothing updated it, so a C++ build declared a
    /// dependency on 18 templates it cannot render and a C11 build on
    /// 65 — meaning a Rust template edit regenerated every C11 output.
    pub fn foreign_template_prefixes(self) -> Vec<&'static str> {
        crate::template_registry::SUPPORTED_LANGUAGES
            .iter()
            .filter(|other| **other != self)
            .filter_map(|other| other.template_owned_subdir())
            .filter(|subdir| Some(*subdir) != self.template_owned_subdir())
            .collect()
    }

    /// The identifier [`FromStr`](std::str::FromStr) accepts for this
    /// language, and the one a caller should present.
    ///
    /// `FromStr` also accepts aliases (`c++`, `kt`, `golang`); this is
    /// the spelling to round-trip through, so a caller enumerating
    /// languages emits something the parser is guaranteed to take back.
    pub fn canonical_name(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Cpp => "cpp",
            Language::Kotlin => "kotlin",
            Language::Go => "go",
            Language::Python => "python",
            Language::C11 => "c11",
        }
    }

    /// Which language the script engine this backend's output needs must
    /// evaluate — the value the manifest reports to a host.
    ///
    /// It is NOT the same on every backend, and the manifest said it was.
    /// Four backends receive Lua that `sce-build`'s ECMAScript frontend
    /// produced at generation time (`to_lua_guard` / `to_lua_expr` in the
    /// templates). Two — C++ and Kotlin — receive the author's ECMAScript
    /// *source* and hand it to the engine as it stands, because their
    /// generated code takes its engine by injection and does not know at
    /// generation time which one arrives. Their defaults are ECMAScript
    /// engines: `SCE_SCRIPT_ENGINE=quickjs` (`sce/CMakeLists.txt`) and
    /// `W3CTestBase.DEFAULT_ENGINE = "rhino"`.
    ///
    /// A host obeying a manifest that said `lua` for those two would supply
    /// a Lua engine for a machine that hands it ECMAScript — the exact
    /// mis-supply the field exists to prevent.
    ///
    /// `docs/SCE_LUA_TRANSLATION_SEAM.md` carries the per-backend table, and
    /// `sce-build/tests/script_engine_language_parity.rs` holds this answer to
    /// what the emitted artifact actually carries.
    ///
    /// **Derived, not listed.** This used to be a `match` naming four
    /// backends on one side and two on the other, which is a second roster
    /// beside the templates and free to disagree with them — the failure this
    /// field already had once, as a hard-coded `"lua"`. It now reads
    /// [`Self::lowers_expressions_at_build_time`], so moving a backend across
    /// the seam is a template edit and nothing else. Same shape as the
    /// mesh-rpc refusal, which reads `templates/mesh/<lang>/` rather than
    /// asserting which backends have a mesh arm.
    pub fn script_engine_language(self) -> &'static str {
        self.default_script_engine_target().wire_name()
    }

    /// The engine language this backend emits for when nobody asks for
    /// another — today's derived answer, unchanged.
    ///
    /// A *default*, not a fact, since `--script-engine` exists: the run's
    /// selection is what the manifest reports and what the templates render
    /// against. Keeping the default derived means a backend that gains a
    /// lowering arm moves its own default with it.
    pub fn default_script_engine_target(self) -> ScriptEngineTarget {
        if self.lowers_expressions_at_build_time() {
            ScriptEngineTarget::Lua
        } else {
            ScriptEngineTarget::EcmaScript
        }
    }

    /// Whether this backend's templates lower expressions through the
    /// build-time ECMAScript frontend, rather than emitting the author's
    /// source for an engine to deal with at run time.
    ///
    /// Read off the embedded template tree — the same registry the renderer
    /// uses — so the answer cannot drift from what is actually rendered.
    /// Ownership comes from [`Self::template_owned_subdir`], which is the
    /// existing single source of truth for "whose templates are these": five
    /// backends own a subdirectory, and C++ owns everything no other backend
    /// claims (the tree root, `actions/`, `mesh/`, `forge/`, `_macros/`).
    ///
    /// The marker is the GUARD filter specifically. `to_lua_guard` is what
    /// wraps a condition in the frontend's truthiness helper, and truthiness
    /// is where the rewriters' divergences from ECMA-262 concentrate, so it
    /// is the filter whose presence decides which engine a host needs.
    ///
    /// ⚠ Jinja comments are stripped before the search. A template is free to
    /// *mention* `to_lua_guard` in a `{# … #}` block while emitting source —
    /// and a scanner that reads comments as code cannot tell a backend's
    /// behaviour from its documentation. That is not hypothetical here: the
    /// C++ tree's only mention of these filters is a comment
    /// (`invoke_methods.jinja2`), and the artifact-side gate for this field
    /// was fooled by exactly this shape on its first run.
    pub fn lowers_expressions_at_build_time(self) -> bool {
        const GUARD_LOWERING_FILTER: &str = "to_lua_guard";

        crate::template_registry::EMBEDDED_TEMPLATES
            .iter()
            .filter(|(name, _)| self.owns_template(name))
            .any(|(_, body)| strip_jinja_comments(body).contains(GUARD_LOWERING_FILTER))
    }

    /// Whether this backend can emit an artifact for @p target.
    ///
    /// **Derived, like every other answer on this seam.** A backend always
    /// supports its own default. The other direction is supported only when
    /// its templates can actually produce it — which for the Lua target means
    /// EVERY site that hands the engine the author's text goes through the
    /// pair filter, not merely that some site does.
    ///
    /// That "every" is the load-bearing word, and it is why this is a refusal
    /// rather than a lint. `docs/SCE_LUA_TRANSLATION_SEAM.md` states the
    /// hazard: *"Splitting a subset is not a smaller version of this change:
    /// the engine would receive Lua from some sites and ECMAScript from
    /// others, in one session"* — and the mixed artifact would carry no
    /// diagnostic saying so. A half-migrated backend must therefore refuse the
    /// flag, and the refusal lifts by itself when the last site moves, the way
    /// the mesh-rpc refusal lifts when `templates/mesh/<lang>/` appears.
    pub fn supports_script_engine_target(self, target: ScriptEngineTarget) -> bool {
        if target == self.default_script_engine_target() {
            return true;
        }
        match target {
            // BOTH lists, and the second is not a formality. `unmigrated` is
            // the population this file knows how to classify; `unclassified`
            // is the population it does NOT — a site mentioning a model
            // expression whose callee is absent from `ENGINE_ENTRY_POINTS`.
            // Opening the Lua target on the first alone would let the escape
            // hatch defeat the gate: measured 2026-08-28, `unmigrated` reached
            // 0 while 9 sites were still unadjudicated, and any one of them
            // that turns out to evaluate would make exactly the mixed artifact
            // this refusal exists to prevent.
            ScriptEngineTarget::Lua => {
                self.unmigrated_expression_sites().is_empty()
                    && self.unclassified_expression_sites().is_empty()
            }
            // Nothing walks the other way yet: a backend that lowers has no
            // arm that emits the author's source, and inventing one would be
            // a second emission path for the four backends already correct.
            ScriptEngineTarget::EcmaScript => false,
        }
    }

    /// Template interpolations that still hand the engine the author's text
    /// without routing it through the `(lowered, source)` pair filter.
    ///
    /// Counted by MODEL FIELD rather than by call shape, which is the lesson
    /// the 38-site count paid for twice. A filter-keyed scan misses a site
    /// that passes `{{ action.array }}` with no filter at all, and an
    /// entry-point-keyed scan misses a helper nobody thought to list; but the
    /// author's text can only reach a template through one of these fields, so
    /// asking which interpolations mention one catches both shapes.
    ///
    /// Each entry is `"<template>: <interpolation>"`, so a refusal can name
    /// what is left rather than only how much.
    pub fn unmigrated_expression_sites(self) -> Vec<String> {
        /// Fields whose value is text some engine will evaluate. Extending
        /// the model with another one belongs here in the same commit.
        const MODEL_EXPRESSION_FIELDS: &[&str] = &[
            ".cond",
            ".expr",
            ".array",
            ".typeexpr",
            ".targetexpr",
            ".sendidexpr",
            ".delayexpr",
            ".contentexpr",
            ".srcexpr",
            ".eventexpr",
            ".location",
            // ⚠ `<assign>` / `<data>` inline content. Added 2026-08-28 after
            // adjudicating the sites this list had produced: the content arm
            // of `actions/assign.jinja2` renders through
            // `to_author_assign_content` and hands the result to
            // `executeAssignment`, which is every bit the hand-off the `expr`
            // arm beside it is — and this list had no field that matched it,
            // so the scan reported the arm as absent rather than as done.
            ".content",
        ];
        /// What a migrated site routes through. One prefix covers the guard
        /// and expression spellings both.
        const PAIR_FILTER_PREFIX: &str = "to_script_source";

        let mut sites = Vec::new();
        for (name, body) in crate::template_registry::EMBEDDED_TEMPLATES.iter() {
            if !self.owns_template(name) {
                continue;
            }
            // ⚠ Ownership is not the same question as this one. C++ owns
            // everything no other backend claims, and that complement
            // includes `forge/` — which has its own per-language directories
            // and whose `expr` / `cond` are FORGE AST fields, emitted as
            // native code. Measured 2026-08-28:
            // `forge/cpp/procedure.h.jinja2` renders `if ({{ tr.cond }})`
            // directly and mentions no engine entry point at all. Counting
            // those as script sites would hold the seam's refusal shut over
            // expressions that never reach an engine.
            if name.starts_with("forge/") {
                continue;
            }
            // Line by line, because whether a site reaches the engine is a
            // property of the LINE it lands on: every backend echoes the
            // author's expression into a comment beside the code that uses
            // it, and a scan that reads comments as code cannot tell output
            // from documentation — the mistake this seam's first gate made.
            let stripped = strip_jinja_comments(body);
            let lines: Vec<&str> = stripped.lines().collect();
            for (index, line) in lines.iter().copied().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
                    continue;
                }
                for interpolation in jinja_interpolations(line) {
                    if !mentions_model_expression(interpolation, MODEL_EXPRESSION_FIELDS) {
                        continue;
                    }
                    // A site that already carries the pair filter is migrated.
                    if interpolation.contains(PAIR_FILTER_PREFIX) {
                        continue;
                    }
                    match classify_expression_site(&lines, index, interpolation) {
                        ExpressionSiteRole::EngineBound => {
                            sites.push(format!("{name}: {{{{ {} }}}}", interpolation.trim()))
                        }
                        // A message and a validation are not hand-offs, and
                        // counting them would hold the refusal shut forever
                        // over text no engine ever sees.
                        ExpressionSiteRole::Message | ExpressionSiteRole::NotAHandOff => {}
                    }
                }
            }
        }
        sites.sort();
        sites.dedup();
        sites
    }

    /// Sites that mention a model expression, are not prose, and reach a
    /// callee this file does not know.
    ///
    /// ⚠ The escape hatch's escape hatch. `classify_expression_site` reads an
    /// unknown callee as [`ExpressionSiteRole::NotAHandOff`] — the same answer
    /// it gives `validateForeachAttributes`, which is correct there and would
    /// be a silent hole anywhere else. A helper nobody added to
    /// `ENGINE_ENTRY_POINTS` would drop out of the migration population
    /// without a word, which is exactly how the seam document's own table
    /// came to miss five helpers.
    ///
    /// So the unknown callees are listed rather than assumed benign. Each one
    /// is a decision someone has to make once: it evaluates (add it to
    /// `ENGINE_ENTRY_POINTS`) or it does not (say so where it is used).
    pub fn unclassified_expression_sites(self) -> Vec<String> {
        const MODEL_EXPRESSION_FIELDS: &[&str] = &[
            ".cond",
            ".expr",
            ".array",
            ".typeexpr",
            ".targetexpr",
            ".sendidexpr",
            ".delayexpr",
            ".contentexpr",
            ".srcexpr",
            ".eventexpr",
            ".location",
            // ⚠ `<assign>` / `<data>` inline content. Added 2026-08-28 after
            // adjudicating the sites this list had produced: the content arm
            // of `actions/assign.jinja2` renders through
            // `to_author_assign_content` and hands the result to
            // `executeAssignment`, which is every bit the hand-off the `expr`
            // arm beside it is — and this list had no field that matched it,
            // so the scan reported the arm as absent rather than as done.
            ".content",
        ];

        let mut sites = Vec::new();
        for (name, body) in crate::template_registry::EMBEDDED_TEMPLATES.iter() {
            if !self.owns_template(name) || name.starts_with("forge/") {
                continue;
            }
            let stripped = strip_jinja_comments(body);
            let lines: Vec<&str> = stripped.lines().collect();
            for (index, line) in lines.iter().copied().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
                    continue;
                }
                for interpolation in jinja_interpolations(line) {
                    if !mentions_model_expression(interpolation, MODEL_EXPRESSION_FIELDS)
                        || interpolation.contains("to_script_source")
                    {
                        continue;
                    }
                    if classify_expression_site(&lines, index, interpolation)
                        == ExpressionSiteRole::NotAHandOff
                    {
                        sites.push(format!("{name}: {{{{ {} }}}}", interpolation.trim()));
                    }
                }
            }
        }
        sites.sort();
        sites.dedup();
        sites
    }

    /// Template interpolations that DO route through the pair filter.
    ///
    /// The independent witness for [`Self::unmigrated_expression_sites`].
    /// "No sites remain" and "the scan found nothing" are the same answer
    /// from that function alone, and a gate that asked it twice would report
    /// a broken scanner as a finished migration — offering the Lua target to
    /// a backend that never moved a line. This counts the other side, so the
    /// two can only agree when the scanner is working.
    pub fn migrated_expression_sites(self) -> Vec<String> {
        const PAIR_FILTER_PREFIX: &str = "to_script_source";

        let mut sites = Vec::new();
        for (name, body) in crate::template_registry::EMBEDDED_TEMPLATES.iter() {
            if !self.owns_template(name) || name.starts_with("forge/") {
                continue;
            }
            for interpolation in jinja_interpolations(&strip_jinja_comments(body)) {
                if interpolation.contains(PAIR_FILTER_PREFIX) {
                    sites.push(format!("{name}: {{{{ {} }}}}", interpolation.trim()));
                }
            }
        }
        sites.sort();
        sites.dedup();
        sites
    }

    /// Whether `name` (a template path relative to the tree root) belongs to
    /// this backend. See [`Self::lowers_expressions_at_build_time`] for why
    /// C++ is the complement rather than a subdirectory.
    fn owns_template(self, name: &str) -> bool {
        match self.template_owned_subdir() {
            Some(dir) => name.starts_with(&format!("{dir}/")),
            None => crate::template_registry::SUPPORTED_LANGUAGES
                .iter()
                .filter_map(|other| other.template_owned_subdir())
                .all(|dir| !name.starts_with(&format!("{dir}/"))),
        }
    }
}

impl std::str::FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rust" => Ok(Language::Rust),
            "cpp" | "c++" => Ok(Language::Cpp),
            "kotlin" | "kt" => Ok(Language::Kotlin),
            "go" | "golang" => Ok(Language::Go),
            "python" | "py" => Ok(Language::Python),
            "c11" | "c" => Ok(Language::C11),
            _ => Err(format!("Unknown language: {s}")),
        }
    }
}

/// Generated output — may contain multiple files (e.g., C++ .h + .inl)
/// plus the canonical paths of every external file the compile consumed
/// (`<xi:include>` targets, `<sce:use>` template fragments,
/// `<sce:import>` forge documents).
///
/// `deps` is the single source of truth for build-system rerun
/// invalidation. Consumers that drive Cargo (`compile_scxml`) or write
/// Make-style depfiles (`sce-codegen --write-deps`) must emit each
/// path so a fragment edit re-fires codegen even if the host SCXML
/// did not change — without this, fragment edits become silent
/// no-ops and downstream artifacts diverge from author source until
/// a clean rebuild (tc8-harness hazard class). `from_string` entry
/// points populate this with `Vec::new()` because they have no
/// filesystem dependencies.
///
/// "Single source of truth" is load-bearing, not decorative: a consumer
/// that reconstructs the list instead of reading this field gets a
/// different answer. `sce-codegen` rebuilt the forge half from
/// `parsed.imports` and so named the *direct* imports only, leaving an
/// `algorithm → codec → codec` chain's leaf undeclared while editing it
/// still changed the compiled document's `source-hash`.
#[derive(Default)]
pub struct GeneratedOutput {
    pub files: Vec<(String, String)>, // (filename, content)
    /// Canonical paths of the external inputs this compile read.
    ///
    /// On the statechart route these are the preprocessor inputs, from
    /// `Parser::preprocessor_deps()`. On the forge route they are the
    /// transitive `<sce:import>` closure, from the walk
    /// `forge::cross_kind_check::check` already performs. Empty for
    /// `from_string` / in-memory routes.
    pub deps: Vec<PathBuf>,
}

/// Return `content` guaranteed to end with a newline.
///
/// POSIX defines a text file as a sequence of lines, each terminated by
/// a newline — a trailing fragment without one is not a line. Consumers
/// enforce it: `clang -Werror -Wnewline-eof` rejects such a header
/// outright, and `-Werror` is a default MCU consumers build with.
///
/// Every template already ends with a newline, but Jinja's
/// `keep_trailing_newline` default strips exactly one from each render.
/// Backends whose pipeline includes a formatter (clang-format for C++,
/// gofmt, rustfmt, ktlint) had it restored downstream and so never
/// showed the defect; C11 has no formatter, so its headers shipped
/// ending in `#endif  /* GUARD */` with no newline at all.
///
/// The rule belongs at the boundary where an artefact becomes a file,
/// not in the template environment. Ending with a newline is a property
/// of a file, not of a render: enabling `keep_trailing_newline` would
/// equally preserve the newline of every included partial and smear
/// blank lines through the middle of the output. Applied at the write
/// boundary it holds once for every backend, including ones that have
/// no formatter and templates not yet written.
///
/// Exposed rather than private to the binary because two write paths
/// need it — `sce-codegen`'s writers and the `compile_scxml` build.rs
/// facade — and any downstream consumer driving [`GeneratedOutput`] to
/// its own files needs the same guarantee.
///
/// An empty artefact stays empty: a zero-byte file is a valid text file
/// and no consumer diagnoses it.
pub fn with_trailing_newline(content: &str) -> std::borrow::Cow<'_, str> {
    if content.is_empty() || content.ends_with('\n') {
        std::borrow::Cow::Borrowed(content)
    } else {
        std::borrow::Cow::Owned(format!("{content}\n"))
    }
}

/// License configuration matching Python license_config.py
fn license_config() -> serde_json::Value {
    serde_json::json!({
        "project": {
            "name": "SCE (SCXML Core Engine)",
            "copyright_year": "2025-2026",
            "copyright_holder": "newmassrael"
        },
        "urls": {
            "license_main": "https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE"
        },
        "generated_code_header": {
            "copyright_holder": "[Author of input SCXML file]"
        }
    })
}

/// `{# … #}` blocks removed, everything else untouched.
///
/// Used where a template is READ as evidence of what it emits: a comment is
/// documentation, and counting it as behaviour is how a scan reports a
/// backend doing something it only talks about. Unterminated openers swallow
/// the rest of the body, which is the safe direction — a malformed template
/// contributes no evidence rather than false evidence.
fn strip_jinja_comments(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(open) = rest.find("{#") {
        out.push_str(&rest[..open]);
        match rest[open..].find("#}") {
            Some(close) => rest = &rest[open + close + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// What a template interpolation of a model expression is actually DOING.
///
/// The three roles share one spelling — `{{ param.expr | escape_cpp }}` is
/// the same eleven characters whether it is handed to an engine, printed in a
/// log line, or checked for emptiness — so a scan that reads the spelling
/// alone cannot tell them apart, and one that counted all three could never
/// reach zero. That matters here because the count IS the refusal: if it can
/// never reach zero, `--script-engine lua` can never be offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpressionSiteRole {
    /// Handed across the boundary for an engine to evaluate. The migration's
    /// population.
    EngineBound,
    /// Interpolated into a diagnostic — a `SCE_LOG_*` / `AOT_DEBUG` format
    /// string, or any C++ string literal carrying other text besides the
    /// expression. The author's own text is exactly what belongs there.
    Message,
    /// Reaches a callee that does not evaluate. Measured 2026-08-28:
    /// `ForeachValidator::validateForeachAttributes` takes the array text and
    /// only asks whether it is empty (`ForeachValidator.h:27`), so lowering it
    /// would answer a question nobody asked.
    NotAHandOff,
}

/// Callees that evaluate, or hand on to something that does.
///
/// ⚠ Derived from `sce/include/scripting/` and `sce/include/common/` rather
/// than from a document — the seam document's own table missed five helpers by
/// being a list. A callee absent from here reads as [`ExpressionSiteRole::NotAHandOff`],
/// which is why `unclassified_expression_sites` exists to surface exactly that
/// case instead of letting a missed helper pass as "not a hand-off".
/// Macros that render their arguments into a diagnostic rather than evaluating
/// them. An expression handed to one of these is being *reported*, and the
/// author's own text is exactly what belongs in a report.
const DIAGNOSTIC_MACROS: &[&str] = &["SCE_LOG_", "AOT_DEBUG", "AOT_LOG"];

/// How far back to look for the callee of a multi-line call.
///
/// ⚠ Not zero, and this is a shape this seam has already paid for once:
/// *"A multi-line call. Five sites put the call on one line and the expression
/// on the next … Any line-keyed grep, entry-point or filter, reports 33 rather
/// than 38."* `ForeachHelper::executeForeachWithActions(` sits two lines above
/// its `{{ action.array }}`, so a one-line window reads that array expression
/// as reaching nobody — and a site that reaches nobody is a site the migration
/// never has to move.
const CALL_WINDOW_LINES: usize = 3;

const ENGINE_ENTRY_POINTS: &[&str] = &[
    "evaluateExpression",
    "executeScript",
    "validateExpression",
    "safeEvaluateGuard",
    "evaluateGuard",
    "initializeVariableFromExpr",
    "executeForeachWithActions",
    "executeForeachWithoutBody",
    "setLoopVariableFromExpr",
    "evaluateForeachArray",
    "resultToString",
    "resultToStringArray",
    "resultToScriptValueArray",
    "executeAssignment",
    "evaluateContent",
    "evaluateParams",
    "executeFinalizeWithEvent",
];

/// Which of the three roles this interpolation plays on `line`.
fn classify_expression_site(
    lines: &[&str],
    index: usize,
    interpolation: &str,
) -> ExpressionSiteRole {
    let line = lines[index];
    // A string literal carrying anything besides the interpolation is prose
    // with a value in it — a log line, an error message, a comment rendered
    // into the artifact. Checked FIRST because a message can sit on the very
    // same line as a hand-off.
    if interpolation_is_inside_prose(line, interpolation) {
        return ExpressionSiteRole::Message;
    }

    // The window: this line and the few above it, which is where a multi-line
    // call keeps its callee. An engine entry point anywhere in reach wins,
    // because a call that both logs and evaluates is still a hand-off.
    let start = index.saturating_sub(CALL_WINDOW_LINES);
    if lines[start..=index]
        .iter()
        .any(|l| ENGINE_ENTRY_POINTS.iter().any(|entry| l.contains(entry)))
    {
        return ExpressionSiteRole::EngineBound;
    }

    // A diagnostic macro with no engine entry point in reach is a report.
    // This catches the argument form the prose check cannot see:
    // `SCE_LOG_ERROR("… {}: {}", "name", "{{ expr }}")` hands the expression
    // as its OWN literal, so nothing shares the quotes with it.
    if line.contains('(')
        && DIAGNOSTIC_MACROS
            .iter()
            .any(|macro_name| line.contains(macro_name))
    {
        return ExpressionSiteRole::Message;
    }
    ExpressionSiteRole::NotAHandOff
}

/// Whether the interpolation sits in a C++ string literal that also carries
/// other text.
///
/// `"{{ x }}"` is a value being passed. `"failed: {{ x }}"` is a sentence with
/// a value in it. The difference is whether anything else survives inside the
/// quotes once the interpolation is removed.
fn interpolation_is_inside_prose(line: &str, interpolation: &str) -> bool {
    let needle = format!("{{{{{interpolation}}}}}");
    let Some(at) = line.find(&needle) else {
        return false;
    };
    // Count quotes before the interpolation: an odd number means it opened a
    // literal that has not closed, so the interpolation is inside one.
    let before = &line[..at];
    if before.matches('"').count() % 2 == 0 {
        return false;
    }
    let open = before.rfind('"').map(|i| i + 1).unwrap_or(0);
    let after_start = at + needle.len();
    let close = line[after_start..]
        .find('"')
        .map(|i| after_start + i)
        .unwrap_or(line.len());
    let literal = &line[open..close];
    // Whatever the literal holds besides this interpolation. Jinja control
    // blocks do not count — `{% if %}` around a value is still one value.
    let remainder: String = literal.replace(&needle, "");
    let remainder = strip_jinja_statements(&remainder);
    !remainder.trim().is_empty()
}

/// Remove `{% … %}` blocks, which are structure rather than emitted text.
fn strip_jinja_statements(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(open) = rest.find("{%") {
        out.push_str(&rest[..open]);
        match rest[open..].find("%}") {
            Some(close) => rest = &rest[open + close + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Whether an interpolation reads one of the model's expression-bearing
/// fields, matched at a word boundary.
///
/// The boundary is not decoration: `trans.cond_cpp` is the NATIVELY LOWERED
/// guard — a C++ expression the script engine never sees — and a substring
/// test for `.cond` reports it as an unmigrated script site. Counting a
/// natively lowered guard as engine-bound would keep the seam's refusal
/// standing forever over sites that were never on the wrong side of it.
fn mentions_model_expression(interpolation: &str, fields: &[&str]) -> bool {
    fields.iter().any(|field| {
        let mut rest = interpolation;
        while let Some(at) = rest.find(field) {
            let after = &rest[at + field.len()..];
            let boundary = after
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_');
            if boundary {
                return true;
            }
            rest = after;
        }
        false
    })
}

/// Every `{{ … }}` interpolation in a template body, without the braces.
///
/// Deliberately not a regex: an interpolation can hold a quoted string with
/// braces in it, and the migration scan above must not miss a site because
/// its filter chain happened to be spelled unusually. Scanning for the
/// delimiters answers the same question with no pattern to get wrong.
fn jinja_interpolations(body: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        match after.find("}}") {
            Some(close) => {
                found.push(&after[..close]);
                rest = &after[close + 2..];
            }
            None => break,
        }
    }
    found
}

pub(crate) fn render_error(e: minijinja::Error) -> GenerateError {
    use std::error::Error;
    let mut msg = format!("Template render error: {e}");
    let mut source: Option<&dyn Error> = e.source();
    while let Some(cause) = source {
        msg.push_str(&format!("\n  caused by: {cause}"));
        source = cause.source();
    }
    if let Some(detail) = e.detail() {
        msg.push_str(&format!("\n  detail: {detail}"));
    }
    GenerateError::TemplateRender(msg)
}

// ── Mesh-rpc backend gate ────────────────────────────────────────
//
// SCE_MESH.md §9.5 (`<invoke type="sce:mesh-rpc">`) has codegen
// emission only where `templates/mesh/<lang>/` exists. Other backends
// parse the invoke happily (the parser is language-agnostic) but
// produce no transport routing for it — the state's onentry would
// silently ignore the invoke at runtime, which is exactly the "fail
// clearly" inversion CLAUDE.md forbids. This helper turns the silent
// skip into an explicit codegen-time refusal so an operator who picks
// the wrong `--lang` sees the gap immediately, not at runtime.
//
// The refusal is NOT retired the way `<sce:action>`'s was below: mesh
// being C++-only is a stated design constraint, not an accident of
// unfinished work — ARCHITECTURE.md Principle 8 says per-language mesh
// expansion is case-by-case and not a parity obligation. So the gate
// stays and is *split* instead: which languages it refuses is now read
// from the embedded template tree rather than asserted by this
// function. Adding `templates/mesh/<lang>/` is what lifts a refusal,
// so the gate and the templates cannot drift into disagreeing — a
// hand-written "C++-only" could outlive the tree it described.
//
// WHERE THE OTHER HALF OF THAT SPLIT IS WRITTEN. A gate deriving its
// own answer is only half a contract; the other half is the roster an
// author reads before picking `--lang`, and it lives in exactly one
// place — SCE_MESH.md §9.5's backend-coverage table, under the
// `sce:mesh-rpc-backends` anchor. Principle 8 deliberately does NOT
// name the set any more: it used to assert "contains only `cpp/`",
// which is a claim about this tree written where this tree cannot
// correct it, and the same hand-written roster had already gone stale
// in SCE_MESH.md §1 (it listed five non-mesh backends and omitted
// C11). `sce-build/tests/mesh_rpc_backend_contract.rs` holds that
// table to the template tree AND to what this function actually
// answers per `--lang`, and it also reads THIS comment — so a pointer
// that rots is a red rather than something only a reader would
// notice.
fn mesh_templates_exist_for(language: Language) -> bool {
    let prefix = format!("mesh/{}/", language.feature_tree_subdir());
    crate::template_registry::EMBEDDED_TEMPLATES
        .iter()
        .any(|(name, _)| name.starts_with(&prefix))
}

fn reject_mesh_rpc_in_unsupported_lang(
    model: &SCXMLModel,
    language: Language,
) -> Result<(), GenerateError> {
    if !model.has_mesh_rpc_invoke() || mesh_templates_exist_for(language) {
        return Ok(());
    }
    // Named from the tree, so the operator is pointed at the backends
    // that actually carry a mesh arm today rather than at a spelling
    // this message happened to be written with.
    let served: Vec<&'static str> = Language::ALL
        .iter()
        .filter(|candidate| mesh_templates_exist_for(**candidate))
        .map(|candidate| candidate.canonical_name())
        .collect();
    // The remedy is derived for the same reason the diagnosis is. This
    // sentence used to read "generate this machine for `--lang cpp`",
    // three lines under a comment promising the message names no
    // spelling of its own — so a second backend gaining a mesh arm
    // would have widened the "exists for" list and still sent every
    // operator to C++.
    let remedy = served
        .iter()
        .map(|lang| format!("`--lang {lang}`"))
        .collect::<Vec<_>>()
        .join(" / ");
    Err(GenerateError::UnsupportedFeature(format!(
        "<invoke type=\"sce:mesh-rpc\"> in '{}' has no {:?} codegen path \
         (mesh transport emission exists for: {}). \
         Either generate this machine with {} or remove the \
         mesh-rpc invokes from the SCXML.",
        model.name,
        language,
        served.join(", "),
        remedy
    )))
}

// §scxml-G-7 `<sce:action>`: every backend (Rust, C++, C11, Kotlin, Go,
// Python) now lowers a native host action to a direct call on a generated host
// interface, so the former `reject_native_actions_in_unsupported_lang`
// fail-fast gate is retired — no backend can reach codegen with an
// un-lowerable `<sce:action>`.
//
// The refusal was never a design constraint. Validation had always been
// language-neutral (placement, argument shape and cross-call-site signature
// consistency are facts about the document), so the only thing missing was the
// per-language emit — which is why the refusal's own comment called it "purely
// a backend-coverage refusal". The typed-guard channel one node over had
// already walked the same path from Rust-only to six backends and retired its
// gate the same way.
//
// Removing it is the claim that a path exists, and a claim made by deleting
// something has to be paid for elsewhere. Here that is
// `sce-build/tests/native_action_backend_parity.rs` — which asserts that every
// backend emits an interface, a host seam and a call site for the same
// document, that the arg-bearing call is guarded by that backend's typed
// payload tag, and that the guard's OTHER arm answers with `error.execution` in
// one shared wording — plus the six runtime channels named on
// `sce-build/tests/fixtures/event_schema/statechart_native_action.scxml`, each
// driving a real host implementation and asserting the effects fired with the
// typed arguments the event carried.
//
// The fifth question was added on 2026-08-24 because the first four passed
// while the six disagreed about the arm nothing takes: five skipped in silence
// and Rust aborted through a `debug_assert!` that `--release` compiled away.
// Emitting the same guard is not the same as agreeing on what it guards
// against.

// §scxml-6.2.5: a host-declared Event I/O Processor lowers to a
// dispatch into a runtime registry, and a backend without one has
// nowhere for the emitted call to reach. Every backend parses and
// analyses the declaration happily — it rides on the model, which is
// language-agnostic — so the refusal has to be stated per backend.
//
// Refusing here rather than emitting is the whole point. The defect
// being repaid is a build that accepts a `<send type>` it cannot
// service and lets the failure surface hours later at run time;
// honouring a declaration on a backend with no registry would rebuild
// that defect one layer up, with the build now actively promising the
// send would be delivered.
//
// §scxml-6.2.5 host-served Event I/O Processor AND §scxml-6.4.1 host-run
// `<invoke>`: every backend (Rust, C++, C11, Go, Python, Kotlin) now lowers
// both declarations to a dispatch into a runtime registry, so both fail-fast
// gates — `reject_host_processors_in_unsupported_lang` and
// `reject_host_invokers_in_unsupported_lang`, and the shared
// `reject_declaration_in_unsupported_lang` they were built from — are retired.
//
// Their removal is the claim that a path exists, and a claim made by deleting
// something has to be paid for elsewhere. For the send half that is the
// per-backend `a_declared_type_emits_a_dispatch_for_*` tests and the six
// runtime channels. For the invoke half it is the six channels named on
// `sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml`,
// each asserting the four outcomes an invocation's LIFETIME can take: started
// with what the document wrote, cancelled when the state exits, cancelled once
// and only if it started, and reported when nobody ran it. The last two are
// the ones a configuration assertion cannot see — the machine looks correct
// whether or not the host was ever told to stop.
//
// The two were asked separately for as long as a backend had one registry and
// not the other, which C++ started and every other backend then repeated. That
// asymmetry is what ended here; the flags stay separate because the contracts
// are, not because the coverage differs.

/// The native-code prefixes a `cond` may carry, paired with the one
/// backend that lowers each.
///
/// `cpp:` is C++'s and `kt:` is Kotlin's: their templates branch on
/// `is_cpp_condition` / `is_kt_condition` and emit `cond_cpp_transformed`
/// / `cond_kt`, which are the *stripped* bodies. No other backend has
/// that branch.
const NATIVE_COND_PREFIXES: &[(&str, &str)] = &[("cpp:", "C++"), ("kt:", "Kotlin")];

/// The first `cond` in `model` carrying a native prefix this language
/// cannot lower.
fn first_unlowerable_native_cond(model: &SCXMLModel, language: &str) -> Option<(String, String)> {
    fn scan_actions(actions: &[crate::model::Action], language: &str) -> Option<(String, String)> {
        for action in actions {
            if let Some(hit) = unlowerable(&action.cond, language) {
                return Some(hit);
            }
            for branch in &action.elseif_branches {
                if let Some(hit) = unlowerable(&branch.cond, language) {
                    return Some(hit);
                }
                if let Some(hit) = scan_actions(&branch.actions, language) {
                    return Some(hit);
                }
            }
            for nested in action
                .then_actions
                .iter()
                .chain(action.else_actions.iter())
                .chain(action.actions.iter())
            {
                if let Some(hit) = scan_actions(std::slice::from_ref(nested), language) {
                    return Some(hit);
                }
            }
        }
        None
    }

    fn unlowerable(cond: &str, language: &str) -> Option<(String, String)> {
        NATIVE_COND_PREFIXES
            .iter()
            .find(|(prefix, owner)| cond.starts_with(prefix) && *owner != language)
            .map(|(_, owner)| (cond.to_string(), (*owner).to_string()))
    }

    let mut states: Vec<&crate::model::State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        for transition in &state.transitions {
            if let Some(hit) = unlowerable(&transition.cond, language) {
                return Some(hit);
            }
            if let Some(hit) = scan_actions(&transition.actions, language) {
                return Some(hit);
            }
        }
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            if let Some(hit) = scan_actions(block, language) {
                return Some(hit);
            }
        }
        if let Some(hit) = scan_actions(&state.initial_transition_actions, language) {
            return Some(hit);
        }
    }
    None
}

/// §scxml-3.13 native `cond`: a `cpp:` / `kt:` prefix names the language
/// the guard is written in, and only that language's backend has a
/// branch that strips the prefix and emits the body.
///
/// Every other backend used to accept the document and emit something
/// that cannot work, in one of two shapes, both silent:
///
/// * Rust, Go and C11 fall through to the else-branch that emits the
///   `cond` **verbatim** — a branch that is right for a guard which is
///   already a valid expression in the target language (`true`, `1 == 1`)
///   and produces `if cpp:hardware.hasPower() {`, which no compiler
///   accepts, for one that is not. `sce-codegen` reported success and
///   listed the artifact.
/// * Python has no else-branch and lowers the guard through the
///   ECMAScript frontend, which refuses `cpp:…` at the `:`. That
///   compiles, and the guard then raises `error.execution` and reads
///   false on every evaluation — a transition that can never be taken.
///
/// Refused here for the same reason `<sce:action>` is: a construct one
/// backend implements is a backend-axis gap, and the operator who picked
/// the wrong `--lang` should learn it from a diagnostic rather than from
/// a compiler or from a machine that quietly never moves.
fn reject_native_conditions_in_unsupported_lang(
    model: &SCXMLModel,
    language: &'static str,
) -> Result<(), GenerateError> {
    let Some((cond, owner)) = first_unlowerable_native_cond(model, language) else {
        return Ok(());
    };
    Err(GenerateError::UnsupportedFeature(format!(
        "cond=\"{cond}\" in '{}' is a native {owner} guard and has no {language} \
         codegen path — a native cond is lowered only by the backend whose \
         language it names. Either generate this machine for that backend or \
         write the guard as an ECMAScript expression.",
        model.name
    )))
}

/// The backend that can emit `action`'s body, when the body is not
/// ECMAScript — the [`NATIVE_COND_PREFIXES`] question for `<script>`.
///
/// A `<script><cpp>…</cpp></script>` body is C++ source and a
/// `<script><kt>…</kt></script>` body is Kotlin source; the parser
/// records which by setting `is_cpp_function` / `is_kt_function`, and
/// only those two backends have a branch that emits the body as code.
///
/// A function rather than the table its `cond` sibling uses, because the
/// discriminator here is a field on the action rather than a prefix on a
/// string: there is no data to tabulate.
fn native_script_owner(action: &crate::model::Action) -> Option<&'static str> {
    if action.action_type != "script" {
        return None;
    }
    if action.is_cpp_function {
        return Some("C++");
    }
    if action.is_kt_function {
        return Some("Kotlin");
    }
    None
}

/// The first `<script>` in `model` written in a language this backend
/// cannot emit.
fn first_unlowerable_native_script(model: &SCXMLModel, language: &str) -> Option<String> {
    fn owner(action: &crate::model::Action, language: &str) -> Option<String> {
        native_script_owner(action)
            .filter(|owner| *owner != language)
            .map(str::to_string)
    }

    fn scan(actions: &[crate::model::Action], language: &str) -> Option<String> {
        for action in actions {
            if let Some(hit) = owner(action, language) {
                return Some(hit);
            }
            for branch in &action.elseif_branches {
                if let Some(hit) = scan(&branch.actions, language) {
                    return Some(hit);
                }
            }
            for nested in action
                .then_actions
                .iter()
                .chain(action.else_actions.iter())
                .chain(action.actions.iter())
            {
                if let Some(hit) = scan(std::slice::from_ref(nested), language) {
                    return Some(hit);
                }
            }
        }
        None
    }

    let mut states: Vec<&crate::model::State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        for transition in &state.transitions {
            if let Some(hit) = scan(&transition.actions, language) {
                return Some(hit);
            }
        }
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            if let Some(hit) = scan(block, language) {
                return Some(hit);
            }
        }
        if let Some(hit) = scan(&state.initial_transition_actions, language) {
            return Some(hit);
        }
    }
    None
}

/// A native `<script>`: the same rule
/// [`reject_native_conditions_in_unsupported_lang`] enforces for `cond`,
/// for the element one node over.
///
/// The two had drifted. A native `cond` was refused on the backends that
/// cannot lower it; a native `<script>` was not, and the four backends
/// that run the datamodel on Lua piped its C++ body straight through the
/// ECMAScript frontend. That produced a Lua chunk calling
/// `aim.onDisabled()` on a session where `aim` is a C++ object and no Lua
/// value at all — nil, at run time, with `sce-codegen` reporting success.
///
/// Found by giving the frontend the document's declarations: `aim` is
/// declared by `<sce:context>` for the C++ backend and by nothing for the
/// others, so the refusal that had been invisible became a message. The
/// message is right and the lowering is what was wrong.
fn reject_native_scripts_in_unsupported_lang(
    model: &SCXMLModel,
    language: &'static str,
) -> Result<(), GenerateError> {
    let Some(owner) = first_unlowerable_native_script(model, language) else {
        return Ok(());
    };
    Err(GenerateError::UnsupportedFeature(format!(
        "<script><{}>…</{}></script> in '{}' is a native {owner} body and has no \
         {language} codegen path — a native script is lowered only by the backend \
         whose language it names. Either generate this machine for that backend or \
         write the script as ECMAScript.",
        if owner == "C++" { "cpp" } else { "kt" },
        if owner == "C++" { "cpp" } else { "kt" },
        model.name
    )))
}

// EventSchema MCU native lowering: every backend
// (Rust, C11, C++, Kotlin, Go, Python) now lowers a typed `_event.data`
// transition guard to a script-engine-free native comparison, so the former
// `reject_typed_native_guard_unsupported` fail-fast gate is retired — no
// backend can reach codegen with an un-lowerable typed guard.

// ── §16.5 L3500 barrier-timeout observability gate ────────────────
//
// A deploy.yaml `barrier_timeout_ms:` on a Root partition is a signal
// that the author wants to **observe** the `error.communication`
// (reason `PARALLEL_BARRIER_TIMEOUT`) raise when regions fail to
// converge in time. If the author's SCXML carries no transition for
// `error.communication`, the raised event falls into the default
// microstep path and is silently discarded — the knob is set but has
// no observable consequence, the `feedback_silently_broken_hooks`
// anti-pattern verbatim. Refuse at codegen instead so the gap
// surfaces with the SCXML in hand rather than as a post-deploy
// observation that "the timeout does nothing".
//
// The check is local to the machine currently being codegen'd. A
// distributed `<parallel>` whose Root lives in a different machine
// has `partition_barrier_timeouts` empty here (only Root-owning
// machines carry an entry); NonRoot machines never reach this gate.
fn reject_barrier_timeout_without_handler(model: &SCXMLModel) -> Result<(), GenerateError> {
    if model.partition_barrier_timeouts.is_empty() {
        return Ok(());
    }
    if model.events.contains("error.communication") {
        return Ok(());
    }
    let parallels: Vec<String> = model.partition_barrier_timeouts.keys().cloned().collect();
    Err(GenerateError::UnsupportedFeature(format!(
        "machine '{}' declares `barrier_timeout_ms:` on a Root partition \
         for <parallel id=\"{}\"> but the SCXML has no transition for \
         event `error.communication`. SCE_MESH.md §16.5 raises \
         `error.communication` (reason PARALLEL_BARRIER_TIMEOUT) when \
         the barrier elapses — without a transition the raise is \
         silently discarded and the timeout has no observable effect. \
         Add a `<transition event=\"error.communication\">` handler \
         (optionally guarded on `_event.data.reason == \
         'PARALLEL_BARRIER_TIMEOUT'`) or drop `barrier_timeout_ms:` \
         from the partition declaration.",
        model.name,
        parallels.join(", ")
    )))
}

// ── §16.4 / §16.7 liveness observability gate ────────────────────
//
// Symmetric to `reject_barrier_timeout_without_handler` for the
// liveness raise paths. `deploy.yaml`'s `liveliness:` block drives
// two §16.7 rows that both surface as `error.communication`:
//   - row 8 `PEER_PARTITIONED` — fires on DROP of a machine-level
//     `sce/live/<machine>` token, i.e. on every machine that
//     declares `liveliness:` regardless of partitioning.
//   - row 13 `REGION_PARTITIONED` — fires on DROP of a partition
//     token `sce/live/<machine>/<partition>` — partitioned machines
//     only.
// Without a matching `<transition event="error.communication">`
// either raise sinks into the default microstep path and is
// silently discarded, which is exactly the
// `feedback_silently_broken_hooks` anti-pattern. A single gate
// covers both rows because the model flag is set whenever the
// machine declares `liveliness:` — there is no `liveliness:` shape
// that produces row 13 without also authorizing row 8.
fn reject_liveliness_without_handler(model: &SCXMLModel) -> Result<(), GenerateError> {
    if !model.machine_liveliness_opt_in {
        return Ok(());
    }
    if model.events.contains("error.communication") {
        return Ok(());
    }
    Err(GenerateError::UnsupportedFeature(format!(
        "machine '{}' declares `liveliness:` but the SCXML has no \
         transition for event `error.communication`. SCE_MESH.md \
         §16.4 / §16.7 rows 8 and 13 raise `error.communication` \
         (reason PEER_PARTITIONED or REGION_PARTITIONED) when a \
         peer's Zenoh liveliness token drops — without a transition \
         the raise is silently discarded and the signal has no \
         observable effect. Add a `<transition \
         event=\"error.communication\">` handler (optionally guarded \
         on `_event.data.reason`) or drop `liveliness:` from the \
         machine declaration.",
        model.name
    )))
}

// ── Rust generator ───────────────────────────────────────────────

/// Caller-facing knobs for the statechart Rust codegen. Defaults
/// reproduce the pre-options behaviour byte-for-byte (std build, no
/// extra derives), so every caller routing through [`generate`] is
/// unaffected.
#[derive(Default, Clone, Debug)]
pub struct StatechartCodegenOptions {
    /// `no_std` codegen mode (SCE Protocol-Synthesis RFC §synth-5-J-2): emits
    /// `#![no_std]` at the crate root and switches
    /// `parent_external_queue` + microstep `HashSet` to heapless
    /// variants. Default `false` keeps std-coupled output for the
    /// existing 200+ AOT W3C fixtures byte-identical.
    pub no_std: bool,
    /// Extra derive arguments appended verbatim to the generated
    /// `{machine}State` enum (e.g. `"serde::Serialize"`,
    /// `"my_crate::MyStateDerive"`). Deduped against the SSOT
    /// defaults ([`crate::rust_derive_policy::RustDeriveCategory::StatechartState`]).
    /// Empty ⇒ no change. The consuming crate must have the named
    /// traits / derive-macros in scope; SCE forwards the paths
    /// unresolved and takes on no dependency for them.
    pub state_extra_derives: Vec<String>,
    /// As above, for the generated `{machine}Event` enum
    /// ([`crate::rust_derive_policy::RustDeriveCategory::StatechartEvent`]).
    pub event_extra_derives: Vec<String>,
    /// Event I/O Processor `type` values the calling host serves
    /// (§scxml-6.2.5 makes the identifier extensible). The library
    /// equivalent of `sce-codegen --host-processor`.
    ///
    /// A `<send>` naming one of these lowers to a dispatch into
    /// [`Engine::register_event_processor`]'s registry instead of the
    /// §scxml-6.2 `error.execution` refusal. Empty ⇒ unchanged
    /// behaviour, which is why the default reproduces every existing
    /// caller byte-for-byte.
    ///
    /// Honoured by every backend. It was Rust-only, with
    /// `reject_host_processors_in_unsupported_lang` refusing the other
    /// five by name so a declaration could not have the build promise a
    /// delivery nothing performs; all six grew a registry and that gate
    /// retired — see the note above, which this line contradicted until
    /// 2026-08-24. What pays for the deletion is the per-backend
    /// `a_declared_type_emits_a_dispatch_for_*` tests and the six runtime
    /// channels.
    pub host_processor_types: Vec<String>,
    /// `<invoke type="...">` values the calling host can run
    /// (§scxml-6.4.1 leaves the invokable set to the platform). The
    /// library equivalent of `sce-codegen --host-invoker`.
    ///
    /// Separate from [`Self::host_processor_types`] because they are
    /// separate contracts; see that field. Empty ⇒ unchanged behaviour.
    pub host_invoker_types: Vec<String>,
}

/// Generate Rust code from an analyzed SCXMLModel (filesystem-based).
///
/// See [`StatechartCodegenOptions::no_std`] for the `no_std` toggle.
/// Thin delegate over [`generate_with_options`] for callers that only
/// need the `no_std` knob (the `--no-std` CLI flag, the AOT harness).
pub fn generate(
    model: &SCXMLModel,
    template_dir: &Path,
    no_std: bool,
) -> Result<String, GenerateError> {
    generate_with_options(
        model,
        template_dir,
        &StatechartCodegenOptions {
            no_std,
            ..Default::default()
        },
    )
}

/// Generate Rust code from an analyzed SCXMLModel (filesystem-based),
/// with full [`StatechartCodegenOptions`]. This is the base entry;
/// [`generate`] delegates here. Downstream consumers that need to
/// inject extra derives on the generated `State` / `Event` enums
/// (serde, an a11y / RPC-introspect proc-macro derive) route through
/// [`crate::compile_scxml_with_derives`], which reaches this via
/// [`crate::compile_scxml_lang_typed_with_section`].
pub fn generate_with_options(
    model: &SCXMLModel,
    template_dir: &Path,
    options: &StatechartCodegenOptions,
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Rust)?;
    reject_native_conditions_in_unsupported_lang(model, "Rust")?;
    reject_native_scripts_in_unsupported_lang(model, "Rust")?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_filters(&mut env, &document_scope(model));
    render_rust(&mut env, model, options)
}

/// Render the `mod.rs` module index for a generated Rust state-machine
/// directory.
///
/// Separate from [`generate_with_options`] because it is a separate
/// artifact with its own drift header, but it goes through the same
/// template environment so its SCE-MAP marker is rendered by the one
/// macro that owns the marker shape —
/// and points at the machine's real source location rather than a
/// hand-written guess.
pub fn generate_rust_module_index(
    model: &SCXMLModel,
    template_dir: &Path,
    module_stem: &str,
) -> Result<String, GenerateError> {
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_filters(&mut env, &document_scope(model));
    let tmpl = env
        .get_template("module_index.rs.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;
    tmpl.render(minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        module_stem => module_stem,
    })
    .map_err(|e| GenerateError::TemplateRender(e.to_string()))
}

/// The module index an `OUT_DIR` artifact needs, where the module file is
/// not a sibling of the source that names it.
///
/// Same two lines [`generate_rust_module_index`] emits, from the same
/// template, with a `#[path]` naming `module_file` absolutely. The
/// consumer `include!`s the result; `include!` accepts it because every
/// line is an item or an outer attribute, and the `mod` then loads the
/// generated machine as a module **file**, where its inner attributes and
/// inner doc comments are legal.
///
/// That is the whole point of the indirection. The generated machine
/// opens with an audited suppression budget spelled as inner attributes;
/// `include!`ing the machine directly puts those attributes in expansion
/// position, where rustc refuses them, so consumers stripped the lines
/// and replaced the budget with a blanket `#![allow(warnings)]` of their
/// own. Two independent consumers reached that same workaround
/// byte-for-byte. Through this shim they delete nothing and the budget
/// arrives as SCE audited it.
pub fn generate_rust_include_shim(
    template_dir: &Path,
    module_stem: &str,
    module_file: &Path,
) -> Result<String, GenerateError> {
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    // No document, and none is needed: `module_index.rs.jinja2` renders
    // two items naming a path, and the one thing a scope decides — which
    // identifiers an authored expression may read — has no expression
    // here to decide it for.
    filters::register_filters(&mut env, &Arc::new(DocumentScope::installed()));
    let tmpl = env
        .get_template("module_index.rs.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(e.to_string()))?;
    // Rust's own `Debug` for `str` is the escaper here: it produces a
    // valid Rust string literal for any path the filesystem can name,
    // including the backslashes a Windows `OUT_DIR` carries.
    let module_path = format!("{:?}", module_file.to_string_lossy());
    tmpl.render(minijinja::context! {
        module_stem => module_stem,
        module_path => module_path,
    })
    .map_err(|e| GenerateError::TemplateRender(e.to_string()))
}

/// Generate Rust code using pre-loaded template strings (WASM-compatible).
///
/// See [`StatechartCodegenOptions::no_std`] for `no_std` semantics. The
/// WASM surface exposes only the `no_std` knob today (no consumer
/// injects extra derives through the in-memory template path).
pub fn generate_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
    no_std: bool,
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Rust)?;
    reject_native_conditions_in_unsupported_lang(model, "Rust")?;
    reject_native_scripts_in_unsupported_lang(model, "Rust")?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_filters(&mut env, &document_scope(model));
    render_rust(
        &mut env,
        model,
        &StatechartCodegenOptions {
            no_std,
            ..Default::default()
        },
    )
}

fn render_rust(
    env: &mut Environment,
    model: &SCXMLModel,
    options: &StatechartCodegenOptions,
) -> Result<String, GenerateError> {
    let machine_name = filters::to_pascal_case(model.name.clone());

    // SCE Forge: render inline kind declarations as Rust code fragments.
    let (inline_kind_types, inline_kind_fns) = if !model.inline_kinds.is_empty() {
        let code = crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Rust,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?;
        (code.type_defs, code.member_fns)
    } else {
        (String::new(), String::new())
    };

    // EventSchema MCU native lowering (step 2) —
    // the typed `_event.data` payload sum, its `type Payload` spelling, and
    // the per-transition native `matches!(…)` guards. The per-machine defs /
    // type / inject seams ride in the render context (no IR home); the
    // per-transition guards ride home on the transition's
    // `native_payload_guard` via a single-language clone — co-located with
    // their owning transition so a per-state `transition_index` cannot
    // collide them across states.
    let mut model_lowered = model.clone();
    // Stamp each transition with the
    // symbol identity the sourcemap keys off, before any analysis pass
    // clones or serialises the transitions.
    symbol_mangling::stamp_symbol_attribution(&mut model_lowered);
    // §scxml-G-7: lower `<sce:action>` Custom Action Elements to native
    // host-trait dispatch (engine-free). Mutates each native action's
    // rendered call site on `model_lowered` and returns the generated
    // `Actions` trait plus the payload events those actions activate. When
    // the document has no native actions every output below collapses to the
    // empty string, so the emitted code is byte-identical to the pre-feature
    // baseline (the generic `Policy<A>` never appears).
    let native =
        crate::forge::native_action::render(&mut model_lowered, &machine_name, Language::Rust);
    // `Engine<P: StatePolicy>` requires `P: 'static` (the runtime stores the
    // policy and erases lifetimes through it), so the host `Actions` type
    // parameter carries the same bound. A host impl is a plain owned type, so
    // `'static` is the expected, non-restrictive shape.
    let (policy_generics_decl, policy_generics_use) = if native.any {
        (
            format!("<A: {} + 'static>", native.interface_name),
            "<A>".to_string(),
        )
    } else {
        (String::new(), String::new())
    };
    let payload = crate::forge::generator::build_rust_event_payload(
        model,
        &machine_name,
        &native.payload_events,
        &policy_generics_decl,
        &policy_generics_use,
    );
    crate::forge::generator::apply_native_guard_writes(&mut model_lowered, &payload.guard_writes);

    let tmpl = env
        .get_template("state_machine.rs.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    // State/Event enum derives flow through the shared Rust
    // derive-policy SSOT: the category defaults plus any caller-injected
    // extras (serde, an a11y / RPC-introspect proc-macro derive),
    // deduped. Empty extras render byte-identical to the pre-SSOT
    // hardcoded `#[derive(...)]` lines.
    let state_derives_attr = crate::rust_derive_policy::render_derives_attr(
        crate::rust_derive_policy::RustDeriveCategory::StatechartState.derives(),
        &options.state_extra_derives,
    );
    let event_derives_attr = crate::rust_derive_policy::render_derives_attr(
        crate::rust_derive_policy::RustDeriveCategory::StatechartEvent.derives(),
        &options.event_extra_derives,
    );

    // The state whose variant carries `#[default]`, so the `Default`
    // derive on `{Machine}State` and `initial_state()` resolve to the
    // SAME `<scxml initial>` state from ONE computation: first whitespace
    // token of `model.initial` that is a real state, else the first
    // sorted state key. This guarantees exactly one `#[default]` marker
    // (the invariant `#[derive(Default)]` requires) and is robust to a
    // multi-token (parallel-root) initial. The template consumes it for
    // both the marker and the `initial_state()` body, so the two can
    // never disagree. The externally-drivable surface is the analyzer's
    // structural fact `model.externally_drivable_events` (published;
    // emitted as the `{Machine}Event::EXTERNALLY_DRIVABLE_EVENTS` const),
    // consumed verbatim below — codegen re-derives no partition.
    let default_state_id = model
        .initial
        .split_whitespace()
        .find(|t| model.states.contains_key(*t))
        .map(str::to_string)
        .or_else(|| model.states.keys().next().cloned())
        .unwrap_or_default();
    let externally_drivable_events: Vec<&String> =
        model.externally_drivable_events.iter().collect();

    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(&model_lowered),
        machine_name => machine_name,
        license_config => minijinja::Value::from_serialize(license_config()),
        inline_kind_types => &inline_kind_types,
        inline_kind_fns => &inline_kind_fns,
        no_std => options.no_std,
        state_derives_attr => &state_derives_attr,
        event_derives_attr => &event_derives_attr,
        default_state_id => &default_state_id,
        externally_drivable_events => &externally_drivable_events,
        event_payload_defs => &payload.defs,
        event_payload_type => &payload.type_name,
        event_payload_entries => &payload.entries,
        has_native_actions => native.any,
        native_actions_defs => &native.interface_def,
        native_actions_interface => &native.interface_name,
        policy_generics_decl => &policy_generics_decl,
        policy_generics_use => &policy_generics_use,
    };
    tmpl.render(ctx).map_err(render_error)
}

// ── C++ generator ────────────────────────────────────────────────

/// Generate C++ code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_cpp(
    model: &SCXMLModel,
    template_dir: &Path,
    input_stem: &str,
    cpp_namespace_prefix: Option<&str>,
) -> Result<GeneratedOutput, GenerateError> {
    generate_cpp_for_engine(
        model,
        template_dir,
        input_stem,
        cpp_namespace_prefix,
        Language::Cpp.default_script_engine_target(),
    )
}

/// [`generate_cpp`] with the run's engine selection named.
///
/// The seam's codegen input reaches the templates here and nowhere else: the
/// filters close over it, so a template cannot ask — and therefore cannot ask
/// inconsistently within one artifact.
pub fn generate_cpp_for_engine(
    model: &SCXMLModel,
    template_dir: &Path,
    input_stem: &str,
    cpp_namespace_prefix: Option<&str>,
    script_engine: ScriptEngineTarget,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_cpp_filters_for_engine(&mut env, &document_scope(model), script_engine);
    render_cpp(&mut env, model, input_stem, cpp_namespace_prefix)
}

/// Generate C++ code using pre-loaded template strings (WASM-compatible).
pub fn generate_cpp_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_cpp_filters(&mut env, &document_scope(model));
    render_cpp(&mut env, model, input_stem, None)
}

fn render_cpp(
    env: &mut Environment,
    model: &SCXMLModel,
    input_stem: &str,
    cpp_namespace_prefix: Option<&str>,
) -> Result<GeneratedOutput, GenerateError> {
    reject_barrier_timeout_without_handler(model)?;
    reject_liveliness_without_handler(model)?;
    // Neither host refusal here: the C++ AOT engine carries both registries
    // (`StaticExecutionEngine::registerEventProcessor` for §scxml-6.2.5 and
    // `registerInvoker` for §scxml-6.4.1), and the templates emit the `<send>`
    // dispatch and the `<invoke>` start/cancel pair into them. The invoker half
    // landed second because it is the wider of the two: an invocation has a
    // lifetime the exit chain has to end.
    reject_native_conditions_in_unsupported_lang(model, "C++")?;
    reject_native_scripts_in_unsupported_lang(model, "C++")?;
    let inl_filename = format!("{input_stem}_sm.inl");
    // §scxml-5.3: base_path is the directory containing the SCXML file,
    // used by DataModelInitHelper for resolving file: URIs in data src attributes.
    // Python codegen uses Path(output_dir).name; we use scxml_base_path which is
    // the parent directory of the SCXML file (set by analyzer::compute_scxml_base_path).
    let base_path = model.scxml_base_path.clone();

    // SCE Forge: render inline kind declarations as C++ code fragment.
    let inline_kind_code = if !model.inline_kinds.is_empty() {
        let machine_name = filters::to_pascal_case(model.name.clone());
        crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Cpp,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?
        .member_fns
    } else {
        String::new()
    };

    // EventSchema native lowering — the C++ typed
    // `_event.data` payload channel: a tag enum + per-event payload structs,
    // the policy fields / `populateTypedPayload` hook that lift the dequeued
    // event's std::any-carried payload, the per-event `raise<Event>` inject
    // seams, and the per-transition native guard (`pendingPayloadTag_ == … &&
    // (…)`). The per-machine defs ride in the render context; the
    // per-transition guards ride home on the transition's
    // `native_payload_guard` via a single-language clone (same SSOT guard
    // selection as every backend).
    let mut model_lowered = model.clone();
    // Stamp each transition with the
    // symbol identity the sourcemap keys off, before any analysis pass
    // clones or serialises the transitions.
    symbol_mangling::stamp_symbol_attribution(&mut model_lowered);
    // §scxml-G-7: lower `<sce:action>` Custom Action Elements to native host
    // dispatch (engine-free) — same call every backend makes.
    let native_machine_name = filters::to_pascal_case(model.name.clone());
    let native = crate::forge::native_action::render(
        &mut model_lowered,
        &native_machine_name,
        Language::Cpp,
    );
    let payload = crate::forge::generator::build_cpp_event_payload(model, &native.payload_events);
    crate::forge::generator::apply_native_guard_writes(&mut model_lowered, &payload.guard_writes);

    let header_tmpl = env
        .get_template("state_machine.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let inl_tmpl = env
        .get_template("state_machine_inl.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;

    let model_val = minijinja::Value::from_serialize(&model_lowered);
    let license_val = minijinja::Value::from_serialize(license_config());

    // Suite namespace: when set, this is the ready-to-prepend `<prefix>::`
    // segment (empty when unset) that nests the emitted machine namespace
    // under `SCE::Generated::<prefix>::<name>` so identically-named
    // machines from different catalogs coexist in one binary. Computed
    // once here and prepended at every `::SCE::Generated::` site (the `.h`
    // declarations and the `.inl` invoke/child-send refs) — the same
    // Rust-computed-prepend shape as the C11 backend's `csym_prefix`,
    // rather than a conditional repeated at each template site. Empty =
    // the historical un-nested shape (byte-identical).
    let cpp_ns_prefix = match cpp_namespace_prefix {
        Some(p) if !p.is_empty() => format!("{p}::"),
        _ => String::new(),
    };

    let header_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
        inl_filename => &inl_filename,
        inline_kind_code => &inline_kind_code,
        event_payload_defs => &payload.defs,
        event_payload_active => payload.active,
        event_payload_policy_members => &payload.policy_members,
        event_payload_inject_methods => &payload.inject_methods,
        has_native_actions => native.any,
        native_actions_defs => &native.interface_def,
        native_actions_interface => &native.interface_name,
        cpp_ns_prefix => &cpp_ns_prefix,
    };
    let inl_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
        has_native_actions => native.any,
        native_actions_interface => &native.interface_name,
        // Mirrored from the .h context: the `.inl` carries no namespace of
        // its own, but its invoke/child-send bodies reference sibling
        // machines via `::SCE::Generated::<child>` and must carry the same
        // prefix as the `.h`'s child-member declarations.
        cpp_ns_prefix => &cpp_ns_prefix,
    };

    let header_code = header_tmpl.render(header_ctx).map_err(render_error)?;
    let header_code = postprocess_cpp_header(&header_code);
    let inl_code = inl_tmpl.render(inl_ctx).map_err(render_error)?;
    let inl_code = postprocess_cpp_inl(&inl_code);

    Ok(GeneratedOutput {
        files: vec![
            (format!("{input_stem}_sm.h"), header_code),
            (inl_filename, inl_code),
        ],
        ..Default::default()
    })
}

// ── C11 generator ────────────────────────────────────────────────
//
// RFC §synth-5-J-1 — downstream consumer (MCU AOT backend). Mirrors the
// C++ pair-render shape (`generate_cpp` above) but emits a `.h` + `.c`
// translation unit instead of `.h` + `.inl` because C11 has no in-class
// definitions to hide behind a textual include.
//
// Mesh patterns are out-of-scope for C11 per the RFC — we still call
// the mesh-shape rejectors so an SCXML carrying mesh markings fails
// loud here rather than producing a half-rendered translation unit.

/// Generate C11 code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_c11(
    model: &SCXMLModel,
    template_dir: &Path,
    input_stem: &str,
    c_symbol_prefix: Option<&str>,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_c11_filters(&mut env, &document_scope(model));
    render_c11(&mut env, model, input_stem, c_symbol_prefix)
}

/// Generate C11 code using pre-loaded template strings (WASM-compatible).
pub fn generate_c11_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
    input_stem: &str,
) -> Result<GeneratedOutput, GenerateError> {
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_c11_filters(&mut env, &document_scope(model));
    render_c11(&mut env, model, input_stem, None)
}

fn render_c11(
    env: &mut Environment,
    model: &SCXMLModel,
    input_stem: &str,
    c_symbol_prefix: Option<&str>,
) -> Result<GeneratedOutput, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::C11)?;
    // Neither host refusal here: the C11 backend carries
    // `sce_host_processor_registry_t` for §scxml-6.2.5 and
    // `sce_host_invoker_registry_t` + `sce_host_invocation_set_t` for
    // §scxml-6.4.1 on the generated struct, and the templates emit the
    // `<send>` dispatch and the `<invoke>` start/cancel pair into them. Both
    // are bounded arrays rather than maps, because the freestanding profile
    // has no allocator.
    reject_native_conditions_in_unsupported_lang(model, "C11")?;
    reject_native_scripts_in_unsupported_lang(model, "C11")?;
    reject_barrier_timeout_without_handler(model)?;
    reject_liveliness_without_handler(model)?;
    let base_path = model.scxml_base_path.clone();

    // SCE Forge: render inline kind declarations as C11 code fragment.
    // Mirrors cpp/Kotlin's single-block emit (no top-level type_defs split
    // because C11 has no nested types — enum typedefs and `static inline`
    // functions both flow into member_fns and inject after the policy
    // typedef in state_machine.h.jinja2).
    let inline_kind_code = if !model.inline_kinds.is_empty() {
        let machine_name = filters::to_pascal_case(model.name.clone());
        crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::C11,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?
        .member_fns
    } else {
        String::new()
    };

    // EventSchema MCU native lowering — the C11 typed `_event.data`
    // payload channel: a tagged
    // union `<name>_payload_t`, the per-transition native guard
    // (`sm->pending_payload.tag == … && (…)`), and the `type_name` used by
    // the `event_with_meta`/`pending_payload` fields and the
    // `raise_external_typed` seam. The per-machine defs ride in the render
    // context; the per-transition guards ride home on the transition's
    // `native_payload_guard` via a single-language clone (same SSOT guard
    // selection as every backend).
    let mut model_lowered = model.clone();
    // Stamp each transition with the
    // symbol identity the sourcemap keys off, before any analysis pass
    // clones or serialises the transitions.
    symbol_mangling::stamp_symbol_attribution(&mut model_lowered);
    // §scxml-G-7: lower `<sce:action>` Custom Action Elements to native host
    // dispatch (engine-free). The C11 token is the raw snake stem — the same
    // one `build_c11_event_payload` names its types after — not the PascalCase
    // machine name the hosted backends use.
    let native =
        crate::forge::native_action::render(&mut model_lowered, &model.name, Language::C11);
    let payload = crate::forge::generator::build_c11_event_payload(model, &native.payload_events);
    crate::forge::generator::apply_native_guard_writes(&mut model_lowered, &payload.guard_writes);

    let header_tmpl = env
        .get_template("c/state_machine.h.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let source_tmpl = env
        .get_template("c/state_machine.c.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;

    let model_val = minijinja::Value::from_serialize(&model_lowered);
    let license_val = minijinja::Value::from_serialize(license_config());

    // Suite symbol prefix: C has no namespace, so every emitted symbol is
    // `<name>_…`. When set, this is the ready-to-prepend `<prefix>_` string
    // (empty when unset) that nests every self/child symbol — including the
    // struct tag and UPPER macro/enum names — under the suite prefix so
    // identically-named machines from different catalogs link into one
    // binary without an ODR clash. The C11 peer of `--cpp-namespace-prefix`.
    // Empty = the historical un-prefixed shape (byte-identical). Filenames,
    // SCE-MAP markers, and the Lua session-id string keep the logical name.
    let csym_prefix = match c_symbol_prefix {
        Some(p) if !p.is_empty() => format!("{p}_"),
        _ => String::new(),
    };

    let header_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
        inline_kind_code => &inline_kind_code,
        event_payload_defs => &payload.defs,
        event_payload_type => &payload.type_name,
        event_payload_active => payload.active,
        event_payload_entry_decls => &payload.entry_decls,
        has_native_actions => native.any,
        native_actions_defs => &native.interface_def,
        native_actions_interface => &native.interface_name,
        native_action_ops => &native.operation_names,
        csym_prefix => &csym_prefix,
    };
    let source_ctx = minijinja::context! {
        model => &model_val,
        base_path => &base_path,
        license_config => &license_val,
        event_payload_type => &payload.type_name,
        event_payload_active => payload.active,
        event_payload_entry_defs => &payload.entry_defs,
        has_native_actions => native.any,
        native_actions_interface => &native.interface_name,
        native_action_ops => &native.operation_names,
        csym_prefix => &csym_prefix,
    };

    let header_code = header_tmpl.render(header_ctx).map_err(render_error)?;
    let source_code = source_tmpl.render(source_ctx).map_err(render_error)?;

    Ok(GeneratedOutput {
        files: vec![
            (format!("{input_stem}_sm.h"), header_code),
            (format!("{input_stem}_sm.c"), source_code),
        ],
        ..Default::default()
    })
}

// ── Kotlin generator ─────────────────────────────────────────────

/// Generate Kotlin code from an analyzed SCXMLModel (filesystem-based).
///
/// `package_prefix` overrides the emitted `package` header's prefix. `None`
/// keeps the default `com.sce.generated` used by every W3C IRP fixture;
/// callers that emit into a non-W3C namespace (e.g. integration fixtures
/// under `com.sce.integration`) pass `Some(prefix)`.
pub fn generate_kotlin(
    model: &SCXMLModel,
    template_dir: &Path,
    package_prefix: Option<&str>,
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Kotlin)?;
    // Neither host refusal here: the Kotlin runtime carries
    // `StateMachineEngine.registerEventProcessor` for §scxml-6.2.5 and
    // `registerInvoker` for §scxml-6.4.1, and the templates emit the `<send>`
    // dispatch and the `<invoke>` start/cancel pair into them.
    reject_native_conditions_in_unsupported_lang(model, "Kotlin")?;
    reject_native_scripts_in_unsupported_lang(model, "Kotlin")?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_kotlin_filters(&mut env, &document_scope(model));
    register_kotlin_dynamic_filters(&mut env, model);
    render_kotlin(&mut env, model, package_prefix)
}

/// Generate Kotlin code using pre-loaded template strings (WASM-compatible).
///
/// `package_prefix` matches [`generate_kotlin`]'s semantics.
pub fn generate_kotlin_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
    package_prefix: Option<&str>,
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Kotlin)?;
    // See `generate_kotlin` above: the Kotlin backend carries both host
    // registries.
    reject_native_conditions_in_unsupported_lang(model, "Kotlin")?;
    reject_native_scripts_in_unsupported_lang(model, "Kotlin")?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_kotlin_filters(&mut env, &document_scope(model));
    register_kotlin_dynamic_filters(&mut env, model);
    render_kotlin(&mut env, model, package_prefix)
}

/// Register model-dependent Kotlin filters (event refs, parallel checks).
fn register_kotlin_dynamic_filters(env: &mut Environment, model: &SCXMLModel) {
    use crate::kotlin;

    let kotlin_events: std::collections::BTreeSet<String> = model
        .events
        .iter()
        .filter(|e| e.as_str() != "Wildcard")
        .cloned()
        .collect();
    let event_tree = kotlin::build_event_tree(&kotlin_events);
    let branch_events = kotlin::collect_branch_events(&event_tree, "");

    let branch_events_clone = branch_events.clone();
    env.add_filter("to_event_ref", move |name: String| -> String {
        kotlin::to_event_ref(&name, &branch_events_clone)
    });

    let parallel_regions = model.parallel_regions.clone();
    let states_for_check = model.states.clone();
    env.add_filter(
        "to_parallel_complete_check",
        move |parallel_id: String| -> String {
            kotlin::to_parallel_complete_check(&parallel_id, &parallel_regions, &states_for_check)
        },
    );
}

fn render_kotlin(
    env: &mut Environment,
    model: &SCXMLModel,
    package_prefix: Option<&str>,
) -> Result<String, GenerateError> {
    use crate::{analyzer, kotlin};

    // EventSchema MCU native lowering — the Kotlin typed
    // `_event.data` payload channel: top-level payload data classes, the
    // nullable `pending<Event>Payload` fields + `populateTypedPayload` override
    // that lift the dequeued event's typed carrier, the per-event `raise<Event>`
    // inject seams, and the per-transition native guard (`pending<Event>Payload
    // != null && (…)`). The per-machine defs ride in the render context; the
    // per-transition guards ride home on the transition's
    // `native_payload_guard` via a single-language clone (same SSOT guard
    // selection as every backend).
    let mut model_lowered = model.clone();
    // Stamp each transition with the
    // symbol identity the sourcemap keys off, before any analysis pass
    // clones or serialises the transitions.
    symbol_mangling::stamp_symbol_attribution(&mut model_lowered);
    // §scxml-G-7: lower `<sce:action>` Custom Action Elements to native host
    // dispatch (engine-free) — same call every backend makes.
    let native_machine_name = filters::to_pascal_case(model.name.clone());
    let native = crate::forge::native_action::render(
        &mut model_lowered,
        &native_machine_name,
        Language::Kotlin,
    );
    let payload =
        crate::forge::generator::build_kotlin_event_payload(model, &native.payload_events);
    crate::forge::generator::apply_native_guard_writes(&mut model_lowered, &payload.guard_writes);
    let model = &model_lowered;

    let machine_name = filters::to_pascal_case(model.name.clone());

    // Shared analysis (language-agnostic, from analyzer)
    let ancestor_chains = analyzer::compute_ancestor_chains(model);
    let parent_map = analyzer::compute_parent_map(model);
    let leaf_map = analyzer::compute_leaf_map(model);
    let parallel_descendants = analyzer::compute_parallel_descendants(model);
    let initial_entry_root = analyzer::compute_initial_entry_root(model);

    // Kotlin-specific analysis (serde_json output for template rendering)
    let effective_transitions = kotlin::compute_effective_transitions(model, &ancestor_chains);
    let (ancestors_with_event_transitions, ancestors_with_null_transitions) =
        kotlin::compute_ancestors_with_transitions(model, &ancestor_chains);
    let process_event_needs_else =
        kotlin::process_event_needs_else(model, &ancestors_with_event_transitions);
    let process_null_event_needs_else =
        kotlin::process_null_event_needs_else(model, &ancestors_with_null_transitions);
    let transition_actions_needs_else =
        kotlin::transition_actions_needs_else(&effective_transitions);
    let deep_initial_entries = kotlin::compute_deep_initial_entries(model);
    let invoke_entries = kotlin::compute_invoke_entries(model);

    // Event tree for sealed interface hierarchy
    let kotlin_events: std::collections::BTreeSet<String> = model
        .events
        .iter()
        .filter(|e| e.as_str() != "Wildcard")
        .cloned()
        .collect();
    let event_tree = kotlin::build_event_tree(&kotlin_events);
    let leaf_events = kotlin::collect_leaf_events(&event_tree, "");

    // Pre-render event tree as Kotlin sealed interfaces
    let event_members =
        kotlin::render_event_tree(&event_tree, &format!("{machine_name}Event"), "    ");

    // SCE Forge: render inline kind declarations as Kotlin code fragment.
    let inline_kind_code = if !model.inline_kinds.is_empty() {
        crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Kotlin,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?
        .member_fns
    } else {
        String::new()
    };

    let tmpl = env
        .get_template("state_machine.kt.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;

    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        machine_name => machine_name,
        event_tree => minijinja::Value::from_serialize(&event_tree),
        event_members => event_members,
        leaf_events => minijinja::Value::from_serialize(&leaf_events),
        license_config => minijinja::Value::from_serialize(license_config()),
        kotlin_package_prefix => package_prefix.unwrap_or("com.sce.generated"),
        initial_entry_root => initial_entry_root,
        ancestor_chains => minijinja::Value::from_serialize(&ancestor_chains),
        effective_transitions => minijinja::Value::from_serialize(&effective_transitions),
        parent_map => minijinja::Value::from_serialize(&parent_map),
        leaf_map => minijinja::Value::from_serialize(&leaf_map),
        parallel_descendants => minijinja::Value::from_serialize(&parallel_descendants),
        deep_initial_entries => minijinja::Value::from_serialize(&deep_initial_entries),
        invoke_entries => minijinja::Value::from_serialize(&invoke_entries),
        ancestors_with_event_transitions => minijinja::Value::from_serialize(&ancestors_with_event_transitions),
        ancestors_with_null_transitions => minijinja::Value::from_serialize(&ancestors_with_null_transitions),
        process_event_needs_else => process_event_needs_else,
        process_null_event_needs_else => process_null_event_needs_else,
        transition_actions_needs_else => transition_actions_needs_else,
        inline_kind_code => &inline_kind_code,
        event_payload_active => payload.active,
        event_payload_defs => &payload.defs,
        event_payload_policy_fields => &payload.policy_fields,
        event_payload_populate => &payload.populate,
        event_payload_inject => &payload.inject_methods,
        has_native_actions => native.any,
        native_actions_defs => &native.interface_def,
        native_actions_interface => &native.interface_name,
    };

    let output = tmpl.render(ctx).map_err(render_error)?;
    // Template leaves class body open; we close it here (implicit contract with state_machine.kt.jinja2)
    Ok(output.trim_end().to_string() + "\n}\n")
}

// ── Go generator ────────────────────────────────────────────────

/// Generate Go code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_go(model: &SCXMLModel, template_dir: &Path) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Go)?;
    // Neither host refusal here any more: the Go runtime carries
    // `Engine.RegisterEventProcessor` for §scxml-6.2.5 and
    // `Engine.RegisterInvoker` for §scxml-6.4.1, and the templates emit the
    // `<send>` dispatch and the `<invoke>` start/cancel pair into them. The
    // invoker half is the wider of the two — an invocation has a lifetime the
    // exit chain has to end — which is why it landed second.
    reject_native_conditions_in_unsupported_lang(model, "Go")?;
    reject_native_scripts_in_unsupported_lang(model, "Go")?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_go_filters(&mut env, &document_scope(model));
    render_go(&mut env, model)
}

/// Generate Go code using pre-loaded template strings (WASM-compatible).
pub fn generate_go_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Go)?;
    // See `generate_go` above: the Go backend carries both host registries.
    reject_native_conditions_in_unsupported_lang(model, "Go")?;
    reject_native_scripts_in_unsupported_lang(model, "Go")?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_go_filters(&mut env, &document_scope(model));
    render_go(&mut env, model)
}

// ── Python generator ────────────────────────────────────────────
//
// Python AOT backend: atomic + compound + parallel + history states,
// basic + eventless transitions, onentry/onexit, transition guards and
// actions, `<data>` early-binding datamodel with `<assign>` updates,
// `<raise>` for internal events, `<send>`/`<cancel>`, and
// `<invoke type="scxml">`. Generated `*_sm.py`
// modules depend on `sce-python-runtime` (pure-Python W3C SCXML engine) —
// analogous to how the Go backend depends on `sce-go-runtime` and Kotlin on
// its runtime package. The pybind11 channel under `sce-python/` is a
// separate (interpreter-mode) integration and is not used here.

/// Generate Python code from an analyzed SCXMLModel (filesystem-based).
pub fn generate_python(model: &SCXMLModel, template_dir: &Path) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Python)?;
    // Neither host refusal here: the Python runtime carries
    // `Engine.register_event_processor` for §scxml-6.2.5 and
    // `Engine.register_invoker` for §scxml-6.4.1, and the templates emit the
    // `<send>` dispatch and the `<invoke>` start/cancel pair into them.
    reject_native_conditions_in_unsupported_lang(model, "Python")?;
    reject_native_scripts_in_unsupported_lang(model, "Python")?;
    reject_python_unsupported_features(model)?;
    let mut env = new_env();
    load_templates(&mut env, template_dir)?;
    filters::register_python_filters(&mut env, &document_scope(model));
    render_python(&mut env, model)
}

/// Generate Python code using pre-loaded template strings (WASM-compatible).
pub fn generate_python_with_templates(
    model: &SCXMLModel,
    templates: &[(&str, &str)],
) -> Result<String, GenerateError> {
    reject_mesh_rpc_in_unsupported_lang(model, Language::Python)?;
    // See `generate_python` above: the Python backend carries both host
    // registries.
    reject_native_conditions_in_unsupported_lang(model, "Python")?;
    reject_native_scripts_in_unsupported_lang(model, "Python")?;
    reject_python_unsupported_features(model)?;
    let mut env = new_env();
    load_template_strings(&mut env, templates)?;
    filters::register_python_filters(&mut env, &document_scope(model));
    render_python(&mut env, model)
}

/// Explicitly reject features the Python codegen does not implement.
/// Failing loudly here keeps generated `*_sm.py` honest: every accepted
/// document produces a working module instead of a silently degraded
/// one. `<parallel>`, `<history>`, the executable-content set, and
/// `<invoke type="scxml">` are all accepted; the rejects below cover
/// mesh-rpc invokes, action elements outside the supported set, and
/// `<send>` forms with no supported transport or event identifier.
fn reject_python_unsupported_features(model: &SCXMLModel) -> Result<(), GenerateError> {
    // §scxml-6.4: `<invoke type="scxml">` (static src=/inline) and
    // `<invoke srcexpr/contentexpr>` (hybrid) both lower the same way
    // now — the hybrid stub written by `generate_hybrid_child_scxmls`
    // produces a child policy whose immediate `<final>` raises
    // `done.invoke.<id>` so W3C 6.4.3 / 6.4.4 fixtures observe the
    // expected event regardless of what the srcexpr/contentexpr would
    // resolve to. Mesh-rpc invokes remain permanently rejected per
    // the C++-first mesh policy (`mesh_cpp_first_policy.md`).
    for inv in &model.invokes {
        match inv {
            crate::model::Invoke::Scxml(_)
            | crate::model::Invoke::Hybrid(_)
            | crate::model::Invoke::Unsupported(_) => {}
            crate::model::Invoke::MeshRpc(_) => {
                return Err(GenerateError::InvalidConfig(
                    "Python codegen rejects <invoke type=\"sce:mesh-rpc\">: \
                     mesh runtime is C++ alone (mesh_cpp_first_policy)"
                        .into(),
                ));
            }
        }
    }
    // §scxml-5.3 — Python AOT used to reject `<data id>` names that
    // collide with Python keywords (`class`, `lambda`, …) because the
    // pre-Lua datamodel stored values as bare Python identifiers parsed
    // by `eval` directly. Post-Lua-migration the datamodel lives inside
    // the `IScriptEngine` session and is accessed exclusively via
    // string-keyed `declare_variable` / `set_variable` / `<assign
    // location>` calls; user expressions go through the
    // ECMAScript→Lua transformer (`to_lua_expr` / `to_lua_guard` /
    // `to_lua_script`) so the SCXML author's identifier never reaches
    // a Python parser. No keyword reject is needed.
    // §scxml-3.13 — `<transition event="*">` matches every external
    // event except the eventless NULL sentinel; the codegen lowers it to
    // `if event != Event.NULL` in `process_transition.py.jinja2`. Prefix
    // (`event="foo.*"`) and multi-event (`event="foo bar"`) descriptors
    // lower through the `_event_name_matches` helper in the same
    // template (W3C 3.13 token-prefix match); no reject is needed.
    // Dynamic `<send>` expressions are accepted: eventexpr / delayexpr
    // evaluate against the datamodel at action time, payload
    // marshalling runs through `_eval_send_payload`, and `idlocation`
    // is written back by `_resolve_send_id`. Only `<send target>`
    // values outside the supported transport set (`#_…`, the `!`
    // sentinel, `http(s)://…`) are reject-walled below.
    fn check_actions(actions: &[crate::model::Action], context: &str) -> Result<(), GenerateError> {
        // §scxml-G-7 `native_action` is on this list because the Python
        // backend now lowers `<sce:action>` to a direct call on the generated
        // `<Machine>Actions` Protocol — see `_actions.py.jinja2`'s dispatch
        // arm. It is listed here rather than special-cased ahead of the walk
        // because this list is what the walk consults, and a construct the
        // emitter handles but the list omits is refused for a reason that
        // stopped being true.
        const SUPPORTED_ACTIONS: &[&str] = &[
            "script",
            "assign",
            "raise",
            "log",
            "if",
            "foreach",
            "send",
            "cancel",
            "native_action",
        ];
        for action in actions {
            if !SUPPORTED_ACTIONS.contains(&action.action_type.as_str()) {
                return Err(GenerateError::InvalidConfig(format!(
                    "Python codegen does not yet support <{}> in {}; deferred to Atomic γ",
                    action.action_type, context
                )));
            }
            if action.action_type == "send" {
                // Accepted `<send>` target forms: the simple in-machine
                // form (empty target) with dynamic exprs + payload +
                // idlocation; `target="#_parent"` (child-to-parent) and
                // `target="#_<invoke_id>"` (parent-to-child via
                // `Invoke.forward_event`); `target="!…"` (the SCXML
                // test-suite's deliberate-invalid sentinel — W3C 6.2:
                // dispatch failure raises `error.execution`); and
                // absolute http:// / https:// targets +
                // `targetexpr` (runtime URL resolution) +
                // `send_type="BasicHTTPEventProcessor"` (W3C C.2). All
                // three lower through `engine.perform_http_send`.
                if !action.target.is_empty()
                    && !action.target.starts_with("#_")
                    && !action.target.starts_with('!')
                    && !action.target.starts_with("http://")
                    && !action.target.starts_with("https://")
                {
                    return Err(GenerateError::InvalidConfig(format!(
                        "Python codegen `<send target=\"{}\">` in {} is not a supported \
                         transport (expected `#_internal`, `#_parent`, `#_<invoke>`, \
                         `!sentinel`, or `http(s)://...`)",
                        action.target, context
                    )));
                }
                // `targetexpr` resolves at action time to one of the
                // supported transport schemas; the codegen emits a runtime
                // dispatch table covering `#_internal` / `#_parent` /
                // `#_<invoke>` / `!sentinel` / `http(s)://...` (W3C C.1).
                // Empty / undefined resolutions raise `error.communication`.
                // The codegen accepts `<send eventexpr>`,
                // `<send delayexpr>`, `<send idlocation>`, and
                // `<param>` / `<content>` / namelist payload
                // marshalling — those rejects are intentionally absent.
                //
                // HTTP-targeted sends (`target="http(s)://…"` or
                // `type=BasicHTTPEventProcessor`) carry the event name
                // in the form payload (`_scxmleventname` param) or
                // default to `HTTP.POST` (W3C C.2). The `event` /
                // `eventexpr` attribute is therefore optional for the
                // HTTP transport; the reject applies only to
                // SCXML-internal/external dispatches that need a
                // static or computed event identifier.
                let is_http_send = action.target.starts_with("http://")
                    || action.target.starts_with("https://")
                    || action.send_type == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";
                if action.event.is_empty() && action.eventexpr.is_empty() && !is_http_send {
                    return Err(GenerateError::InvalidConfig(format!(
                        "Python codegen `<send>` in {} requires `event` or `eventexpr`",
                        context
                    )));
                }
            }
            // Walk into <if>/<foreach> bodies so a nested unsupported
            // action (e.g. <send> inside an <if>) is rejected at the
            // same fail-loud surface as a top-level <send>.
            if action.action_type == "if" {
                check_actions(&action.then_actions, context)?;
                for branch in &action.elseif_branches {
                    check_actions(&branch.actions, context)?;
                }
                check_actions(&action.else_actions, context)?;
            } else if action.action_type == "foreach" {
                check_actions(&action.actions, context)?;
            }
        }
        Ok(())
    }
    for (state_id, state) in &model.states {
        for block in &state.on_entry_blocks {
            check_actions(block, &format!("onentry of `{state_id}`"))?;
        }
        for block in &state.on_exit_blocks {
            check_actions(block, &format!("onexit of `{state_id}`"))?;
        }
        for transition in &state.transitions {
            check_actions(
                &transition.actions,
                &format!("transition from `{state_id}`"),
            )?;
        }
    }
    Ok(())
}

fn render_python(env: &mut Environment, model: &SCXMLModel) -> Result<String, GenerateError> {
    let machine_name = filters::to_pascal_case(model.name.clone());

    // EventSchema native lowering — the Python typed
    // `_event.data` payload channel: module-level payload dataclasses, the
    // `_pending_<event>_payload` __init__ fields + `set_current_event` lift that
    // carries the dequeued event's typed payload, the per-event `raise_<event>`
    // inject seams, and the per-transition native guard (`self._pending_<event>
    // _payload is not None and (…)`). The per-machine defs ride in the render
    // context; the per-transition guards ride home on the transition's
    // `native_payload_guard` via a single-language clone (same SSOT selection
    // as every backend).
    let mut model_lowered = model.clone();
    // Stamp each transition with the
    // symbol identity the sourcemap keys off, before any analysis pass
    // clones or serialises the transitions.
    symbol_mangling::stamp_symbol_attribution(&mut model_lowered);
    // §scxml-G-7: lower `<sce:action>` Custom Action Elements to native host
    // dispatch (engine-free) — same call every backend makes.
    let native =
        crate::forge::native_action::render(&mut model_lowered, &machine_name, Language::Python);
    let payload =
        crate::forge::generator::build_python_event_payload(model, &native.payload_events);
    crate::forge::generator::apply_native_guard_writes(&mut model_lowered, &payload.guard_writes);
    let model = &model_lowered;

    let tmpl = env
        .get_template("state_machine.py.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        machine_name => machine_name,
        license_config => minijinja::Value::from_serialize(license_config()),
        event_payload_active => payload.active,
        event_payload_defs => &payload.defs,
        event_payload_init => &payload.init,
        event_payload_populate => &payload.populate,
        event_payload_inject => &payload.inject,
        has_native_actions => native.any,
        native_actions_defs => &native.interface_def,
        native_actions_interface => &native.interface_name,
    };
    tmpl.render(ctx).map_err(render_error)
}

// ── Go generator ────────────────────────────────────────────────

fn render_go(env: &mut Environment, model: &SCXMLModel) -> Result<String, GenerateError> {
    let machine_name = filters::to_pascal_case(model.name.clone());

    // SCE Forge: render inline kind declarations as Go code fragments.
    let (inline_kind_types, inline_kind_fns) = if !model.inline_kinds.is_empty() {
        let code = crate::forge::generator::render_inline_kinds(
            &model.inline_kinds,
            Language::Go,
            &machine_name,
        )
        .map_err(|e| GenerateError::TemplateRender(e.to_string()))?;
        (code.type_defs, code.member_fns)
    } else {
        (String::new(), String::new())
    };

    // EventSchema MCU native lowering — the Go typed
    // `_event.data` payload channel: a tag enum + per-event payload structs,
    // the policy fields / populate type-switch that lift the dequeued event's
    // typed payload, the per-event `Raise<Event>` inject seams, and the
    // per-transition native guard (`p.pendingPayloadTag == … && (…)`). The
    // per-machine defs ride in the render context; the per-transition guards
    // ride home on the transition's `native_payload_guard` via a
    // single-language clone (same SSOT guard selection as every backend).
    let mut model_lowered = model.clone();
    // Stamp each transition with the
    // symbol identity the sourcemap keys off, before any analysis pass
    // clones or serialises the transitions.
    symbol_mangling::stamp_symbol_attribution(&mut model_lowered);
    // §scxml-G-7: lower `<sce:action>` Custom Action Elements to native host
    // dispatch (engine-free). Same call as every other backend makes — the
    // rendered call site lands on each action, the interface and the
    // payload-event union come back. Empty everywhere when the document has no
    // native actions, so the emitted code is byte-identical to the
    // pre-feature baseline.
    let native =
        crate::forge::native_action::render(&mut model_lowered, &machine_name, Language::Go);
    let payload = crate::forge::generator::build_go_event_payload(model, &native.payload_events);
    crate::forge::generator::apply_native_guard_writes(&mut model_lowered, &payload.guard_writes);
    let model = &model_lowered;

    let tmpl = env
        .get_template("state_machine.go.jinja2")
        .map_err(|e| GenerateError::TemplateLoad(format!("Template load error: {e}")))?;
    let ctx = minijinja::context! {
        model => minijinja::Value::from_serialize(model),
        machine_name => machine_name,
        license_config => minijinja::Value::from_serialize(license_config()),
        inline_kind_types => &inline_kind_types,
        inline_kind_fns => &inline_kind_fns,
        event_payload_defs => &payload.defs,
        event_payload_active => payload.active,
        event_payload_policy_fields => &payload.policy_fields,
        event_payload_populate => &payload.populate,
        event_payload_clear => &payload.clear,
        has_native_actions => native.any,
        native_actions_defs => &native.interface_def,
        native_actions_interface => &native.interface_name,
    };
    tmpl.render(ctx).map_err(render_error)
}

// ── Template loading helpers ─────────────────────────────────────

/// Load templates from pre-loaded string pairs (WASM-compatible).
fn load_template_strings(
    env: &mut Environment<'_>,
    templates: &[(&str, &str)],
) -> Result<(), GenerateError> {
    for (name, content) in templates {
        env.add_template_owned(name.to_string(), content.to_string())
            .map_err(|e| {
                GenerateError::TemplateLoad(format!("Template parse error in {name}: {e}"))
            })?;
    }
    Ok(())
}

/// Recursively load all .jinja2 templates from a directory.
///
/// SCE Protocol-Synthesis RFC §synth-5-O (generated-source traceability) — also loads the workspace-
/// shared `_macros/` directory (one level up from the per-backend
/// template root) so cross-backend shared macros like
/// `_macros/sce_map_marker.jinja2` are visible to every language
/// that calls `find_template_dir_for`. Cpp / C11 already pass the
/// template root (their `subdir = ""`); Rust / Kotlin / Go / Python
/// pass a per-language subdir, so without the shared-macro loader
/// they would lose access to the cross-backend macro family. The
/// shared load skips silently when `_macros/` is absent (vendored
/// builds without the macro tree).
pub fn load_templates(env: &mut Environment<'_>, dir: &Path) -> Result<(), GenerateError> {
    if !dir.exists() {
        return Err(GenerateError::TemplateLoad(format!(
            "Template directory not found: {}",
            dir.display()
        )));
    }
    load_templates_recursive(env, dir, dir)?;
    if let Some((macro_base, macro_dir)) = shared_macro_dir(dir) {
        // base_dir = the parent so loaded names start with `_macros/...`
        // — matching the path callers use in
        // `{% import "_macros/sce_map_marker.jinja2" as sce_map %}`.
        load_templates_recursive(env, &macro_base, &macro_dir)?;
    }
    Ok(())
}

/// Where the workspace-shared `_macros/` tree sits relative to `dir`, as
/// `(base_dir, macro_dir)`.
///
/// `_macros/` lives at `<workspace>/tools/codegen/templates/_macros/`.
/// For per-backend roots like `rust/` that is one level up; for per-kind
/// forge roots like `forge/rust/`, two. The parent chain is walked until
/// it appears, so adding a third-level template tree later does not
/// regress the inheritance. `None` when the tree is absent (vendored
/// builds without the macro family).
fn shared_macro_dir(dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut current = dir;
    while let Some(parent) = current.parent() {
        let shared_macros = parent.join("_macros");
        if shared_macros.is_dir() && shared_macros != dir.join("_macros") {
            return Some((parent.to_path_buf(), shared_macros));
        }
        current = parent;
    }
    None
}

/// Every template file [`load_templates`] would register for `dir`, as
/// `(registered name, path on disk)`.
///
/// Exists so a depfile can state what the render could read without
/// re-deriving the loader's scope. Deriving it separately is not a
/// stylistic difference: the depfile writer used to walk `dir` alone,
/// which silently dropped the shared `_macros/` family for every
/// backend whose scope is a subdirectory (rust / kotlin / go / python).
/// Editing `_macros/sce_map_marker.jinja2` therefore left their output
/// stale while the build reported success.
///
/// The name is the same string [`load_templates`] hands to
/// `add_template_owned` — root-relative, `/`-separated, and the spelling
/// a template's own `{% import %}` uses. It is returned rather than left
/// for the caller to reconstruct because reconstruction is what broke:
/// the depfile writer matched `Language::foreign_template_prefixes`,
/// which are relative by definition, against each template's *absolute*
/// path. Under a checkout whose prefix contains a backend's directory
/// name (`/home/go/…`, `/srv/c/…`) every template matched as foreign and
/// the depfile came out empty, on all six backends and both pipelines.
///
/// Order is deterministic, so a depfile does not churn between runs.
pub fn loader_template_files(dir: &Path) -> Vec<(String, PathBuf)> {
    fn collect(base: &Path, current: &Path, out: &mut Vec<(String, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(current) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(base, &path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("jinja2") {
                let Ok(rel) = path.strip_prefix(base) else {
                    continue;
                };
                out.push((rel.to_string_lossy().replace('\\', "/"), path));
            }
        }
    }

    let mut found = Vec::new();
    if dir.exists() {
        collect(dir, dir, &mut found);
        if let Some((macro_base, macro_dir)) = shared_macro_dir(dir) {
            // Named from the parent, so entries read `_macros/…` exactly
            // as `load_templates` registers them and as the importing
            // templates spell them.
            collect(&macro_base, &macro_dir, &mut found);
        }
    }
    found.sort();
    found.dedup();
    found
}

fn load_templates_recursive(
    env: &mut Environment<'_>,
    base_dir: &Path,
    current_dir: &Path,
) -> Result<(), GenerateError> {
    let entries = std::fs::read_dir(current_dir).map_err(|e| {
        GenerateError::TemplateLoad(format!("Cannot read {}: {e}", current_dir.display()))
    })?;

    for entry in entries {
        let entry =
            entry.map_err(|e| GenerateError::TemplateLoad(format!("Dir entry error: {e}")))?;
        let path = entry.path();
        if path.is_dir() {
            load_templates_recursive(env, base_dir, &path)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("jinja2") {
            let rel = path
                .strip_prefix(base_dir)
                .map_err(|e| GenerateError::TemplateLoad(format!("Path error: {e}")))?;
            let template_name = rel.to_string_lossy().replace('\\', "/");
            let content = std::fs::read_to_string(&path).map_err(|e| {
                GenerateError::TemplateLoad(format!("Cannot read template {}: {e}", path.display()))
            })?;
            env.add_template_owned(template_name, content)
                .map_err(|e| {
                    GenerateError::TemplateLoad(format!(
                        "Template parse error in {}: {e}",
                        path.display()
                    ))
                })?;
        }
    }
    Ok(())
}

// ── C++ post-processing ────────────────────────────────────────
//
// Responsibility: structural corrections that templates cannot express
// (dedent, include sort, blank-line collapse, orphaned-line re-indent).
//
// Style-level formatting (pointer alignment, line wrapping, macro alignment,
// brace insertion) is delegated to clang-format via the CMake build system.
// This keeps a clean separation: codegen → structure → style.

/// Post-process generated C++ header (.h) to match clang-format style.
fn postprocess_cpp_header(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let mut out = Vec::with_capacity(lines.len());

    // Sort include blocks and fix preprocessor indentation.
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Sort contiguous #include blocks alphabetically.
        if line.trim_start().starts_with("#include") {
            let mut include_block = vec![line.to_string()];
            i += 1;
            while i < lines.len() && lines[i].trim_start().starts_with("#include") {
                include_block.push(lines[i].to_string());
                i += 1;
            }
            include_block.sort();
            for inc in include_block {
                out.push(inc);
            }
            continue;
        }

        // Strip indent inside #ifndef preprocessor guards.
        if line.starts_with("    #define ") || line.starts_with("    #define\t") {
            out.push(line.trim_start().to_string());
            i += 1;
            continue;
        }
        if line == "    // Debug logging disabled in release builds" {
            out.push(line.trim_start().to_string());
            i += 1;
            continue;
        }

        // Namespace closing: `} //` → `}  //`
        if line.starts_with("} // namespace") {
            out.push(line.replacen("} // ", "}  // ", 1));
            i += 1;
            continue;
        }

        out.push(line.to_string());
        i += 1;
    }

    // Collapse consecutive blank lines, remove trailing blanks.
    collapse_blank_lines(&out)
}

/// Collapse consecutive blank lines to single blank line, trim trailing.
fn collapse_blank_lines(lines: &[String]) -> String {
    let mut result = String::new();
    let mut prev_blank = false;
    for line in lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        result.push_str(line);
        result.push('\n');
    }
    let trimmed = result.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    format!("{trimmed}\n")
}

/// Post-process generated C++ .inl file to match the project clang-format style.
///
/// The .inl file is `#include`d inside a struct body, so templates produce code
/// with a 4-space base indent. Additionally, action templates (raise, script, etc.)
/// emit code at column 0 regardless of their nesting context (Jinja2 limitation).
///
/// This function:
/// 1. Strips the 4-space base indent from all lines (the template's struct-level indent).
/// 2. Re-indents orphaned lines (lines at 0 indent inside a nested block) by
///    tracking brace depth — a lightweight structural re-indenter.
/// 3. Collapses consecutive blank lines to one.
fn postprocess_cpp_inl(code: &str) -> String {
    // The .inl template uses 4-space indent for all top-level code.
    const BASE_INDENT_STR: &str = "    ";

    let mut lines: Vec<String> = Vec::new();
    let mut brace_depth: i32 = 0;

    for raw_line in code.lines() {
        let is_blank = raw_line.trim().is_empty();
        if is_blank {
            lines.push(String::new());
            continue;
        }

        // Strip the base indent from lines that have it.
        //
        // `strip_prefix` on the literal indent, not `raw_line[..BASE_INDENT]`:
        // `len()` and range indexing are BYTE-based, so a line whose first
        // non-space character is multi-byte (`// §scxml-6.4.1 …` puts `§` at
        // bytes 3..5) made the slice land inside a char and panic. Comment text
        // is author-controlled and now carries `§` citations, so any
        // fixed-width byte slice here is a latent crash; prefix-stripping is
        // both panic-free and exactly the intended operation.
        let line = match raw_line.strip_prefix(BASE_INDENT_STR) {
            Some(rest) => rest.to_string(),
            // Fewer leading spaces than the base indent (e.g. orphaned action
            // code at col 0). Re-indented below from brace depth.
            None => raw_line.to_string(),
        };

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            lines.push(String::new());
            continue;
        }

        // Count braces to track nesting depth, excluding braces in string
        // literals and comments.
        let opens: i32 = count_braces(&trimmed, '{');
        let closes: i32 = count_braces(&trimmed, '}');

        // A line starting with '}' reduces depth BEFORE indentation.
        let effective_depth = if trimmed.starts_with('}') {
            (brace_depth - closes + opens).max(0)
        } else {
            brace_depth
        };

        // The line's own indent (after base stripping).
        let line_indent = line.len() - trimmed.len();

        // If this line is at 0 indent but should be deeper (orphaned action code),
        // re-indent it to match the current brace depth.
        let output_line = if line_indent == 0 && effective_depth > 0 {
            let indent_str: String = " ".repeat(effective_depth as usize * 4);
            format!("{indent_str}{trimmed}")
        } else {
            line
        };

        lines.push(output_line);

        // Update brace depth for the NEXT line.
        if trimmed.starts_with('}') {
            brace_depth = (brace_depth - closes + opens).max(0);
        } else {
            brace_depth = (brace_depth + opens - closes).max(0);
        }
    }

    collapse_blank_lines(&lines)
}

/// Count occurrences of a brace character, skipping string literals and comments.
fn count_braces(line: &str, brace: char) -> i32 {
    let mut count = 0i32;
    let mut in_string = false;
    let mut in_char = false;
    let mut prev = '\0';
    let bytes = line.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        let c = b as char;
        if in_string {
            if c == '"' && prev != '\\' {
                in_string = false;
            }
        } else if in_char {
            if c == '\'' && prev != '\\' {
                in_char = false;
            }
        } else {
            // Check for line comment
            if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                break; // Rest of line is comment
            }
            if c == '"' {
                in_string = true;
            } else if c == '\'' {
                in_char = true;
            } else if c == brace {
                count += 1;
            }
        }
        prev = c;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    /// Prints what the seam still owes, so the migration has a number that
    /// comes from the tree rather than from a document.
    #[test]
    fn report_unmigrated_expression_sites() {
        let sites = Language::Cpp.unmigrated_expression_sites();
        eprintln!("cpp unmigrated expression sites: {}", sites.len());
        for site in &sites {
            eprintln!("  {site}");
        }
        let unknown = Language::Cpp.unclassified_expression_sites();
        eprintln!("cpp unclassified (callee unknown): {}", unknown.len());
        for site in &unknown {
            eprintln!("  ? {site}");
        }
    }

    #[test]
    fn trailing_newline_is_added_only_when_missing() {
        // The defect: a C11 header ends at its include guard.
        assert_eq!(
            with_trailing_newline("#endif  /* SCE_GUARD_H */"),
            "#endif  /* SCE_GUARD_H */\n",
        );
        // Already conformant input is returned untouched, so backends
        // whose formatter already guarantees this produce byte-identical
        // output and their committed trees do not churn.
        assert!(matches!(
            with_trailing_newline("int main(void) { return 0; }\n"),
            std::borrow::Cow::Borrowed(_),
        ));
        // A deliberate blank final line is preserved, not collapsed.
        assert_eq!(with_trailing_newline("body\n\n"), "body\n\n");
        // A zero-byte artefact is a valid text file; adding a newline
        // would turn "nothing was emitted" into a one-line file.
        assert_eq!(with_trailing_newline(""), "");
        // Carriage returns are not newlines: a CRLF file already ends
        // with `\n`, a lone `\r` does not.
        assert_eq!(with_trailing_newline("line\r\n"), "line\r\n");
        assert_eq!(with_trailing_newline("line\r"), "line\r\n");
    }

    /// Document with a single `<invoke type="sce:mesh-rpc">` site —
    /// triggers `model.has_mesh_rpc_invoke()` and exercises the
    /// rejection path on backends without mesh codegen.
    const MESH_RPC_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" name="brake" initial="idle">
  <state id="idle">
    <invoke type="sce:mesh-rpc" src="#motor">
      <param name="_mesh_event" expr="'service.request.compute_force'"/>
    </invoke>
  </state>
</scxml>"##;

    /// §scxml-6.4.1: `type` names no processor SCE implements. The spec
    /// defines the case (raise `error.execution`), so the document parses
    /// and carries an [`crate::model::Invoke::Unsupported`].
    const UNSUPPORTED_INVOKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="brake" initial="idle">
  <state id="idle">
    <invoke type="urn:sce:test:no-such-processor"/>
    <transition event="error.execution" target="done"/>
  </state>
  <final id="done"/>
</scxml>"##;

    fn parse(content: &str) -> SCXMLModel {
        let mut p = SCXMLParser::new();
        p.parse_string(content, "brake").expect("parse")
    }

    /// The parser must keep the `<invoke>` rather than dropping it. This is
    /// the guard on the original defect: `parse_invoke` used to answer
    /// `Ok(None)` for an unsupported type, so the element vanished from the
    /// model and every backend emitted a machine in which the `<invoke>`
    /// simply did not exist.
    #[test]
    fn unsupported_invoke_type_survives_parsing() {
        let model = parse(UNSUPPORTED_INVOKE_SCXML);
        assert!(
            model.has_unsupported_invoke(),
            "an unsupported <invoke type> must reach the model, not be dropped"
        );
        assert!(
            model.events.contains("error.execution"),
            "the analyzer must register error.execution so the generated enum \
             resolves Event::Error_execution without an author handler"
        );
    }

    /// §scxml-6.4.1 on the one backend that lowers it: the emitted machine
    /// must contain the raise. A backend that "accepts" the document while
    /// emitting nothing is the same silent drop in a new place.
    #[test]
    fn cpp_generate_emits_error_execution_for_unsupported_invoke() {
        let model = parse(UNSUPPORTED_INVOKE_SCXML);
        // Real templates: the sibling reject tests can pass `&[]` because
        // they fail before any template loads, but an *emission* assertion
        // has to render the actual `.jinja2` sources.
        let templates = crate::find_template_base();
        match generate_cpp(&model, &templates, "fixture", None) {
            Err(e) => panic!("C++ must lower the §6.4.1 raise, got {e:?}"),
            Ok(out) => {
                let joined: String = out.files.iter().map(|(_, c)| c.as_str()).collect();
                assert!(
                    joined.contains(
                        "<invoke type='urn:sce:test:no-such-processor'> names an \
                         external service this platform does not support"
                    ),
                    "generated C++ must raise error.execution naming the refused type, \
                     in the one wording all seven emitters share"
                );
                assert!(
                    joined.contains("deferInvoke"),
                    "the raise must ride the §scxml-6.4 defer queue, not fire from onentry"
                );
            }
        }
    }

    #[test]
    fn rust_generate_emits_error_execution_for_unsupported_invoke() {
        let model = parse(UNSUPPORTED_INVOKE_SCXML);
        // The Rust generator loads from the per-language subdirectory,
        // unlike `generate_cpp` which takes the shared base.
        let templates = crate::find_template_base().join("rust");
        let joined = generate(&model, &templates, false).expect("Rust must lower the §6.4.1 raise");
        assert!(
            joined.contains("defer_invoke"),
            "the raise must ride the §scxml-6.4 defer queue, not fire from onentry"
        );
        assert!(
            joined.contains("6.4.1: unsupported `type`"),
            "generated Rust must carry the §6.4.1 raise at the execute step"
        );
    }

    /// Every backend must LOWER the §6.4.1 raise, not merely accept the
    /// document. A backend that renders without the raise reproduces the
    /// original defect one layer down — measured directly during this work:
    /// wiring C++ alone left five backends emitting zero raise sites while
    /// reporting success.
    #[test]
    fn every_backend_lowers_error_execution_for_unsupported_invoke() {
        let model = parse(UNSUPPORTED_INVOKE_SCXML);
        let base = crate::find_template_base();

        let kt = generate_kotlin(&model, &base.join("kotlin"), None).expect("kotlin");
        assert!(kt.contains("deferInvoke"), "kotlin defers");
        assert!(
            kt.contains("Error.Execution"),
            "kotlin raises error.execution"
        );

        let go_src = generate_go(&model, &base.join("go")).expect("go");
        assert!(go_src.contains("DeferInvoke"), "go defers");
        assert!(go_src.contains("6.4.1: unsupported `type`"), "go raises");

        let py_src = generate_python(&model, &base.join("python")).expect("python");
        assert!(py_src.contains("_pending_invokes.append"), "python defers");
        assert!(
            py_src.contains("6.4.1: unsupported `type`"),
            "python raises"
        );

        // C11's transition dispatch reads `prefix_matching_events`, which the
        // analyzer populates; the CLI runs it, `parse()` alone does not.
        let mut c11_model = parse(UNSUPPORTED_INVOKE_SCXML);
        crate::analyzer::analyze(&mut c11_model, "unsupported_invoke.scxml");
        let c11 = generate_c11(&c11_model, &base, "fixture", None).expect("c11");
        let c_src: String = c11.files.iter().map(|(_, c)| c.as_str()).collect();
        assert!(c_src.contains("sce_invoke_pending_push"), "c11 defers");
        assert!(c_src.contains("6.4.1: unsupported `type`"), "c11 raises");
    }

    /// `docs/SCE_ACCEPTED_SUBSET.md` §2.12 is the surface a consumer reads to
    /// learn which engines honour the §6.4.1 raise, and nothing was reading
    /// it. The table shipped claiming five AOT backends refuse the document
    /// while the generator lowered the raise on all six, and every suite
    /// stayed green: a doc-as-contract with no test is prose.
    #[test]
    fn accepted_subset_2_12_documents_every_lowering_backend() {
        const DOC: &str = include_str!("../../docs/SCE_ACCEPTED_SUBSET.md");
        const ALL: [Language; 6] = [
            Language::Cpp,
            Language::Rust,
            Language::Kotlin,
            Language::Go,
            Language::Python,
            Language::C11,
        ];

        // The label §2.12's AOT row uses for a backend. Exhaustive on
        // purpose: a seventh backend does not compile here until someone
        // decides what the accepted-subset table calls it.
        fn label(language: Language) -> &'static str {
            match language {
                Language::Cpp => "C++",
                Language::Rust => "Rust",
                Language::Kotlin => "Kotlin",
                Language::Go => "Go",
                Language::Python => "Python",
                Language::C11 => "C11",
            }
        }

        let section = DOC
            .split("### §2.12 ")
            .nth(1)
            .expect("§2.12 heading present in the accepted-subset doc")
            .split("\n### ")
            .next()
            .expect("§2.12 section body");

        let aot_rows: Vec<&str> = section
            .lines()
            .filter(|l| l.trim_start().starts_with("| AOT"))
            .collect();
        assert_eq!(
            aot_rows.len(),
            1,
            "§2.12 must describe one AOT behaviour rather than splitting the \
             backends across rows — they all lower the same raise. Rows: {aot_rows:?}"
        );

        for language in ALL {
            assert!(
                aot_rows[0].contains(label(language)),
                "§2.12's AOT row does not name {}, yet the generator lowers the \
                 §6.4.1 raise there (see \
                 `every_backend_lowers_error_execution_for_unsupported_invoke`). Row: {}",
                label(language),
                aot_rows[0]
            );
        }

        assert!(
            !section.contains("unsupported-feature"),
            "§2.12 still documents a `generate/unsupported-feature` refusal. \
             §6.4.1 assigns the construct a meaning instead of declaring it \
             malformed, so refusing it at build time refuses valid SCXML"
        );
    }

    /// `postprocess_cpp_inl` must not index a line by byte offset.
    ///
    /// The base-indent strip used `raw_line[..4]` while `4` counted BYTES, so a
    /// line whose 4th byte fell inside a multi-byte character panicked with
    /// "not a char boundary". `§scxml-<id>` citations put `§` at bytes 3..5 of
    /// `// §…`, and templates carry those citations, so this crashed real
    /// codegen rather than a synthetic input. Each case below has a multi-byte
    /// character straddling the byte-4 boundary at a different indent depth.
    #[test]
    fn postprocess_cpp_inl_handles_multibyte_at_indent_boundary() {
        for line in [
            "// §scxml-6.4.1 autoforward",    // § at bytes 3..5, no base indent
            "    // §scxml-3.13 transitions", // base indent present, § later
            "/* §scxml-5.10 */",              // § at bytes 3..5, block comment
            "   §scxml-4.6",                  // 3 spaces then multi-byte
        ] {
            let out = postprocess_cpp_inl(line);
            assert!(
                out.contains("scxml-"),
                "citation text must survive postprocessing: {line:?} -> {out:?}"
            );
        }
    }

    // ── Language enum / FromStr drift guards ────────────────────
    //
    // RFC §synth-5-J-1 (downstream consumer, M1 foundation): the C11 enum
    // variant was added without a working emitter. These tests pin the
    // boundary contract so future edits cannot silently drop "c11"/"c"
    // recognition (which would silently route C11 callers to
    // UnknownLanguage at the CLI) and so the M2+ vertical slice has a
    // signal when adding a real C11 generator path.

    #[test]
    fn language_fromstr_accepts_c11_and_c_aliases() {
        use std::str::FromStr;
        assert_eq!(Language::from_str("c11").unwrap(), Language::C11);
        assert_eq!(Language::from_str("c").unwrap(), Language::C11);
    }

    #[test]
    fn language_fromstr_rejects_unknown_strings() {
        use std::str::FromStr;
        assert!(Language::from_str("c99").is_err());
        assert!(Language::from_str("C11").is_err()); // case-sensitive, matches prior precedent
        assert!(Language::from_str("").is_err());
    }

    #[test]
    fn language_c11_distinct_from_other_variants() {
        // Distinct enum membership is the reason to add the variant
        // before the emitter exists — matches gain a pinned arm so M2+
        // implementation changes are visible in diff review.
        assert_ne!(Language::C11, Language::Cpp);
        assert_ne!(Language::C11, Language::Rust);
        assert_ne!(Language::C11, Language::Kotlin);
        assert_ne!(Language::C11, Language::Go);
        assert_ne!(Language::C11, Language::Python);
    }

    /// SCE_MESH.md §9.5 mesh-rpc invokes only have a C++ codegen path
    /// today. `generate` (Rust) MUST refuse — silent skipping would
    /// hand the operator a state machine where an `<invoke>` quietly
    /// does nothing at runtime.
    #[test]
    fn rust_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        let err = generate_with_templates(&model, templates, false).unwrap_err();
        match err {
            GenerateError::UnsupportedFeature(msg) => {
                assert!(msg.contains("sce:mesh-rpc"), "msg names the feature: {msg}");
                assert!(msg.contains("Rust"), "msg names the language: {msg}");
                assert!(msg.contains("brake"), "msg names the machine: {msg}");
            }
            other => panic!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    #[test]
    fn kotlin_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        let err = generate_kotlin_with_templates(&model, templates, None).unwrap_err();
        assert!(matches!(err, GenerateError::UnsupportedFeature(_)));
    }

    #[test]
    fn go_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        let err = generate_go_with_templates(&model, templates).unwrap_err();
        assert!(matches!(err, GenerateError::UnsupportedFeature(_)));
    }

    #[test]
    fn c11_generate_rejects_mesh_rpc_invoke() {
        let model = parse(MESH_RPC_SCXML);
        let templates: &[(&str, &str)] = &[];
        match generate_c11_with_templates(&model, templates, "fixture") {
            Ok(_) => panic!("expected UnsupportedFeature, got Ok"),
            Err(GenerateError::UnsupportedFeature(msg)) => {
                assert!(msg.contains("sce:mesh-rpc"), "msg names the feature: {msg}");
                assert!(msg.contains("C11"), "msg names the language: {msg}");
            }
            Err(other) => panic!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    /// The refusal set is derived from the template tree, not asserted.
    ///
    /// A backend is refused exactly when the embedded tree carries no
    /// `mesh/<lang>/` arm for it, so adding that directory is what lifts
    /// a refusal. The sibling tests above pin the *messages*; this one
    /// pins the *rule*, which is the half a hand-written "C++-only"
    /// string could outlive — the gate would keep refusing a backend the
    /// tree had since learned to emit, and nothing would disagree.
    #[test]
    fn mesh_rpc_refusal_is_derived_from_the_template_tree() {
        let model = parse(MESH_RPC_SCXML);
        let (mut served, mut refused) = (0usize, 0usize);
        for &language in Language::ALL {
            let arm = language.feature_tree_subdir();
            let verdict = reject_mesh_rpc_in_unsupported_lang(&model, language);
            if mesh_templates_exist_for(language) {
                served += 1;
                assert!(
                    verdict.is_ok(),
                    "{language:?} has templates/mesh/{arm}/ but the gate refused it"
                );
            } else {
                refused += 1;
                assert!(
                    verdict.is_err(),
                    "{language:?} has no templates/mesh/{arm}/ arm, so codegen \
                     would drop the invoke silently — the gate must refuse"
                );
            }
        }
        // Both bounds, because either side alone makes the equivalence
        // vacuous: a tree with no mesh arm at all satisfies the loop by
        // refusing everything, and one with every arm by refusing nothing.
        assert!(
            served >= 1,
            "no backend has a mesh arm — this case read nothing"
        );
        assert!(
            refused >= 1,
            "every backend has a mesh arm — the gate is dead code"
        );
    }

    // §scxml-G-7 `<sce:action>` is lowered by EVERY backend. A no-argument
    // action needs no schema, so the document parses without an
    // `<sce:import>`.
    const NATIVE_ACTION_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s">
  <state id="s"><transition event="e" target="s"><sce:action name="do_effect"/></transition></state>
</scxml>"##;

    /// The five tests below asserted the OPPOSITE until 2026-08-24: that C++,
    /// Kotlin, Go, Python and C11 each refused this document with
    /// `generate/unsupported-feature`. That refusal was backend coverage
    /// rather than a design constraint, and retiring it left five tests whose
    /// whole subject was the gap — the shape that keeps reading as a pass
    /// while measuring nothing.
    ///
    /// So they are re-aimed at what is still true at THIS layer. These
    /// entry points are handed an EMPTY template list, so the furthest they
    /// can get is template loading — and that is exactly the assertion: the
    /// document reaches each backend's emitter, and whatever stops it there
    /// is about a missing template, never about the construct. The positive
    /// half, with real templates, is
    /// `sce-build/tests/native_action_backend_parity.rs`.
    fn assert_reaches_the_emitter(err: GenerateError, lang: &str) {
        match err {
            // Template loading is as far as an empty template list goes.
            GenerateError::TemplateLoad(_) | GenerateError::TemplateRender(_) => {}
            GenerateError::UnsupportedFeature(msg) if msg.contains("sce:action") => panic!(
                "{lang} still refuses <sce:action> as an unsupported feature. That \
                 backend-coverage refusal was retired when every backend grew a \
                 native-action path; a refusal here means one of them lost it: {msg}"
            ),
            other => panic!(
                "{lang}: expected to reach template loading, got {other:?} — the \
                 document stopped somewhere before the emitter"
            ),
        }
    }

    #[test]
    fn cpp_generate_reaches_the_emitter_for_a_native_action() {
        let model = parse(NATIVE_ACTION_SCXML);
        // `GeneratedOutput` is not `Debug`, so match rather than `unwrap_err`.
        match generate_cpp_with_templates(&model, &[], "fixture") {
            Ok(_) => panic!("an empty template list cannot produce output"),
            Err(e) => assert_reaches_the_emitter(e, "C++"),
        }
    }

    #[test]
    fn kotlin_generate_reaches_the_emitter_for_a_native_action() {
        let model = parse(NATIVE_ACTION_SCXML);
        let err = generate_kotlin_with_templates(&model, &[], None).unwrap_err();
        assert_reaches_the_emitter(err, "Kotlin");
    }

    #[test]
    fn go_generate_reaches_the_emitter_for_a_native_action() {
        let model = parse(NATIVE_ACTION_SCXML);
        let err = generate_go_with_templates(&model, &[]).unwrap_err();
        assert_reaches_the_emitter(err, "Go");
    }

    #[test]
    fn python_generate_reaches_the_emitter_for_a_native_action() {
        let model = parse(NATIVE_ACTION_SCXML);
        let err = generate_python_with_templates(&model, &[]).unwrap_err();
        assert_reaches_the_emitter(err, "Python");
    }

    #[test]
    fn c11_generate_reaches_the_emitter_for_a_native_action() {
        let model = parse(NATIVE_ACTION_SCXML);
        match generate_c11_with_templates(&model, &[], "fixture") {
            Ok(_) => panic!("an empty template list cannot produce output"),
            Err(e) => assert_reaches_the_emitter(e, "C11"),
        }
    }

    /// SCE_MESH.md §14 rule 12 / §16.5 shape assertion. With the
    /// semantic payload landed, the C++ codegen output diverges by
    /// role — Root emits a `ParallelCompletionTracker` member and
    /// `onParallelRegionDone` dispatch method; NonRoot emits a
    /// `sendParallelRegionDone` method with a wire-21 envelope
    /// constructor; SinglePartition (empty role map or absent
    /// partition_context) preserves the legacy
    /// `ParallelCompletionHelper` path.
    ///
    /// The P0 byte-identical carve-out has retired — `partition_context_present`
    /// alone (role map empty) still reproduces the pre-mesh output
    /// because `parallel_final.jinja2` falls through to the
    /// SinglePartition branch, but toggling a Role per-`<parallel>`
    /// in `partition_parallel_roles` intentionally perturbs the
    /// generated SM.
    const PARALLEL_FINAL_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="pf_fixture" initial="root">
  <parallel id="root">
    <state id="left" initial="left_run">
      <state id="left_run">
        <transition event="finish_left" target="left_done"/>
      </state>
      <final id="left_done"/>
    </state>
    <state id="right" initial="right_run">
      <state id="right_run">
        <transition event="finish_right" target="right_done"/>
      </state>
      <final id="right_done"/>
    </state>
  </parallel>
</scxml>"##;

    fn render_with_role(role: Option<crate::model::PartitionRole>) -> String {
        let mut model = parse(PARALLEL_FINAL_SCXML);
        crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
        if let Some(role) = role {
            model.partition_context_present = true;
            model
                .partition_parallel_roles
                .insert("root".to_string(), role);
        }
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("tools")
            .join("codegen")
            .join("templates");
        let out = generate_cpp(&model, &template_dir, "pf_fixture", None).expect("render");
        out.files
            .into_iter()
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Variant of [`PARALLEL_FINAL_SCXML`] that adds an
    /// `error.communication` handler so the §16.5 L3500 barrier-
    /// timeout runtime can emit without tripping
    /// [`reject_barrier_timeout_without_handler`]. The transition
    /// target (`timeout_failed`) is a dedicated final state so the
    /// raise path is authoring-observable in E2E tests too.
    const PARALLEL_FINAL_WITH_TIMEOUT_HANDLER_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="pf_fixture" initial="root">
  <parallel id="root">
    <state id="left" initial="left_run">
      <state id="left_run">
        <transition event="finish_left" target="left_done"/>
      </state>
      <final id="left_done"/>
    </state>
    <state id="right" initial="right_run">
      <state id="right_run">
        <transition event="finish_right" target="right_done"/>
      </state>
      <final id="right_done"/>
    </state>
    <transition event="error.communication" target="timeout_failed"/>
  </parallel>
  <final id="timeout_failed"/>
</scxml>"##;

    fn render_root_with_barrier_timeout(timeout_ms: Option<u32>) -> String {
        let mut model = parse(PARALLEL_FINAL_WITH_TIMEOUT_HANDLER_SCXML);
        crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
        model.partition_context_present = true;
        model
            .partition_parallel_roles
            .insert("root".to_string(), crate::model::PartitionRole::Root);
        if let Some(ms) = timeout_ms {
            model
                .partition_barrier_timeouts
                .insert("root".to_string(), ms);
        }
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("tools")
            .join("codegen")
            .join("templates");
        let out = generate_cpp(&model, &template_dir, "pf_fixture", None).expect("render");
        out.files
            .into_iter()
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn partition_role_root_emits_tracker_and_handlers() {
        let body = render_with_role(Some(crate::model::PartitionRole::Root));
        assert!(
            body.contains("tracker_root_"),
            "Root role must emit ParallelCompletionTracker member `tracker_root_`; body was:\n{body}"
        );
        assert!(
            body.contains("onParallelRegionDone"),
            "Root role must emit the `onParallelRegionDone` wire-21 receiver method"
        );
        // §16.5 wire-21 typed envelope — receiver dispatches on the typed
        // `env.parallel_id` / `env.region_id` (CBOR keys 16/17) without
        // string concat/split. The earlier `subject = "pid/rid"` +
        // `subject.find('/')` + `substr` path is retired.
        assert!(
            body.contains("env.parallel_id.has_value()") && body.contains("env.region_id.has_value()"),
            "Root wire-21 receiver must gate on both typed optional fields `parallel_id` and `region_id`; body was:\n{body}"
        );
        assert!(
            !body.contains("env.subject.has_value()"),
            "Root wire-21 receiver must not read `env.subject` — migrated to typed fields (§16.5)"
        );
        assert!(
            !body.contains("subject.find('/')") && !body.contains("subject.substr"),
            "Root wire-21 receiver must not string-parse a `subject` field — dispatch is on typed fields"
        );
        // §16.5 dispatch path: `parallel_final.jinja2` calls
        // `engine.triggerParallelRegionLocalComplete(parallel_id, region_id)`
        // (a base StaticExecutionEngine method); the SM ctor's
        // `setParallelRegionLocalCompleteCallback` closure routes that
        // through `tracker_<pid>_.onLocalRegionComplete(region_id)`.
        // Asserting both call sites pins the full dispatch path.
        assert!(
            body.contains("triggerParallelRegionLocalComplete(\"root\", \"left\")"),
            "Root region-final branch must dispatch via `engine.triggerParallelRegionLocalComplete`; body was:\n{body}"
        );
        assert!(
            body.contains("setParallelRegionLocalCompleteCallback"),
            "Root SM ctor must install the local-complete callback on the base via `setParallelRegionLocalCompleteCallback`"
        );
        assert!(
            body.contains("tracker_root_.onLocalRegionComplete(region_id)"),
            "Root SM ctor closure must terminate the dispatch in `tracker_root_.onLocalRegionComplete(region_id)`"
        );
        assert!(
            !body.contains("sendParallelRegionDone"),
            "Root role must NOT emit the non-root sender hook — the region is local"
        );
    }

    #[test]
    fn partition_role_non_root_emits_wire21_sender_only() {
        let body = render_with_role(Some(crate::model::PartitionRole::NonRoot));
        assert!(
            body.contains("sendParallelRegionDone"),
            "NonRoot role must emit the `sendParallelRegionDone` wire-21 sender method"
        );
        assert!(
            body.contains("PatternKind::ParallelRegionDone"),
            "NonRoot sender body must construct the wire-21 envelope"
        );
        // §16.5 wire-21 typed envelope — sender assigns BOTH typed fields
        // (`env.parallel_id`, `env.region_id`) on every outbound, replacing
        // the earlier `env.subject = parallel_id + "/" + region_id` concat.
        assert!(
            body.contains("env.parallel_id = parallel_id"),
            "NonRoot sender must set typed `env.parallel_id` on the wire-21 envelope; body was:\n{body}"
        );
        assert!(
            body.contains("env.region_id = region_id"),
            "NonRoot sender must set typed `env.region_id` on the wire-21 envelope"
        );
        assert!(
            !body.contains("env.subject = parallel_id"),
            "NonRoot sender must not populate `env.subject` for wire-21 — migrated to typed fields (§16.5)"
        );
        // §16.5 dispatch path mirrors the Root assertions: the
        // `parallel_final.jinja2` body calls
        // `engine.triggerParallelRegionRemoteSend(parallel_id, region_id, donedata)`,
        // and the SM ctor's `setParallelRegionRemoteSendCallback`
        // closure terminates the dispatch in `sendParallelRegionDone`.
        assert!(
            body.contains("triggerParallelRegionRemoteSend"),
            "NonRoot region-final branch must dispatch via `engine.triggerParallelRegionRemoteSend`; body was:\n{body}"
        );
        assert!(
            body.contains("setParallelRegionRemoteSendCallback"),
            "NonRoot SM ctor must install the remote-send callback on the base via `setParallelRegionRemoteSendCallback`"
        );
        assert!(
            !body.contains("tracker_root_"),
            "NonRoot role must NOT emit a tracker — aggregation is the root's job"
        );
        assert!(
            !body.contains("onParallelRegionDone"),
            "NonRoot role must NOT emit the receiver hook — envelopes land on the root"
        );
        assert!(
            !body.contains("ParallelCompletionHelper::areAllRegionsInFinal"),
            "NonRoot must not fall back to single-partition legacy completion check"
        );
    }

    #[test]
    fn partition_context_absent_falls_back_to_single_partition() {
        // `partition_context_present=false` → template's outer `{% if %}`
        // does not include the delegate; single-partition AOT path is
        // byte-identical to pre-mesh legacy.
        let body = render_with_role(None);
        assert!(
            body.contains("ParallelCompletionHelper::areAllRegionsInFinal"),
            "Default path must emit the legacy single-partition completion check"
        );
        assert!(
            !body.contains("tracker_root_"),
            "Default path must not emit mesh tracker members"
        );
        assert!(
            !body.contains("sendParallelRegionDone"),
            "Default path must not emit mesh sender hooks"
        );
    }

    #[test]
    fn partition_role_single_partition_preserves_legacy_path() {
        // A partitioned machine whose `<parallel>` lives entirely in one
        // partition (SinglePartition role) still uses the legacy helper.
        let body = render_with_role(Some(crate::model::PartitionRole::SinglePartition));
        assert!(
            body.contains("ParallelCompletionHelper::areAllRegionsInFinal"),
            "SinglePartition role must use the legacy completion helper"
        );
        assert!(
            !body.contains("tracker_root_"),
            "SinglePartition role must not emit mesh tracker members"
        );
    }

    // ── §16.5 L3500 barrier-timeout shape + observability gate ───

    #[test]
    fn partition_barrier_timeout_absent_emits_no_timer_machinery() {
        // Root role without `partition_barrier_timeouts` ⇒ W3C
        // normative infinity ⇒ no TimerHooks, no scheduler call,
        // no `PARALLEL_BARRIER_TIMEOUT` string.
        let body = render_root_with_barrier_timeout(None);
        assert!(
            body.contains("tracker_root_"),
            "Root role must still emit the tracker member"
        );
        assert!(
            !body.contains("TimerHooks"),
            "infinity (no barrier_timeout_ms) must not emit `TimerHooks`; body was:\n{body}"
        );
        assert!(
            !body.contains("ParallelBarrierTimeout"),
            "infinity must not emit the §16.7 row 6 ReasonCode raise path"
        );
        assert!(
            !body.contains("__sce_barrier_timeout_"),
            "infinity must not emit the deterministic timer send-id constant"
        );
    }

    #[test]
    fn partition_barrier_timeout_present_emits_timer_hooks_and_raise() {
        // Root role with a finite `barrier_timeout_ms` ⇒ TimerHooks
        // block populated, arm/cancel call through `scheduleEvent` /
        // `cancelEvent` with the deterministic send-id, payload shaped
        // by `CommunicationError::toJsonBytes`.
        let body = render_root_with_barrier_timeout(Some(3500));
        assert!(
            body.contains("ParallelCompletionTracker::TimerHooks"),
            "finite barrier_timeout_ms must emit the TimerHooks aggregate; body was:\n{body}"
        );
        assert!(
            body.contains("ReasonCode::ParallelBarrierTimeout"),
            "finite barrier_timeout_ms must pin the §16.7 row 6 ReasonCode raise path"
        );
        assert!(
            body.contains("__sce_barrier_timeout_root"),
            "arm/cancel must route through a deterministic per-parallel send-id"
        );
        assert!(
            body.contains("CommunicationError"),
            "JSON payload must be shaped via CommunicationError::toJsonBytes"
        );
        assert!(
            body.contains("PolicyType::Event::Error_communication"),
            "timer-fire event must be the W3C-bridged Event::Error_communication"
        );
        assert!(
            body.contains("Error_communication") && body.contains("scheduleEvent"),
            "arm callback must call the base engine's scheduleEvent with the error event"
        );
        assert!(
            body.contains("3500"),
            "timeout_ms must be baked in verbatim from deploy.yaml"
        );
    }

    #[test]
    fn partition_barrier_timeout_without_error_handler_rejects() {
        // Same fixture but WITHOUT the `error.communication` transition
        // — codegen must refuse so the silent-broken observability
        // gap (`feedback_silently_broken_hooks`) is closed at build
        // time instead of as a mysterious no-op at runtime.
        let mut model = parse(PARALLEL_FINAL_SCXML);
        crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
        model.partition_context_present = true;
        model
            .partition_parallel_roles
            .insert("root".to_string(), crate::model::PartitionRole::Root);
        model
            .partition_barrier_timeouts
            .insert("root".to_string(), 1000);
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("tools")
            .join("codegen")
            .join("templates");
        let res = generate_cpp(&model, &template_dir, "pf_fixture", None);
        let err = match res {
            Ok(_) => panic!("barrier_timeout_ms without error.communication handler must reject"),
            Err(e) => e,
        };
        match err {
            GenerateError::UnsupportedFeature(msg) => {
                assert!(
                    msg.contains("barrier_timeout_ms"),
                    "msg cites the knob: {msg}"
                );
                assert!(
                    msg.contains("error.communication"),
                    "msg names the missing handler: {msg}"
                );
                assert!(
                    msg.contains("PARALLEL_BARRIER_TIMEOUT"),
                    "msg names §16.7 row 6 reason: {msg}"
                );
                // Machine name is whatever the test parser assigned via
                // `parse_string(..., "brake")`; the assertion only cares
                // that SOME machine identifier is surfaced.
                assert!(
                    msg.contains("machine"),
                    "msg names the compiled machine: {msg}"
                );
                assert!(msg.contains("root"), "msg names the parallel id: {msg}");
            }
            other => panic!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    #[test]
    fn machine_liveliness_without_error_handler_rejects() {
        // Symmetric to `partition_barrier_timeout_without_error_handler_rejects`
        // for the §16.4 / §16.7 liveness raise paths. `machine_liveliness_opt_in=true`
        // with no `<transition event="error.communication">` in the SCXML
        // must be refused at codegen — the `feedback_silently_broken_hooks`
        // gate covers both row 8 (`PEER_PARTITIONED`, non-partitioned) and
        // row 13 (`REGION_PARTITIONED`, partitioned) because both rows
        // surface through `error.communication` and share the same
        // silent-broken failure mode. Partitioned and non-partitioned
        // fixtures both probed so the gate is never dead for either axis.
        for &(context_present, label) in &[(true, "partitioned"), (false, "non-partitioned")] {
            let mut model = parse(PARALLEL_FINAL_SCXML);
            crate::analyzer::analyze(&mut model, "pf_fixture.scxml");
            model.partition_context_present = context_present;
            if context_present {
                model
                    .partition_parallel_roles
                    .insert("root".to_string(), crate::model::PartitionRole::Root);
            }
            model.machine_liveliness_opt_in = true;
            let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("workspace root")
                .join("tools")
                .join("codegen")
                .join("templates");
            let res = generate_cpp(&model, &template_dir, "pf_fixture", None);
            let err = match res {
                Ok(_) => panic!(
                    "machine_liveliness_opt_in without error.communication handler must reject \
                     ({label})"
                ),
                Err(e) => e,
            };
            match err {
                GenerateError::UnsupportedFeature(msg) => {
                    assert!(
                        msg.contains("liveliness"),
                        "{label}: msg cites the knob: {msg}"
                    );
                    assert!(
                        msg.contains("error.communication"),
                        "{label}: msg names the missing handler: {msg}"
                    );
                    // Gate speaks for both rows; pin both reason codes so
                    // a future narrowing (e.g. re-splitting the gate) has
                    // to update the test intentionally rather than drift.
                    assert!(
                        msg.contains("PEER_PARTITIONED"),
                        "{label}: msg names §16.7 row 8 reason: {msg}"
                    );
                    assert!(
                        msg.contains("REGION_PARTITIONED"),
                        "{label}: msg names §16.7 row 13 reason: {msg}"
                    );
                    // Machine name is whatever `<scxml name=...>` carries
                    // in PARALLEL_FINAL_SCXML — the assertion only pins
                    // that SOME machine identifier is surfaced, not the
                    // exact string.
                    assert!(
                        msg.contains("machine"),
                        "{label}: msg names the compiled machine: {msg}"
                    );
                }
                other => panic!("{label}: expected UnsupportedFeature, got {other:?}"),
            }
        }
    }

    /// Models without mesh-rpc invokes must NOT be rejected — the gate
    /// is feature-specific, not a blanket non-C++ block.
    #[test]
    fn rust_generate_accepts_plain_scxml() {
        let plain = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="plain" initial="s">
  <state id="s"><transition event="e" target="s2"/></state>
  <state id="s2"/>
</scxml>"##;
        let model = parse(plain);
        // We only care that the gate doesn't reject; the actual
        // template render needs the full template set, which this
        // unit test deliberately omits to keep it focused. The gate
        // runs FIRST, so its early-return on Ok(()) lets the call
        // proceed to the (failing-without-templates) render path —
        // any error here other than UnsupportedFeature proves the
        // gate is not the blocker.
        let templates: &[(&str, &str)] = &[];
        // Anything other than UnsupportedFeature (Ok / template error / etc.)
        // means the gate let the model through, which is the contract.
        if let Err(GenerateError::UnsupportedFeature(_)) =
            generate_with_templates(&model, templates, false)
        {
            panic!("plain SCXML must not trip the mesh-rpc gate")
        }
    }

    /// A `<send>` `<param>` whose `expr` is a quoted string literal, on a
    /// machine that emits no script engine. Both the immediate and the
    /// delayed send path carry a params block, and each had its own copy
    /// of the emission, so both are exercised here.
    const STATIC_PARAM_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" name="sp_fixture" initial="idle">
  <state id="idle">
    <onentry>
      <send event="now.fired">
        <param name="corner" expr="'front_left'"/>
      </send>
      <send event="later.fired" delay="10ms">
        <param name="wheel" expr="'rear_right'"/>
      </send>
    </onentry>
    <transition event="now.fired" target="done"/>
    <transition event="later.fired" target="done"/>
  </state>
  <final id="done"/>
</scxml>"##;

    /// A static string literal `<param>` must be emitted as its value.
    ///
    /// The regression it guards: with no script engine on the machine,
    /// both params blocks pasted `expr` verbatim into
    /// `std::to_string(...)`, so `expr="'front_left'"` became
    /// `std::to_string('front_left')` — a C++ multi-character literal.
    /// That is a *number* (`'42'` is 13362), and for a literal longer
    /// than an `int` it is "character constant too long for its type",
    /// which `-Werror` rejects outright. Four sibling param blocks in
    /// `send.jinja2` already read `static_value`; these two did not.
    ///
    /// Asserted at the render layer rather than only through the DDS pool
    /// runtime test that found it, because that test needs CycloneDDS and
    /// this path has nothing to do with any transport.
    #[test]
    fn static_literal_send_param_emits_its_value_not_a_char_literal() {
        let mut model = parse(STATIC_PARAM_SCXML);
        crate::analyzer::analyze(&mut model, "sp_fixture.scxml");
        assert!(
            !model.needs_script_engine,
            "fixture precondition: a static-literal param must not pull in the script \
             engine, or the branch under test is not the one being rendered"
        );
        let template_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("tools")
            .join("codegen")
            .join("templates");
        let out = generate_cpp(&model, &template_dir, "sp_fixture", None).expect("render");
        let body = out
            .files
            .into_iter()
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            body.contains(r#"params["corner"].push_back("front_left")"#),
            "the immediate send's static literal param must emit its value; body was:\n{body}"
        );
        assert!(
            body.contains(r#"params["wheel"].push_back("rear_right")"#),
            "the delayed send's static literal param must emit its value; body was:\n{body}"
        );
        // The shape of the defect, independent of which param tripped it:
        // no `<send>` param value may reach C++ inside a character
        // literal, whatever the emission around it looks like.
        assert!(
            !body.contains("std::to_string('"),
            "a param expr was pasted into a C++ character literal; body was:\n{body}"
        );
    }

    /// Every dynamic `<send>` attribute must force the script engine.
    ///
    /// `send.jinja2` renders each of these attributes through the engine
    /// and leaves a `#error` on the other side of the branch, because the
    /// alternative — pasting the author's SCXML expression into C++ — is
    /// what made the static-literal `<param>` above emit a character
    /// literal. Those `#error`s are only correct while this invariant
    /// holds, so the invariant is asserted here rather than assumed:
    /// `send_has_dynamic_attr` names the same six attributes, and a
    /// future exemption (the shape the `<param>` path took, via
    /// `is_static_literal`) would turn a `#error` into a build break on
    /// valid input.
    #[test]
    fn every_dynamic_send_attribute_forces_the_script_engine() {
        // `contentexpr` is the sixth attribute `send_has_dynamic_attr`
        // names; it has no C++ paste site, so the five with one are
        // covered here and `idlocation` rides along as an attribute
        // rather than an expression.
        for (label, attrs) in [
            (
                "typeexpr",
                r#"typeexpr="'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'" event="e.x""#,
            ),
            ("targetexpr", r#"targetexpr="'#_internal'" event="e.x""#),
            ("delayexpr", r#"delayexpr="'10ms'" event="e.x""#),
            ("eventexpr", r#"eventexpr="'e.x'""#),
            ("idlocation", r#"event="e.x" idlocation="slot""#),
        ] {
            let doc = format!(
                r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" name="dyn_fixture" initial="idle">
  <state id="idle">
    <onentry><send {attrs}/></onentry>
    <transition event="e.x" target="done"/>
  </state>
  <final id="done"/>
</scxml>"##
            );
            let mut model = parse(&doc);
            crate::analyzer::analyze(&mut model, "dyn_fixture.scxml");
            assert!(
                model.needs_script_engine,
                "`{label}` did not force the script engine — send.jinja2's `#error` for it \
                 would now fire on valid input"
            );
        }
    }
}
