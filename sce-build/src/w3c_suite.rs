// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Identity and packaging of an emitted W3C conformance suite.
//
// `generate-w3c --output-dir` has been able to write the generated
// trees under any root the caller names, but what landed there was not
// a package. Two things were missing, and each one alone is enough to
// stop a build:
//
//   1. The generated tests name the suite they belong to — a Rust
//      integration test lives outside the crate it exercises and must
//      spell the crate (`sce_rust_tests::generated::test144`), a Go
//      test imports the module's harness package, a Kotlin test
//      imports the package root. Those names were generator-side
//      literals, so the emitted tree compiled only inside a checkout
//      that happened to carry SCE's own names.
//   2. The files that make the tree a package — the manifest, the
//      crate/module root, the harness the tests call into — were never
//      emitted, because in this repository they are hand-authored and
//      already sitting where the generator writes.
//
// This module makes the suite's own name an input and carries the
// harness sources that travel with it. The build manifests are
// rendered next to the test-file emitters in `sce_codegen.rs`, which
// is where the sibling per-backend emission already lives.
//
// Deliberately no new crate dependency and no new file under
// `tools/codegen/templates/`: `forge::drift::compute_template_hash`
// digests `Cargo.lock` *and* the whole template tree, so either one
// would restamp `template-hash` in every committed generated file and
// force a full-tree regeneration for a change that generates no state
// machine code.

use std::fmt;

use crate::generator::Language;

/// Rust module path the committed harness and the committed generated
/// tests use to name this repository's own suite.
///
/// Substituting from this token is what lets a suite emitted under
/// another name reach itself, and pinning it here — rather than
/// spelling `sce_rust_tests` at each substitution site — is what keeps
/// the default emission byte-identical to the committed tree.
pub const RUST_DEFAULT_MODULE_PATH: &str = "sce_rust_tests";

/// Cargo package name of this repository's Rust conformance suite.
pub const RUST_DEFAULT_PACKAGE: &str = "sce-rust-tests";

/// Go module path of this repository's Go conformance suite.
pub const GO_DEFAULT_MODULE: &str = "github.com/newmassrael/sce-go-tests";

/// Kotlin package root of this repository's Kotlin conformance suite.
///
/// The generated machines land in `<root>.generated.test<id>` and the
/// generated JUnit classes in `<root>.w3c`, which is why the root — not
/// either full package — is the thing a caller names.
pub const KOTLIN_DEFAULT_PACKAGE_ROOT: &str = "com.sce";

/// The committed Rust harness, compiled into the generator so an
/// emitted suite cannot carry a stale copy of it.
///
/// Reading the real file rather than restating it is what makes drift
/// impossible: there is one harness, and the emitted suite gets that
/// one. The only edit applied on the way out is the suite's own name
/// (see [`SuiteIdentity::rewrite_rust_source`]), because the harness
/// documents its usage from an integration test, which sits outside
/// the crate and therefore has to spell it.
pub const RUST_HARNESS_SOURCE: &str = include_str!("../../backends/rust/tests/src/harness.rs");

/// The committed Go harness, compiled in for the same reason.
///
/// Its own package clause is `package harness` and its imports name the
/// SCE runtime modules, never the suite, so it travels verbatim — the
/// suite's module path reaches it through `go.mod` alone.
pub const GO_HARNESS_SOURCE: &str = include_str!("../../backends/go/tests/harness/harness.go");

/// The committed Go checksum database.
///
/// The SCE modules an emitted suite depends on are `replace`d onto
/// filesystem paths, which Go verifies by path rather than by sum; what
/// still needs a checksum is the one remote module the SCE Lua binding
/// pulls in. Shipping the committed sums lets the emitted module build
/// without a network round trip to rebuild a file this repository
/// already has.
pub const GO_SUM_SOURCE: &str = include_str!("../../backends/go/tests/go.sum");

/// The committed pytest fixtures, compiled in for the same reason.
///
/// Python's generated tests import their sibling machine by path and
/// take `setup_http` from a pytest fixture resolved by directory, so
/// nothing in the emitted Python names the suite. What the emitted tree
/// does need is this file: without it every BasicHTTP fixture errors on
/// a missing `setup_http` argument.
pub const PYTHON_CONFTEST_SOURCE: &str = include_str!("../../backends/python/tests/conftest.py");

/// The one line in the committed conftest that assumes SCE's directory
/// layout: it reaches the runtime by walking up from the conftest's own
/// location, which only lands anywhere inside this repository.
///
/// Pinned as a constant rather than matched by pattern so a rewrite
/// that no longer applies fails loudly — see
/// [`rewrite_python_conftest`].
pub const PYTHON_CONFTEST_RUNTIME_LINE: &str =
    "sys.path.insert(0, str(_HERE.parent / \"runtime\"))";

/// Re-point the emitted conftest's runtime path at `sce_root`.
///
/// Returns `Err` when the pinned line is absent. A rewrite that quietly
/// matched nothing would emit a conftest reaching for a directory the
/// suite does not have, and every fixture would then fail on an import
/// error that says nothing about the real cause.
pub fn rewrite_python_conftest(source: &str, sce_root: &std::path::Path) -> Result<String, String> {
    if !source.contains(PYTHON_CONFTEST_RUNTIME_LINE) {
        return Err(format!(
            "the committed conftest no longer contains `{PYTHON_CONFTEST_RUNTIME_LINE}`, so \
             an emitted suite would have no way to reach the SCE Python runtime. Update \
             w3c_suite::PYTHON_CONFTEST_RUNTIME_LINE to the line that replaced it."
        ));
    }
    Ok(source.replace(
        PYTHON_CONFTEST_RUNTIME_LINE,
        &format!(
            "sys.path.insert(0, {:?})",
            sce_root
                .join("backends/python/runtime")
                .display()
                .to_string(),
        ),
    ))
}

/// Packages beneath `com.sce.` that belong to the SCE **runtime**, not
/// to the conformance suite.
///
/// Everything else under that root is the suite's own — the tests
/// module owns `w3c`, `http` and `generated` — which is what makes the
/// rewrite below a rule rather than a hand-list that silently misses a
/// package. It missed one exactly that way: `com.sce.http` held the
/// BasicHTTP test server, was not in the list, and the emitted Kotlin
/// failed to compile on an unresolved reference.
pub const KOTLIN_RUNTIME_PACKAGES: &[&str] = &["runtime", "scripting"];

/// The committed Kotlin sources a suite carries: the two JUnit base
/// classes every generated test extends, and the BasicHTTP test server
/// one of them drives.
///
/// Each entry is `(source set, path within the package root, contents)`
/// — a Kotlin source tree mirrors package names as directories, so the
/// emitted path is `<source set>/<package dir>/<path>`.
pub const KOTLIN_SUITE_SOURCES: &[(&str, &str, &str)] = &[
    (
        "src/test/kotlin",
        "w3c/W3CTestBase.kt",
        include_str!("../../backends/kotlin/tests/src/test/kotlin/com/sce/w3c/W3CTestBase.kt"),
    ),
    (
        "src/test/kotlin",
        "w3c/W3CHttpTestBase.kt",
        include_str!("../../backends/kotlin/tests/src/test/kotlin/com/sce/w3c/W3CHttpTestBase.kt"),
    ),
    (
        "src/main/kotlin",
        "http/W3CHttpTestServer.kt",
        include_str!(
            "../../backends/kotlin/tests/src/main/kotlin/com/sce/http/W3CHttpTestServer.kt"
        ),
    ),
];

/// Why a language refuses to take a suite name.
///
/// A flag that is accepted and then ignored is worse than one that is
/// refused: the caller reads the acceptance as a promise. Both refusals
/// below are statements about the emitted code, not gaps — see
/// [`SuiteIdentity::for_language`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuiteIdentityError {
    /// The backend emits nothing that names the suite, so there is no
    /// name for the caller to set.
    NotApplicable {
        /// Language the caller asked for.
        language: Language,
        /// What that backend emits instead, so the message can say why.
        reason: &'static str,
    },
    /// The name cannot be spelled in the target language.
    Malformed {
        /// Language the name was to be read as.
        language: Language,
        /// The name as the caller gave it.
        name: String,
        /// What is wrong with it, in the target language's terms.
        reason: String,
    },
}

impl fmt::Display for SuiteIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotApplicable { language, reason } => write!(
                f,
                "--suite-package does not apply to the {} W3C backend: {reason}",
                language.canonical_name(),
            ),
            Self::Malformed {
                language,
                name,
                reason,
            } => write!(
                f,
                "'{name}' is not a usable {} conformance suite name: {reason}",
                language.canonical_name(),
            ),
        }
    }
}

/// What an emitted conformance suite calls itself.
///
/// One value per run. Each backend reads it in its own idiom — a Cargo
/// package name, a Go module path, a Kotlin package root — because that
/// is the form its generated tests have to spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteIdentity {
    language: Language,
    name: String,
}

impl SuiteIdentity {
    /// The name this repository's own suite carries, which is also what
    /// an in-repo regeneration must keep emitting: the committed trees
    /// are byte-compared, so the default has to be the committed value
    /// rather than a neutral placeholder.
    ///
    /// Returns `Err(NotApplicable)` for the two backends whose emitted
    /// code never names the suite.
    pub fn for_language(language: Language) -> Result<Self, SuiteIdentityError> {
        let name = match language {
            Language::Rust => RUST_DEFAULT_PACKAGE,
            Language::Go => GO_DEFAULT_MODULE,
            Language::Kotlin => KOTLIN_DEFAULT_PACKAGE_ROOT,
            Language::Python => {
                return Err(SuiteIdentityError::NotApplicable {
                    language,
                    reason: "its generated tests import the machine beside them by path and \
                             take fixtures from pytest's directory-scoped conftest, so no \
                             emitted file spells the suite",
                })
            }
            Language::Cpp | Language::C11 => {
                return Err(SuiteIdentityError::NotApplicable {
                    language,
                    reason: "it emits state machine translation units only — the test drivers \
                             are hand-authored headers under tests/w3c/aot_tests/ that CMake \
                             configures, so sce-codegen emits nothing that names a suite",
                })
            }
        };
        Ok(Self {
            language,
            name: name.to_string(),
        })
    }

    /// Read a caller-supplied name, refusing anything the target
    /// language cannot spell.
    ///
    /// Validation is not politeness: an unspellable name reaches the
    /// caller as a compiler error inside generated code they did not
    /// write, several hundred files after the run reported success.
    pub fn parse(language: Language, name: &str) -> Result<Self, SuiteIdentityError> {
        // Establishes applicability first, so a caller naming a suite
        // for a backend that has none is told that rather than being
        // told their name is malformed.
        Self::for_language(language)?;

        let malformed = |reason: String| SuiteIdentityError::Malformed {
            language,
            name: name.to_string(),
            reason,
        };
        if name.is_empty() {
            return Err(malformed("it is empty".to_string()));
        }

        match language {
            Language::Rust => validate_rust_package(name).map_err(malformed)?,
            Language::Go => validate_go_module(name).map_err(malformed)?,
            Language::Kotlin => validate_kotlin_package_root(name).map_err(malformed)?,
            // `for_language` above already refused these.
            Language::Python | Language::Cpp | Language::C11 => unreachable!(),
        }

        Ok(Self {
            language,
            name: name.to_string(),
        })
    }

    /// The name as the caller spelled it — the Cargo package name, the
    /// Go module path, the Kotlin package root.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether this is the name the committed trees carry.
    ///
    /// The emission of support files keys off the output root rather
    /// than this, but a caller who explicitly renames the suite while
    /// writing into the repository is asking for two different things
    /// at once, and that is worth refusing.
    pub fn is_repository_default(&self) -> bool {
        Self::for_language(self.language).is_ok_and(|d| d.name == self.name)
    }

    /// The token generated Rust tests use to reach the suite root.
    ///
    /// Cargo package names are conventionally kebab-case and the module
    /// path rustc derives from them is snake_case; generated code needs
    /// the latter.
    pub fn rust_module_path(&self) -> String {
        debug_assert_eq!(self.language, Language::Rust);
        self.name.replace('-', "_")
    }

    /// The import path prefix generated Go tests use. The harness they
    /// import lives at `<module>/harness`.
    pub fn go_module_path(&self) -> &str {
        debug_assert_eq!(self.language, Language::Go);
        &self.name
    }

    /// The package root generated Kotlin sources sit under: machines in
    /// `<root>.generated.test<id>`, JUnit classes in `<root>.w3c`.
    pub fn kotlin_package_root(&self) -> &str {
        debug_assert_eq!(self.language, Language::Kotlin);
        &self.name
    }

    /// Source-directory path the Kotlin package root maps onto
    /// (`com.acme.conformance` -> `com/acme/conformance`).
    pub fn kotlin_package_dir(&self) -> String {
        self.kotlin_package_root().replace('.', "/")
    }

    /// Rewrite a committed Rust source so it names *this* suite.
    ///
    /// Applies to the harness, whose module documentation shows the
    /// call from an integration test — which sits outside the crate and
    /// therefore spells it. At the repository default the rewrite is
    /// the identity, which is the property
    /// `emitted_default_harness_matches_the_committed_file` pins.
    pub fn rewrite_rust_source(&self, source: &str) -> String {
        let module_path = self.rust_module_path();
        if module_path == RUST_DEFAULT_MODULE_PATH {
            return source.to_string();
        }
        source.replace(RUST_DEFAULT_MODULE_PATH, &module_path)
    }

    /// Rewrite a Kotlin source so its `package` clause and intra-suite
    /// imports name *this* suite.
    ///
    /// The rule is "everything under `com.sce.` that the runtime does
    /// not own", not a list of the suite's packages: the suite grows a
    /// package whenever a harness file does, and a list would go on
    /// emitting the old root for it. `com.sce.runtime` and
    /// `com.sce.scripting.*` come from other Gradle projects and keep
    /// their names whoever consumes them.
    pub fn rewrite_kotlin_source(&self, source: &str) -> String {
        let root = self.kotlin_package_root();
        if root == KOTLIN_DEFAULT_PACKAGE_ROOT {
            return source.to_string();
        }
        let prefix = format!("{KOTLIN_DEFAULT_PACKAGE_ROOT}.");
        let mut out = String::with_capacity(source.len());
        let mut rest = source;
        while let Some(at) = rest.find(&prefix) {
            let (before, from_prefix) = rest.split_at(at);
            out.push_str(before);
            let tail = &from_prefix[prefix.len()..];
            let segment: String = tail
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if KOTLIN_RUNTIME_PACKAGES.contains(&segment.as_str()) {
                out.push_str(&prefix);
            } else {
                out.push_str(root);
                out.push('.');
            }
            rest = tail;
        }
        out.push_str(rest);
        out
    }
}

/// A Cargo package name, and one whose derived module path is a usable
/// Rust identifier — `cargo` accepts names rustc then cannot spell.
fn validate_rust_package(name: &str) -> Result<(), String> {
    let first = name.chars().next().expect("non-empty checked by caller");
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(format!(
            "a Cargo package name starts with a letter or underscore, not {first:?}"
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        return Err(format!(
            "a Cargo package name holds only ASCII letters, digits, '-' and '_', not {bad:?}"
        ));
    }
    let module_path = name.replace('-', "_");
    if RUST_KEYWORDS.contains(&module_path.as_str()) {
        return Err(format!(
            "its module path '{module_path}' is a Rust keyword, so generated tests could \
             not name it"
        ));
    }
    Ok(())
}

/// A Go module path: slash-separated, no empty element, no whitespace.
fn validate_go_module(name: &str) -> Result<(), String> {
    if let Some(bad) = name.chars().find(|c| c.is_whitespace()) {
        return Err(format!(
            "a Go module path holds no whitespace, and this has {bad:?}"
        ));
    }
    if name.starts_with('/') || name.ends_with('/') {
        return Err("a Go module path neither starts nor ends with '/'".to_string());
    }
    if name.split('/').any(str::is_empty) {
        return Err("a Go module path has no empty element between slashes".to_string());
    }
    Ok(())
}

/// A Kotlin package root: dot-separated identifiers.
fn validate_kotlin_package_root(name: &str) -> Result<(), String> {
    if name.starts_with('.') || name.ends_with('.') {
        return Err("a Kotlin package root neither starts nor ends with '.'".to_string());
    }
    for element in name.split('.') {
        if element.is_empty() {
            return Err("a Kotlin package root has no empty element between dots".to_string());
        }
        let first = element.chars().next().expect("non-empty checked above");
        if !first.is_ascii_alphabetic() && first != '_' {
            return Err(format!(
                "package element '{element}' starts with {first:?}, and a Kotlin identifier \
                 starts with a letter or underscore"
            ));
        }
        if let Some(bad) = element
            .chars()
            .find(|c| !c.is_ascii_alphanumeric() && *c != '_')
        {
            return Err(format!(
                "package element '{element}' holds {bad:?}, and a Kotlin identifier holds \
                 only ASCII letters, digits and '_'"
            ));
        }
    }
    Ok(())
}

/// Reserved words a Rust module path may not be. Only the strict list
/// matters here: a package whose module path is one of these cannot be
/// named by the generated integration tests at all.
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "async", "await", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_names_are_the_ones_the_committed_trees_carry() {
        // An in-repo regeneration must not move a single byte, so the
        // defaults are compared against the literals the committed
        // sources spell rather than against each other.
        let rust = SuiteIdentity::for_language(Language::Rust).expect("Rust names a suite");
        assert_eq!(rust.name(), "sce-rust-tests");
        assert_eq!(rust.rust_module_path(), RUST_DEFAULT_MODULE_PATH);
        assert!(rust.is_repository_default());

        let go = SuiteIdentity::for_language(Language::Go).expect("Go names a suite");
        assert_eq!(go.go_module_path(), "github.com/newmassrael/sce-go-tests");

        let kotlin = SuiteIdentity::for_language(Language::Kotlin).expect("Kotlin names a suite");
        assert_eq!(kotlin.kotlin_package_root(), "com.sce");
        assert_eq!(kotlin.kotlin_package_dir(), "com/sce");
    }

    #[test]
    fn the_backends_that_name_no_suite_refuse_rather_than_ignore() {
        for language in [Language::Python, Language::Cpp, Language::C11] {
            let err = SuiteIdentity::for_language(language)
                .expect_err("this backend emits nothing that names a suite");
            assert!(
                matches!(err, SuiteIdentityError::NotApplicable { .. }),
                "{language:?} must refuse the flag, not accept a name it will not use",
            );
            // The refusal has to survive `parse` too, or naming a suite
            // on the command line would be quietly accepted.
            assert!(matches!(
                SuiteIdentity::parse(language, "whatever"),
                Err(SuiteIdentityError::NotApplicable { .. }),
            ));
        }
    }

    #[test]
    fn a_rust_suite_name_becomes_the_module_path_generated_tests_spell() {
        let id = SuiteIdentity::parse(Language::Rust, "acme-conformance").expect("valid");
        assert_eq!(id.rust_module_path(), "acme_conformance");
        assert!(!id.is_repository_default());
    }

    #[test]
    fn rust_names_that_rustc_could_not_spell_are_refused() {
        for (name, why) in [
            ("9lives", "starts with a digit"),
            ("acme conformance", "holds a space"),
            ("acme.conformance", "holds a dot"),
            ("crate", "is a keyword"),
            ("super", "is a keyword"),
        ] {
            let err = SuiteIdentity::parse(Language::Rust, name)
                .expect_err(&format!("'{name}' {why}, so it must be refused"));
            assert!(matches!(err, SuiteIdentityError::Malformed { .. }));
        }
    }

    #[test]
    fn go_and_kotlin_names_are_read_in_their_own_idiom() {
        let go = SuiteIdentity::parse(Language::Go, "github.com/acme/conformance").expect("valid");
        assert_eq!(go.go_module_path(), "github.com/acme/conformance");
        for bad in ["/leading", "trailing/", "two//slashes", "has space"] {
            assert!(
                SuiteIdentity::parse(Language::Go, bad).is_err(),
                "'{bad}' is not a Go module path",
            );
        }

        let kt = SuiteIdentity::parse(Language::Kotlin, "com.acme.conformance").expect("valid");
        assert_eq!(kt.kotlin_package_dir(), "com/acme/conformance");
        for bad in [
            ".leading",
            "trailing.",
            "two..dots",
            "com.9lives",
            "com.a-b",
        ] {
            assert!(
                SuiteIdentity::parse(Language::Kotlin, bad).is_err(),
                "'{bad}' is not a Kotlin package root",
            );
        }
    }

    #[test]
    fn the_default_rewrite_is_the_identity() {
        // This is what keeps an in-repo regeneration byte-stable: the
        // rewrite runs on every emission, so at the default it must
        // change nothing at all.
        let rust = SuiteIdentity::for_language(Language::Rust).expect("Rust names a suite");
        assert_eq!(
            rust.rewrite_rust_source(RUST_HARNESS_SOURCE),
            RUST_HARNESS_SOURCE
        );
        let kotlin = SuiteIdentity::for_language(Language::Kotlin).expect("Kotlin names a suite");
        for (_, _, source) in KOTLIN_SUITE_SOURCES {
            assert_eq!(&kotlin.rewrite_kotlin_source(source), source);
        }
    }

    #[test]
    fn no_shipped_kotlin_source_keeps_a_suite_package_behind() {
        // The miss this pins actually happened: the rewrite listed the
        // suite's packages by hand, `com.sce.http` was not among them,
        // and the emitted Kotlin failed to compile on an unresolved
        // reference to the BasicHTTP test server. Asserting the
        // *absence* of a stale root is what a list cannot do for
        // itself.
        let id = SuiteIdentity::parse(Language::Kotlin, "com.acme.conformance").expect("valid");
        for (_, path, source) in KOTLIN_SUITE_SOURCES {
            let rewritten = id.rewrite_kotlin_source(source);
            for reference in kotlin_sce_references(&rewritten) {
                let segment = reference
                    .trim_start_matches("com.sce.")
                    .split('.')
                    .next()
                    .unwrap_or_default();
                assert!(
                    KOTLIN_RUNTIME_PACKAGES.contains(&segment),
                    "{path} still names `{reference}` after the rewrite, and \
                     `com.sce.{segment}` is not a runtime package — an emitted \
                     suite would reference a package it does not carry",
                );
            }
        }
    }

    /// Every `com.sce.…` reference in a Kotlin source.
    fn kotlin_sce_references(source: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut rest = source;
        while let Some(at) = rest.find("com.sce.") {
            let tail = &rest[at..];
            let end = tail
                .char_indices()
                .find(|(_, c)| !(c.is_alphanumeric() || *c == '_' || *c == '.'))
                .map_or(tail.len(), |(i, _)| i);
            found.push(tail[..end].trim_end_matches('.').to_string());
            rest = &tail[end.max(1)..];
        }
        found
    }

    #[test]
    fn the_runtime_packages_survive_a_rename_and_the_suite_packages_do_not() {
        let id = SuiteIdentity::parse(Language::Kotlin, "com.acme.conformance").expect("valid");
        let rewritten = id.rewrite_kotlin_source(
            "package com.sce.http\n\
             import com.sce.runtime.State\n\
             import com.sce.scripting.lua.LuaScriptEngine\n\
             import com.sce.w3c.W3CTestBase\n\
             import com.sce.generated.test144.Test144StateMachine\n",
        );
        assert!(rewritten.contains("package com.acme.conformance.http"));
        assert!(rewritten.contains("import com.acme.conformance.w3c.W3CTestBase"));
        assert!(rewritten.contains("import com.acme.conformance.generated.test144"));
        assert!(rewritten.contains("import com.sce.runtime.State"));
        assert!(rewritten.contains("import com.sce.scripting.lua.LuaScriptEngine"));
    }

    #[test]
    fn the_python_conftest_rewrite_refuses_to_match_nothing() {
        // A rewrite that quietly matched nothing would emit a conftest
        // reaching for a directory the suite does not have, and every
        // fixture would fail on an import error naming neither cause.
        assert!(
            rewrite_python_conftest("nothing to see", std::path::Path::new("/sce")).is_err(),
            "a conftest without the pinned line must be refused, not shipped as-is",
        );
        let rewritten =
            rewrite_python_conftest(PYTHON_CONFTEST_SOURCE, std::path::Path::new("/sce"))
                .expect("the committed conftest carries the pinned line");
        assert!(rewritten.contains("/sce/backends/python/runtime"));
        assert!(!rewritten.contains(PYTHON_CONFTEST_RUNTIME_LINE));
    }

    #[test]
    fn the_rust_rewrite_reaches_the_harness_documentation() {
        // The harness spells the crate in its usage example because the
        // example is an integration test, which lives outside the
        // crate. A renamed suite shipping that example would document a
        // crate the consumer does not have.
        assert!(
            RUST_HARNESS_SOURCE.contains(RUST_DEFAULT_MODULE_PATH),
            "the substitution has nothing to reach unless the harness names the suite",
        );
        let id = SuiteIdentity::parse(Language::Rust, "acme-conformance").expect("valid");
        let rewritten = id.rewrite_rust_source(RUST_HARNESS_SOURCE);
        assert!(rewritten.contains("acme_conformance::"));
        assert!(!rewritten.contains(RUST_DEFAULT_MODULE_PATH));
    }

    #[test]
    fn the_kotlin_rewrite_moves_the_suite_and_leaves_the_runtime_alone() {
        let id = SuiteIdentity::parse(Language::Kotlin, "com.acme.conformance").expect("valid");
        let source = "package com.sce.w3c\n\
                      import com.sce.runtime.State\n\
                      import com.sce.scripting.RhinoScriptEngine\n\
                      import com.sce.generated.test144.Test144StateMachine\n";
        let rewritten = id.rewrite_kotlin_source(source);
        assert!(rewritten.contains("package com.acme.conformance.w3c"));
        assert!(rewritten.contains("import com.acme.conformance.generated.test144"));
        // The runtime is a dependency, not part of the suite: it keeps
        // its own package whoever consumes it.
        assert!(rewritten.contains("import com.sce.runtime.State"));
        assert!(rewritten.contains("import com.sce.scripting.RhinoScriptEngine"));
    }

    #[test]
    fn the_committed_harness_sources_are_the_ones_that_ship() {
        // Compiled in rather than restated, so the assertion worth
        // making is that each really is the file it claims to be.
        assert!(RUST_HARNESS_SOURCE.contains("pub fn setup_http_test"));
        assert!(RUST_HARNESS_SOURCE.contains("pub const HTTP_TEST_SERVER_URL"));
        assert!(GO_HARNESS_SOURCE.contains("func SetupHTTPTest"));
        assert!(GO_HARNESS_SOURCE.contains("const BasicHTTPAccessURI"));
        assert!(PYTHON_CONFTEST_SOURCE.contains("setup_http"));
        assert_eq!(KOTLIN_SUITE_SOURCES.len(), 3);
        assert!(KOTLIN_SUITE_SOURCES[0]
            .2
            .contains("abstract class W3CTestBase"));
        assert!(KOTLIN_SUITE_SOURCES[1].2.contains("W3CHttpTestBase"));
        assert!(KOTLIN_SUITE_SOURCES[2]
            .2
            .contains("class W3CHttpTestServer"));
    }
}
