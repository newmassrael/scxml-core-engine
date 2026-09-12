// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A standard named in executable code is one SCE implements.
//!
//! Owner's instruction, 2026-09-12: *"범용적으로 만들어야 해, 특정 스펙에
//! 종속되면 안 돼"*. Written as a rule the tree can be asked:
//!
//! > No production code may branch on the identity of a specification. It
//! > branches on declared properties, which a source SCE has never seen can
//! > supply too.
//!
//! # The discriminator is the position, not the word
//!
//! A word ban would be the opposite of the goal. `provenance.rs` says in a
//! comment that its rule must accept `3.DoIP-152` because ISO 13400-2
//! numbers its requirements that way, and `requirement_manifest.rs` carries
//! two dozen citations of the same kind: each is the measurement a rule was
//! derived from, and deleting them would leave the rules standing with their
//! reasons gone. So the citation is exactly what must survive, and what this
//! gate looks at is whether a standard's name reaches a position that RUNS.
//!
//! Every file is therefore lexed before it is searched, and the comment
//! prose is blanked in place. [`the_position_split_is_what_this_gate_asks`]
//! is what holds that claim up: it drives a designation through both
//! positions in every language the sweep reads, and fails if either answer
//! moves. Without it a stripper that blanked whole files would report a
//! clean tree, which is this repository's most-repeated shape of green.
//!
//! A string literal counts as a position that runs. `t == "someip"` is a
//! branch, and `"an RFC 4122 canonical UUID"` is a sentence SCE's output
//! depends on a standard to write; both are things the registry below should
//! have to argue for, and neither is a comment.
//!
//! # Why the sweep is in two halves, and what each one is worth
//!
//! **A designation is found generatively.** A standards body's token
//! followed by its number — `ISO 13400-2`, `IEC 61131-3`, `SAE J1939`,
//! `IEEE 802.1Q` — is a shape, not a list. The bodies are a closed set and
//! the numbers are open, so a specification SCE has never seen is caught the
//! first time anyone writes its name. This half is the one that makes the
//! rule general, and it is the half that guards the requirement-closure
//! path, because a requirements document is cited by its designation.
//!
//! **A protocol nickname is found only if it is registered.** `SOME/IP`,
//! `DDS` and `zenoh` have no generative shape; nothing distinguishes them
//! from any other word. So the roster below is not a discovery mechanism and
//! this gate does not pretend otherwise: what it buys is CONFINEMENT. SCE
//! implements all three wire formats, so mesh may name them the way it names
//! TCP — and naming one anywhere else, above all in the closure path, is the
//! defect the rule exists for. A nickname absent from the roster is not
//! detected at all, and the honest reading of a green run is "no registered
//! nickname left its scope", not "no protocol is named anywhere".
//!
//! ## What earns a place on the roster
//!
//! Three of the four conditions are enforced below rather than trusted, so
//! membership is a rule and not a taste:
//!
//! 1. SCE's own code produces or consumes the wire form. This one is the
//!    reviewer's, and the entry's prose is where the case gets made.
//! 2. Confining the name is meaningful — its scope is narrower than the
//!    tree ([`no_entry_exempts_more_than_a_part_of_the_tree`]). This is
//!    what keeps `JSON`, `HTTP` and `TCP` out without needing an opinion
//!    about them: an exemption they would need is the whole tree, and an
//!    exemption that wide asserts nothing.
//! 3. Every path in that scope still names it
//!    ([`every_entry_is_still_carrying_weight`]).
//! 4. None of it reaches the requirement-closure path
//!    ([`no_entry_reaches_the_requirement_closure_path`]).
//!
//! ⚠ A first draft of this file kept `zenoh` out on the ground that it has
//! no standards body. That line does not survive the rule above — SCE emits
//! the protocol either way — and it was convenience dressed as principle:
//! Zenoh's scope is simply longer than the others'. It is in.
//!
//! # The allowlist is this gate's body
//!
//! [`IMPLEMENTED`] is where the argument gets written down, and four of its
//! properties are held rather than hoped for: an entry must carry prose
//! ([`every_entry_carries_an_argument`]), its scope must name paths that
//! exist and must not cover the tree ([`no_entry_exempts_more_than_a_part_of_the_tree`]),
//! it must still be earning its place ([`every_entry_is_still_carrying_weight`]),
//! and it may not reach the requirement-closure path
//! ([`no_entry_reaches_the_requirement_closure_path`]). Whether the prose is
//! a GOOD argument is a reviewer's judgement and this gate does not claim to
//! make it; what it refuses is a silent exemption, an exemption that outlived
//! its implementation, and an exemption wide enough to cover the code the
//! rule is about.
//!
//! # Measured before this gate landed, 2026-09-12
//!
//! Production positions across every tracked source file: **2** standards
//! designations (`RFC 4122`, and a citation of this repository's own
//! synthesis RFC that reads like one — see [`is_this_repos_own_rfc_section`]),
//! and every protocol nickname inside mesh. Behavioural dependencies on a
//! specification's identity: **0**. The gate is cheap now and expensive
//! later, which is the whole argument for building it before the next
//! specification is admitted rather than after.
//!
//! # What this gate cannot see
//!
//! A branch that reaches a specification's identity without writing its name
//! — `if doc_id == manifest.primary_id()` — is invisible to any scan of the
//! text, and no floor here would catch it. That is a reviewer's job, and
//! saying so is better than a check that implies otherwise.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::rust_source::{code_mask, code_only};

// ---------------------------------------------------------------------------
// The rule's population
// ---------------------------------------------------------------------------

/// Standards bodies, as a designation spells them.
///
/// A closed set on purpose: these are the organisations that publish
/// numbered specifications, and the number after the token is what makes the
/// pair a designation. Longer spellings come first so `ISO/IEC 14882` is not
/// read as `ISO` followed by rubbish.
///
/// ⚠ `EN`, `UL` and `BS` are real bodies and are deliberately absent. Each is
/// a short token that occurs in code for unrelated reasons, and a body that
/// fires on `EN 1` in an enum would spend the gate's credibility on noise. A
/// designation from one of them, named in code, is caught by review rather
/// than here — which is a gap, and is written down instead of papered over.
const BODIES: &[&str] = &[
    "ISO/IEC/IEEE",
    "ISO/IEC",
    "ISO",
    "IEC",
    "IEEE",
    "SAE",
    "ETSI",
    "ITU-T",
    "ITU",
    "ANSI",
    "RFC",
    "AUTOSAR",
    "OMG",
    "ASAM",
    "MISRA",
    "NIST",
    "FIPS",
    "ARINC",
    "DIN",
    "JIS",
    "GOST",
];

/// One specification SCE implements, and the argument for naming it in code.
struct Implemented {
    /// The name as code spells it, matched case-insensitively at an
    /// identifier boundary. `someip` also matches `SomeipTransport` and
    /// `vsomeip`, because a type named after a wire format names it.
    token: &'static str,
    /// Path prefixes where naming it is admitted. A prefix ending in `/` is
    /// a directory; anything else is one file.
    scope: &'static [&'static str],
    /// Why SCE may name this specification at all — what its own code does
    /// with the wire format. Required, and printed whenever the gate
    /// refuses, so the next reader meets the argument and not just the list.
    why: &'static str,
}

/// The allowlist. Read the module header before adding a row.
///
/// The line an entry has to be on the right side of: SCE *implements* this
/// specification — its own code produces or consumes the wire form — as
/// against SCE *depending on the specification for meaning*, which is the
/// thing the rule forbids and which no entry here may license.
const IMPLEMENTED: &[Implemented] = &[
    Implemented {
        token: "someip",
        scope: &[
            "sce-build/src/mesh/",
            "sce-build/src/lib.rs",
            "sce-build/src/forge/diagnostic.rs",
            "sce/include/mesh/",
            "tools/codegen/templates/mesh/",
        ],
        why: "SCE emits the SOME/IP wire format: mesh/transport/someip.rs \
              plans the service and event ids, vsomeip_config.rs writes the \
              stack's configuration, and templates/mesh/cpp emits the \
              endpoint that speaks it. Naming it here is naming what this \
              code produces, the way the same files name TCP. The two \
              outliers are deliberate — lib.rs routes a deploy document's \
              declared transport to that planner, and forge/diagnostic.rs is \
              where every diagnostic code in the tree is declared, mesh's \
              among them.",
    },
    Implemented {
        token: "some/ip",
        scope: &["sce-build/src/mesh/", "tools/codegen/templates/mesh/"],
        why: "The same wire format as `someip`, in the spelling a diagnostic \
              message shows a user. Two paths and not five: this entry was \
              first given `someip`'s scope on the argument that both \
              spellings should be confined alike, and the per-path check \
              refuted it — the slashed spelling is written in the mesh \
              planner and the mesh template, and nowhere else. A scope \
              reasoned from a neighbour rather than measured is the thing \
              that check exists to find.",
    },
    Implemented {
        token: "dds",
        scope: &[
            "sce-build/src/mesh/",
            "sce-build/src/lib.rs",
            "sce-build/src/forge/diagnostic.rs",
            "sce/include/mesh/",
            "tools/codegen/templates/mesh/",
        ],
        why: "SCE emits DDS: transport/mod.rs plans the QoS overlay and \
              partition mapping, sce/include/mesh/transports/DdsTransport.h \
              and templates/mesh/cpp carry the publisher and subscriber that \
              speak it. Same argument and same scope as SOME/IP.",
    },
    Implemented {
        token: "zenoh",
        scope: &[
            "sce-build/src/mesh/",
            "sce-build/src/lib.rs",
            "sce-build/src/generator.rs",
            "sce-build/src/forge/generator.rs",
            "sce/include/mesh/",
            "tools/codegen/templates/mesh/",
        ],
        why: "SCE speaks Zenoh: transport/zenoh.rs plans the key \
              expressions and liveliness tokens, and templates/mesh/cpp \
              emits the session that publishes and queries over them. It is \
              admitted on the same argument as SOME/IP and DDS rather than \
              on having a standards body, which is the line a first draft \
              drew and could not defend. The last two paths are one \
              diagnostic sentence each, not implementation: the generators \
              explain a liveliness rule and a deferral in prose the user \
              reads at run time.",
    },
    Implemented {
        token: "RFC 4122",
        scope: &["tools/codegen/templates/mesh/"],
        why: "SCE generates UUIDs in the canonical form that document \
              defines, and the mesh transport says so when it refuses a \
              malformed one. The name appears because SCE produces the \
              format, not because anything branches on the document.",
    },
];

/// Modules implementing requirement closure, which no entry above may reach.
///
/// This is the scope check with the sharpest teeth, and it is what keeps the
/// allowlist from quietly becoming the rule's exception. An entry scoped
/// `sce-build/src/` would be a reasonable-looking widening that admitted a
/// specification's name into exactly the code whose whole purpose is to not
/// know which specification it is reading.
///
/// ⚠ These files are where a designation legitimately appears in PROSE, and
/// densely: `requirement_manifest.rs` alone cites ISO 13400-2 fifteen times
/// to record what each of its rules was measured against. The rule this list
/// enforces is about the other position, and the comments stay.
const CLOSURE_PATH: &[&str] = &[
    "sce-build/src/provenance.rs",
    "sce-build/src/requirement_manifest.rs",
    "sce-build/src/requirements_report.rs",
    "sce-build/src/transition_table.rs",
    "sce-build/src/forge/provenance.rs",
];

/// Path segments that mark a test tree.
///
/// A test names a real standard on purpose — `iso13400_requirement_closure`
/// could not be written otherwise — and a rule about production code that
/// failed its own suite's fixtures would be a rule nobody could keep.
///
/// ⚠ DERIVED, and the first version of this was not. It was a hand-written
/// list of test directories, and it named SIX of the sixteen this tree
/// actually has: `backends/rust/tests`, `backends/kotlin/tests`,
/// `backends/go/tests`, `backends/python/tests`, four `forge-runtime/tests`
/// and `backends/kotlin/lowered-ecma262/src/test` were all swept as
/// production — over two thousand files. It surfaced by accident, when a
/// probe reported eighteen refusals inside a directory the list had never
/// heard of. This is the failure `spec_citations_carry_no_line_numbers`
/// names in its own header, where a hand-kept list reported full coverage
/// while checking 8 % of the tree, arriving again one gate later.
///
/// The segment is the convention the tree already follows, including the
/// JVM spelling `src/test/`, so a test directory added anywhere is covered
/// on the day it appears. [`the_test_trees_are_derived_not_listed`] is what
/// keeps that true.
const TEST_TREE_SEGMENTS: &[&str] = &["tests", "test"];

/// Trees excluded for a reason other than holding tests.
const NOT_PRODUCTION: &[&str] = &[
    // Vendored code SCE did not write and does not maintain. Naming a
    // standard there is that project's decision, not this one's.
    "third_party/",
    // Build outputs that happen to be tracked.
    "build/",
    "target/",
];

/// Whether the sweep reads this path.
fn is_production(path: &str) -> bool {
    !NOT_PRODUCTION.iter().any(|x| path.starts_with(x))
        && !path
            .split('/')
            .any(|segment| TEST_TREE_SEGMENTS.contains(&segment))
}

/// The path up to and including the segment that marks it as a test tree.
fn test_tree_of(path: &str) -> Option<String> {
    let mut prefix = String::new();
    for segment in path.split('/') {
        prefix.push_str(segment);
        prefix.push('/');
        if TEST_TREE_SEGMENTS.contains(&segment) {
            return Some(prefix);
        }
    }
    None
}

/// Source extensions the sweep reads, each mapped to its lexical rules.
///
/// A file whose extension is absent is not read, and that is the honest half
/// of the coverage claim: [`the_sweep_reads_the_tree_it_claims_to`] pins how
/// many files each language contributes, so a language dropping out of the
/// enumeration is a red rather than a quieter green.
fn language_of(path: &str) -> Option<Lang> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "rs" => Some(Lang::Rust),
        "c" | "h" | "cpp" | "hpp" | "cc" | "inl" => Some(Lang::CFamily),
        "go" => Some(Lang::Go),
        "kt" | "kts" => Some(Lang::Kotlin),
        "py" => Some(Lang::Python),
        "jinja2" => Some(target_language_of_template(path)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Lexing: where does a comment end and code begin
// ---------------------------------------------------------------------------

/// How a language spells a comment and a string.
///
/// Rust is its own arm because the suite already owns a Rust lexer
/// ([`common::rust_source`]) that knows raw strings, lifetimes and nested
/// block comments, and a second answer to where a Rust string ends is the
/// duplication this repository forbids.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    Rust,
    CFamily,
    Go,
    Kotlin,
    Python,
    /// A template: `{# #}` on top of the target language's own rules.
    Template(Target),
}

/// The language a template emits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    CFamily,
    Go,
    Kotlin,
    Python,
    Rust,
}

/// The directory that says which language a template emits.
///
/// Derived rather than declared, because the template tree already says it:
/// every backend has its own directory.
const TEMPLATE_DIRS: &[(&str, Target)] = &[
    ("/forge/c/", Target::CFamily),
    ("/forge/cpp/", Target::CFamily),
    ("/forge/go/", Target::Go),
    ("/forge/kotlin/", Target::Kotlin),
    ("/forge/python/", Target::Python),
    ("/forge/rust/", Target::Rust),
    ("/mesh/cpp/", Target::CFamily),
    ("/templates/c/", Target::CFamily),
    ("/templates/go/", Target::Go),
    ("/templates/kotlin/", Target::Kotlin),
    ("/templates/python/", Target::Python),
    ("/templates/rust/", Target::Rust),
];

/// Where the C++ backend's own templates live. They predate the
/// per-language directories and so have none of their own.
const CPP_TEMPLATE_ROOTS: &[&str] = &[
    "tools/codegen/templates/_macros/",
    "tools/codegen/templates/actions/",
];

/// Which language a template emits, when its directory says.
fn declared_template_target(path: &str) -> Option<Target> {
    TEMPLATE_DIRS
        .iter()
        .find(|(segment, _)| path.contains(segment))
        .map(|(_, target)| *target)
}

/// Whether a template is one of the C++ backend's, which carry no directory
/// of their own: the templates directly in the templates root, and the two
/// shared trees beside them.
fn is_cpp_backend_template(path: &str) -> bool {
    if CPP_TEMPLATE_ROOTS.iter().any(|r| path.starts_with(r)) {
        return true;
    }
    path.strip_prefix("tools/codegen/templates/")
        .is_some_and(|rest| !rest.contains('/'))
}

/// Which language a template emits.
///
/// [`every_template_resolves_to_a_target_language`] is what keeps the
/// fallback honest when a seventh backend arrives with a directory this
/// table does not know.
fn target_language_of_template(path: &str) -> Lang {
    Lang::Template(declared_template_target(path).unwrap_or(Target::CFamily))
}

/// Whether the derivation above knows this template, rather than having
/// fallen through to the C++ default by accident.
fn template_target_is_declared(path: &str) -> bool {
    declared_template_target(path).is_some() || is_cpp_backend_template(path)
}

/// The source with its comments blanked, line numbering and length kept.
///
/// Every arm preserves position: a scan that reports a line number reports
/// the input's. Comments become spaces rather than disappearing, which is
/// what lets the same offsets be used against the original text.
fn executable_text(source: &str, lang: Lang) -> String {
    match lang {
        Lang::Rust => elide_rust_test_modules(source),
        Lang::CFamily => strip(source, Syntax::c_family()),
        Lang::Go => strip(source, Syntax::go()),
        Lang::Kotlin => strip(source, Syntax::kotlin()),
        Lang::Python => strip(source, Syntax::python()),
        Lang::Template(target) => {
            let without_template_prose = strip(source, Syntax::jinja());
            let target_syntax = match target {
                Target::CFamily => Syntax::c_family(),
                Target::Go => Syntax::go(),
                Target::Kotlin => Syntax::kotlin(),
                Target::Python => Syntax::python(),
                Target::Rust => Syntax::rust_like(),
            };
            strip(&without_template_prose, target_syntax)
        }
    }
}

/// A language's comment and string delimiters.
struct Syntax {
    line: &'static [&'static str],
    block: &'static [(&'static str, &'static str)],
    /// Whether a block comment may contain another.
    nests: bool,
    /// String delimiters, longest first, paired with whether a backslash
    /// escapes inside them.
    ///
    /// ⚠ The single quote is absent from every C-family arm, and that is a
    /// measured decision rather than an oversight. C++ writes digit
    /// separators with it — `0x0000'FFFF'FFFF'FFFFULL` in
    /// `sce/src/common/Uuid.cpp` — and a lexer that read the third of those
    /// as a character literal ran the literal to the end of the file,
    /// swallowing eighteen lines of comment into what it called code. The
    /// first run of this gate reported the `RFC 9562` citation eight lines
    /// below it as a production branch, which is a false red on exactly the
    /// prose the rule promises to protect.
    ///
    /// Dropping it costs nothing here: a character literal holds ONE
    /// character, so it can contain neither `//` nor `/*` nor `#`, and no
    /// comment can hide inside one. Python keeps its `'` because there it
    /// delimits a full string, and Python has no digit separator spelled
    /// that way.
    strings: &'static [(&'static str, bool)],
    /// Delimiters whose content reads as prose when the literal begins a
    /// line — Python's docstring, which is that language's form of the
    /// citation this gate exists to protect.
    prose_when_line_initial: &'static [&'static str],
    /// C++ raw strings: `R"delim( ... )delim"`.
    cpp_raw: bool,
}

impl Syntax {
    fn c_family() -> Self {
        Syntax {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: false,
            strings: &[("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: true,
        }
    }
    fn go() -> Self {
        Syntax {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: false,
            strings: &[("`", false), ("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: false,
        }
    }
    fn kotlin() -> Self {
        // Kotlin's `"""` is an ordinary string; its documentation comment is
        // `/** */`, which the block arm already covers.
        Syntax {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: true,
            strings: &[("\"\"\"", false), ("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: false,
        }
    }
    fn python() -> Self {
        Syntax {
            line: &["#"],
            block: &[],
            nests: false,
            strings: &[("\"\"\"", false), ("'''", false), ("\"", true), ("'", true)],
            prose_when_line_initial: &["\"\"\"", "'''"],
            cpp_raw: false,
        }
    }
    /// Rust as reached through a template, where [`common::rust_source`]
    /// cannot be used because the text is not a Rust file yet.
    fn rust_like() -> Self {
        Syntax {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: true,
            strings: &[("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: false,
        }
    }
    /// Jinja's own prose. `{{ }}` and `{% %}` are code and are left alone;
    /// only `{# #}` is a comment.
    fn jinja() -> Self {
        Syntax {
            line: &[],
            block: &[("{#", "#}")],
            nests: false,
            strings: &[],
            prose_when_line_initial: &[],
            cpp_raw: false,
        }
    }
}

/// Blank every comment, leaving code and string literals in place.
fn strip(source: &str, syntax: Syntax) -> String {
    let s: Vec<char> = source.chars().collect();
    let mut out: Vec<char> = s.clone();
    let mut i = 0usize;
    while i < s.len() {
        // A string literal first: a `//` inside one is data, and `http://`
        // in a URL is the case that proves it.
        if let Some((delim, escapes)) = opening_string(&s, i, &syntax) {
            let end = end_of_string(&s, i, delim, escapes);
            if syntax.prose_when_line_initial.contains(&delim) && begins_a_line(&s, i) {
                for slot in out.iter_mut().take(end).skip(i) {
                    if *slot != '\n' {
                        *slot = ' ';
                    }
                }
            }
            i = end;
            continue;
        }
        if syntax.cpp_raw {
            if let Some(end) = end_of_cpp_raw_string(&s, i) {
                i = end;
                continue;
            }
        }
        if let Some(marker) = syntax.line.iter().find(|m| starts_with(&s, i, m)) {
            let _ = marker;
            while i < s.len() && s[i] != '\n' {
                out[i] = ' ';
                i += 1;
            }
            continue;
        }
        if let Some((open, close)) = syntax.block.iter().find(|(o, _)| starts_with(&s, i, o)) {
            let mut depth = 1usize;
            let start = i;
            i += open.chars().count();
            while i < s.len() && depth > 0 {
                if syntax.nests && starts_with(&s, i, open) {
                    depth += 1;
                    i += open.chars().count();
                } else if starts_with(&s, i, close) {
                    depth -= 1;
                    i += close.chars().count();
                } else {
                    i += 1;
                }
            }
            for slot in out.iter_mut().take(i).skip(start) {
                if *slot != '\n' {
                    *slot = ' ';
                }
            }
            continue;
        }
        i += 1;
    }
    out.into_iter().collect()
}

/// Whether `needle` sits at `at`.
///
/// Written without collecting the needle: this runs once per character of
/// every tracked source file, and allocating there put the first run of this
/// gate at 26 seconds inside a lane whose whole budget is already the
/// largest a push can carry.
fn starts_with(s: &[char], at: usize, needle: &str) -> bool {
    for (i, nc) in (at..).zip(needle.chars()) {
        if i >= s.len() || s[i] != nc {
            return false;
        }
    }
    true
}

/// Whether only whitespace precedes `at` on its line.
///
/// What separates a Python docstring — the citation form this gate must
/// leave standing — from a triple-quoted literal used as a value. A branch
/// on a specification's identity is never written as a line-initial
/// docstring, so the direction this rule can be wrong in is not the
/// direction that matters.
fn begins_a_line(s: &[char], at: usize) -> bool {
    let mut i = at;
    while i > 0 {
        i -= 1;
        if s[i] == '\n' {
            return true;
        }
        if !s[i].is_whitespace() {
            return false;
        }
    }
    true
}

/// The string delimiter opening at `at`, if one does.
fn opening_string(s: &[char], at: usize, syntax: &Syntax) -> Option<(&'static str, bool)> {
    // A quote that is really a Rust-style lifetime or a Kotlin character
    // cannot arise here: the arms that use `'` as a delimiter are the
    // languages where it always is one.
    syntax
        .strings
        .iter()
        .find(|(d, _)| starts_with(s, at, d))
        .copied()
}

/// Where the string opening at `at` ends, one past its closing delimiter.
fn end_of_string(s: &[char], at: usize, delim: &str, escapes: bool) -> usize {
    let width = delim.chars().count();
    let mut i = at + width;
    while i < s.len() {
        if escapes && s[i] == '\\' {
            i += 2;
            continue;
        }
        if starts_with(s, i, delim) {
            return i + width;
        }
        i += 1;
    }
    s.len()
}

/// Where a C++ raw string `R"delim(` opening at `at` ends, if one does.
fn end_of_cpp_raw_string(s: &[char], at: usize) -> Option<usize> {
    if s[at] != 'R' || at + 1 >= s.len() || s[at + 1] != '"' {
        return None;
    }
    if at > 0 && (s[at - 1].is_alphanumeric() || s[at - 1] == '_') {
        return None;
    }
    let mut j = at + 2;
    let mut delim = String::new();
    while j < s.len() && s[j] != '(' {
        delim.push(s[j]);
        j += 1;
    }
    if j >= s.len() {
        return None;
    }
    let closing = format!("){delim}\"");
    let mut k = j + 1;
    while k < s.len() {
        if starts_with(s, k, &closing) {
            return Some(k + closing.chars().count());
        }
        k += 1;
    }
    Some(s.len())
}

/// Rust source with its comments and its `#[cfg(test)]` modules blanked.
///
/// Test modules live inside `src/` in this language, so a sweep over
/// production code that read them would report a test's fixture as a
/// production branch — which is precisely the four hits the measurement in
/// the module header had to set aside by hand.
///
/// The structure is read from [`code_mask`], whose literals are blanked, so
/// a brace inside a string cannot close a module early; the text is taken
/// from [`code_only`], which keeps them. The two are the same length and the
/// same line numbering, which is what lets one index both.
fn elide_rust_test_modules(source: &str) -> String {
    let code: Vec<char> = code_only(source).chars().collect();
    let mask: Vec<char> = code_mask(source).chars().collect();
    // The two answers are documented as the same length, and every offset
    // below is taken from one and applied to the other. Asserted rather than
    // assumed: if that ever stopped holding, this would blank the wrong span
    // and the sweep would go quiet in the direction nothing else notices.
    assert_eq!(
        code.len(),
        mask.len(),
        "code_only and code_mask disagree on length; offsets taken from one \
         cannot be applied to the other"
    );
    let mut out = code.clone();
    let attr = "#[cfg(test)]";
    let mut i = 0usize;
    while i < mask.len() {
        if !starts_with(&mask, i, attr) {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i + attr.chars().count();
        // An attribute may govern an item with a body or one without. A `;`
        // before the first `{` means the latter — `#[cfg(test)] use x;` —
        // and blanking to the next brace would eat the item after it.
        let mut end = mask.len();
        let mut depth = 0usize;
        let mut opened = false;
        while j < mask.len() {
            match mask[j] {
                ';' if !opened => {
                    end = j + 1;
                    break;
                }
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => {
                    depth -= 1;
                    if opened && depth == 0 {
                        end = j + 1;
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        for slot in out.iter_mut().take(end).skip(start) {
            if *slot != '\n' {
                *slot = ' ';
            }
        }
        i = end;
    }
    out.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Matching: what reads as a specification's identity
// ---------------------------------------------------------------------------

/// One name found in a position that runs.
#[derive(Debug, Clone)]
struct Hit {
    path: String,
    line: usize,
    /// The text as the file spells it.
    text: String,
    /// The registry token this matches, for a nickname; `None` for a
    /// designation found generatively.
    token: Option<&'static str>,
}

/// Whether the character before `at` ends a word.
///
/// A lowercase-to-uppercase step counts, so `parseIso13400` is found. A type
/// named after a standard names it just as surely as a string does.
fn at_word_start(s: &[char], at: usize) -> bool {
    if at == 0 {
        return true;
    }
    let prev = s[at - 1];
    if !(prev.is_alphanumeric() || prev == '_') {
        return true;
    }
    (prev.is_lowercase() || prev.is_ascii_digit()) && s[at].is_uppercase()
}

/// Whether this repository's own design document is what is being cited.
///
/// `docs/spec/synth/rfc-sce-protocol-synthesis.md` is cited throughout the
/// tree as `RFC <section>`, and a section carries a dot where an IETF number
/// never does: `RFC 5.E` against `RFC 4122`. `RFC` is the only body token
/// that is also this repository's word for its own documents, so the
/// exception is exactly one token wide and is checked in
/// [`the_position_split_is_what_this_gate_asks`].
fn is_this_repos_own_rfc_section(body: &str, s: &[char], after_digits: usize) -> bool {
    body == "RFC" && after_digits < s.len() && s[after_digits] == '.'
}

/// Every standards designation in `text`, as (offset, spelling).
fn designations(text: &str) -> Vec<(usize, String)> {
    let s: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut i = 0usize;
    'outer: while i < s.len() {
        // Every body token starts with an ASCII letter, and most positions
        // in a source file are not one. Checked before the body loop so the
        // sweep costs one comparison per character rather than twenty-one.
        if !s[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        for body in BODIES {
            let width = body.chars().count();
            if !starts_with_ignoring_case(&s, i, body) || !at_word_start(&s, i) {
                continue;
            }
            // Whatever follows the body: separators, then an optional short
            // series prefix (`TS`, `TR`, `J`), then the number itself.
            let mut j = i + width;
            let mut sep = 0;
            while j < s.len() && matches!(s[j], ' ' | '-' | '_' | '/' | '\u{a0}') {
                j += 1;
                sep += 1;
            }
            if sep > 1 {
                continue;
            }
            let prefix_start = j;
            let mut letters = 0;
            while j < s.len() && s[j].is_ascii_uppercase() && letters < 2 {
                j += 1;
                letters += 1;
            }
            if letters > 0 {
                let mut sep2 = 0;
                while j < s.len() && matches!(s[j], ' ' | '-' | '_') {
                    j += 1;
                    sep2 += 1;
                }
                if sep2 > 1 {
                    j = prefix_start;
                }
            }
            let digits_start = j;
            while j < s.len() && s[j].is_ascii_digit() {
                j += 1;
            }
            if j == digits_start {
                continue;
            }
            if is_this_repos_own_rfc_section(body, &s, j) {
                continue;
            }
            // The rest of the designation: `13400-2:2019`, `802.1Q`.
            while j < s.len()
                && (s[j].is_ascii_alphanumeric() || matches!(s[j], '-' | '.' | ':'))
                && !(s[j] == '.' && j + 1 < s.len() && !s[j + 1].is_ascii_alphanumeric())
            {
                j += 1;
            }
            found.push((i, s[i..j].iter().collect()));
            i = j;
            continue 'outer;
        }
        i += 1;
    }
    found
}

fn starts_with_ignoring_case(s: &[char], at: usize, needle: &str) -> bool {
    for (i, nc) in (at..).zip(needle.chars()) {
        if i >= s.len() || !s[i].eq_ignore_ascii_case(&nc) {
            return false;
        }
    }
    true
}

/// Every registered nickname in `text`, as (offset, token).
fn nicknames(text: &str) -> Vec<(usize, &'static str)> {
    let s: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    for entry in IMPLEMENTED.iter().filter(|e| is_nickname(e.token)) {
        let width = entry.token.chars().count();
        let first = entry.token.chars().next().expect("a token is not empty");
        let mut i = 0usize;
        while i + width <= s.len() {
            if s[i].eq_ignore_ascii_case(&first)
                && starts_with_ignoring_case(&s, i, entry.token)
                && at_word_start(&s, i)
            {
                found.push((i, entry.token));
                i += width;
            } else {
                i += 1;
            }
        }
    }
    found
}

/// Whether a registry token is a nickname rather than a designation.
///
/// A designation carries its standards body's number, and the generative
/// half already finds it; only a nickname needs the roster pass.
fn is_nickname(token: &str) -> bool {
    !token.chars().any(|c| c.is_ascii_digit())
}

/// Whether this hit's text names this entry's specification.
///
/// A nickname hit spells the token exactly; a designation hit opens with it
/// (`RFC 4122`). Both are ASCII by construction, so `get` cannot land off a
/// character boundary — and it answers `None` rather than panicking if that
/// ever stopped being true.
fn names(entry: &Implemented, text: &str) -> bool {
    text.get(..entry.token.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(entry.token))
}

/// Which registry entry admits `text` at `path`, if any.
fn admitted_by(path: &str, text: &str) -> Option<&'static Implemented> {
    IMPLEMENTED
        .iter()
        .find(|entry| names(entry, text) && entry.scope.iter().any(|s| path.starts_with(s)))
}

/// Character offsets of every newline, so a file's hits are numbered once
/// rather than by re-counting from the top for each one.
fn newline_offsets(text: &str) -> Vec<usize> {
    text.chars()
        .enumerate()
        .filter(|(_, c)| *c == '\n')
        .map(|(i, _)| i)
        .collect()
}

fn line_of(newlines: &[usize], offset: usize) -> usize {
    newlines.partition_point(|n| *n < offset) + 1
}

// ---------------------------------------------------------------------------
// The sweep
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every tracked production source file, with its language.
///
/// `git ls-files` rather than a directory walk: the claim this gate makes is
/// about the tree as committed, and an untracked scratch file is not part of
/// it. That also makes this gate's inputs wider than any `paths:` filter,
/// which is why it is registered in `UNFILTERABLE_GATES`.
fn production_sources(root: &Path) -> Vec<(String, Lang)> {
    tracked_files(root)
        .into_iter()
        .filter(|p| is_production(p))
        .filter_map(|p| language_of(&p).map(|l| (p, l)))
        .collect()
}

/// Every path `git` tracks.
fn tracked_files(root: &Path) -> Vec<String> {
    let out = Command::new("git")
        .args(["-C", &root.display().to_string(), "ls-files"])
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// Every name in a position that runs, in ONE file.
///
/// Split out from the sweep because the sweep can only ever come back
/// green, and a green from a scan is this repository's most-repeated shape
/// of measuring nothing. [`the_decision_is_exercised_where_it_fails`]
/// drives this same function — the whole chain of lex, match and ask the
/// registry — over sources written down here, so the decision is exercised
/// in the direction the tree cannot exercise it.
fn hits_in(path: &str, source: &str, lang: Lang) -> Vec<Hit> {
    let code = executable_text(source, lang);
    let newlines = newline_offsets(&code);
    let mut hits: Vec<Hit> = designations(&code)
        .into_iter()
        .map(|(at, text)| Hit {
            path: path.to_string(),
            line: line_of(&newlines, at),
            text,
            token: None,
        })
        .collect();
    hits.extend(nicknames(&code).into_iter().map(|(at, token)| Hit {
        path: path.to_string(),
        line: line_of(&newlines, at),
        text: token.to_string(),
        token: Some(token),
    }));
    hits
}

/// One refusal, as the gate reports it.
fn describe(hit: &Hit) -> String {
    let kind = match hit.token {
        Some(_) => "a registered protocol outside its scope",
        None => "a standards designation",
    };
    format!("{}:{}: {} — `{}`", hit.path, hit.line, kind, hit.text)
}

/// The hits in one file that no registry entry admits.
///
/// The single place the rule is decided. Both the tree-wide sweep and the
/// worked cases go through it, so there is one answer to "is this
/// refused?" rather than two that can drift apart.
fn refusals_in(path: &str, source: &str, lang: Lang) -> Vec<Hit> {
    hits_in(path, source, lang)
        .into_iter()
        .filter(|hit| admitted_by(&hit.path, &hit.text).is_none())
        .collect()
}

/// Every name in a position that runs, across production source.
fn sweep(root: &Path) -> (Vec<Hit>, BTreeMap<&'static str, usize>) {
    let mut hits = Vec::new();
    let mut per_language: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (path, lang) in production_sources(root) {
        let Ok(source) = std::fs::read_to_string(root.join(&path)) else {
            continue;
        };
        *per_language.entry(language_name(lang)).or_default() += 1;
        hits.extend(hits_in(&path, &source, lang));
    }
    (hits, per_language)
}

fn language_name(lang: Lang) -> &'static str {
    match lang {
        Lang::Rust => "rust",
        Lang::CFamily => "c-family",
        Lang::Go => "go",
        Lang::Kotlin => "kotlin",
        Lang::Python => "python",
        Lang::Template(_) => "template",
    }
}

/// Measured floors on the sweep's reach, per language.
///
/// A sweep that stops enumerating reports a clean tree, and every arm
/// carries its own floor: a single total would let one language go blind
/// while another's count made up the number — the failure this suite has
/// already been caught by once, where a sweep arm went dark and a
/// per-target arm kept the sum respectable.
///
/// Measured 2026-09-12, at roughly 90 % of what the tree holds:
/// rust 153, c-family 598, go 45, kotlin 59, python 43, template 270.
///
/// ⚠ These are much smaller than the first numbers written here — rust
/// alone fell from 969 — and the difference is not a tree that shrank. It
/// is [`TEST_TREE_SEGMENTS`] becoming a derivation instead of a list, which
/// took roughly eight hundred test files back out of a population that had
/// been calling itself production.
const FILE_FLOORS: &[(&str, usize)] = &[
    ("rust", 137),
    ("c-family", 538),
    ("go", 40),
    ("kotlin", 53),
    ("python", 38),
    ("template", 243),
];

// ---------------------------------------------------------------------------
// The rule
// ---------------------------------------------------------------------------

/// No production code names a specification SCE does not implement.
#[test]
fn production_code_names_no_standard_outside_the_allowlist() {
    let root = repo_root();
    let (hits, _) = sweep(&root);

    let mut refused: Vec<String> = hits
        .iter()
        .filter(|hit| admitted_by(&hit.path, &hit.text).is_none())
        .map(describe)
        .collect();
    refused.sort();

    assert!(
        refused.is_empty(),
        "production code names a specification in a position that RUNS:\n\n{}\n\n\
         No production code may branch on the identity of a specification; it \
         branches on declared properties, which a source SCE has never seen can \
         supply too. A comment citing the document a rule came from is not this \
         — the citation is the rule's evidence and must stay. If SCE genuinely \
         IMPLEMENTS this specification — its own code produces or consumes the \
         wire form, the way mesh emits SOME/IP — add it to IMPLEMENTED in \
         {} with the scope it is confined to and the argument for it. \
         If SCE merely READS documents written against it, the name does not \
         belong in code at all: put the behaviour behind a property the \
         manifest declares.",
        refused.join("\n"),
        file!(),
    );
}

/// No allowlist entry reaches the requirement-closure path.
///
/// The rule's whole subject is that this code does not know which
/// specification it is reading, so an exemption that covered it would be the
/// defect wearing the gate's own clothes.
#[test]
fn no_entry_reaches_the_requirement_closure_path() {
    let mut overlaps = Vec::new();
    for entry in IMPLEMENTED {
        for scope in entry.scope {
            for closure in CLOSURE_PATH {
                if closure.starts_with(scope) || scope.starts_with(closure) {
                    overlaps.push(format!(
                        "  `{}` is scoped `{scope}`, which covers {closure}",
                        entry.token
                    ));
                }
            }
        }
    }
    assert!(
        overlaps.is_empty(),
        "an allowlist entry reaches the requirement-closure path:\n{}\n\n\
         These modules are the ones the rule exists for: they read a \
         requirements manifest without knowing which standard wrote it. No \
         exemption may name a specification inside them.",
        overlaps.join("\n"),
    );

    // The list is only a guard while it names files that exist.
    let root = repo_root();
    for closure in CLOSURE_PATH {
        assert!(
            root.join(closure).exists(),
            "CLOSURE_PATH names {closure}, which is not in the tree — the \
             module moved and this guard has been pointing at nothing since. \
             Re-derive the list from the modules that read a requirement \
             manifest.",
        );
    }
}

/// Every entry is still earning its exemption — path by path.
///
/// An exemption that outlives its implementation is worse than none: it
/// reads as a considered decision while licensing a name nothing produces
/// any more.
///
/// ⚠ The check is PER SCOPE PATH, and the weaker version — "the entry is
/// used somewhere" — was written first and is the reason this note exists.
/// A scope is a list, and a list checked only in aggregate can be padded:
/// one live path carries five dead ones, and each dead one is a directory
/// where the name may be written freely and nobody would learn it had
/// stopped being true. Per-path is what makes a long scope cost something
/// to keep.
#[test]
fn every_entry_is_still_carrying_weight() {
    let root = repo_root();
    let (hits, _) = sweep(&root);
    let mut unused = Vec::new();
    for entry in IMPLEMENTED {
        for scope in entry.scope {
            let used = hits
                .iter()
                .any(|h| h.path.starts_with(scope) && names(entry, &h.text));
            if !used {
                unused.push(format!(
                    "  `{}` is scoped `{scope}`, and is not named there",
                    entry.token
                ));
            }
        }
    }
    assert!(
        unused.is_empty(),
        "an allowlist scope admits a name nothing writes there:\n{}\n\n\
         Either the implementation moved, or the path was copied from a \
         neighbouring entry rather than measured. Drop it. A scope that is \
         not checked against the tree is a list of intentions, and every \
         stale path in one is a directory where the rule quietly does not \
         apply.",
        unused.join("\n"),
    );
}

/// The test trees are derived from the tree, not listed by hand.
///
/// The floor is the whole point. A derivation that stopped matching would
/// sweep every test fixture as production and turn this gate red on files
/// whose job is to name standards; one that matched too much would sweep
/// nothing and go quiet. Measured 2026-09-12: sixteen test trees, against
/// the six a hand-written list had named.
#[test]
fn the_test_trees_are_derived_not_listed() {
    const FLOOR: usize = 14;
    let excluded: std::collections::BTreeSet<String> = tracked_files(&repo_root())
        .into_iter()
        .filter(|p| !is_production(p))
        .filter_map(|p| test_tree_of(&p))
        .collect();
    assert!(
        excluded.len() >= FLOOR,
        "only {} test tree(s) are excluded, floor {FLOOR}: {:?}\n\n\
         Either the derivation stopped matching — in which case every \
         fixture that names a standard on purpose is about to be reported \
         as production code — or the tree genuinely lost test directories \
         and this floor should be re-derived in the same commit.",
        excluded.len(),
        excluded,
    );
}

/// Every entry carries an argument, and a scope that is a real part of the
/// tree rather than the whole of it.
#[test]
fn every_entry_carries_an_argument() {
    let root = repo_root();
    let mut bad = Vec::new();
    for entry in IMPLEMENTED {
        // A floor, not a judgement of the prose. What it refuses is the
        // silent entry — a row added with nothing said about why.
        if entry.why.split_whitespace().count() < 12 {
            bad.push(format!(
                "  `{}`: the argument is {} words. Say what SCE's own code \
                 does with this wire format, and where.",
                entry.token,
                entry.why.split_whitespace().count()
            ));
        }
        assert!(
            !entry.scope.is_empty(),
            "`{}` has no scope — an exemption without one covers the tree",
            entry.token
        );
        for scope in entry.scope {
            if !root.join(scope).exists() {
                bad.push(format!(
                    "  `{}`: scope `{scope}` is not in the tree",
                    entry.token
                ));
            }
        }
    }
    assert!(bad.is_empty(), "allowlist entries:\n{}", bad.join("\n"));
}

/// No entry's scope is wide enough to stop meaning anything.
///
/// An exemption scoped at a source root would admit the name everywhere the
/// rule is about. The bound is stated as a shape rather than a count: a
/// scope must name something below a crate's source root.
#[test]
fn no_entry_exempts_more_than_a_part_of_the_tree() {
    const TOO_WIDE: &[&str] = &[
        "",
        "/",
        "sce-build/",
        "sce-build/src/",
        "sce/",
        "sce/src/",
        "sce/include/",
        "backends/",
        "tools/",
        "tools/codegen/",
        "tools/codegen/templates/",
    ];
    let mut wide = Vec::new();
    for entry in IMPLEMENTED {
        for scope in entry.scope {
            if TOO_WIDE.contains(scope) {
                wide.push(format!("  `{}` is scoped `{scope}`", entry.token));
            }
        }
    }
    assert!(
        wide.is_empty(),
        "an allowlist entry covers a whole source root:\n{}\n\n\
         An exemption that wide licenses the name in code the rule is about. \
         Name the directories that actually implement the wire format.",
        wide.join("\n"),
    );
}

/// The sweep reads the tree it claims to, in every language.
#[test]
fn the_sweep_reads_the_tree_it_claims_to() {
    let root = repo_root();
    let (_, per_language) = sweep(&root);
    let mut short = Vec::new();
    for (lang, floor) in FILE_FLOORS {
        let seen = per_language.get(lang).copied().unwrap_or(0);
        if seen < *floor {
            short.push(format!("  {lang}: {seen} files, floor {floor}"));
        }
    }
    assert!(
        short.is_empty(),
        "the sweep read fewer files than it is meant to:\n{}\n\n\
         A green result from a sweep that stopped enumerating says nothing. \
         Either the enumeration broke, or the tree genuinely shed files and \
         the floor should be re-derived and lowered in the same commit.",
        short.join("\n"),
    );
}

/// Every template resolves to a target language that was declared.
///
/// The derivation falls back to C++ for the templates at the tree's root,
/// which is correct today and would silently absorb a seventh backend's
/// directory. This is what makes that arrival a red instead.
#[test]
fn every_template_resolves_to_a_target_language() {
    let root = repo_root();
    let undeclared: Vec<String> = production_sources(&root)
        .into_iter()
        .filter(|(p, l)| matches!(l, Lang::Template(_)) && !template_target_is_declared(p))
        .map(|(p, _)| format!("  {p}"))
        .collect();
    assert!(
        undeclared.is_empty(),
        "a template's target language is not declared:\n{}\n\n\
         The sweep infers a template's comment syntax from its directory, and \
         falls back to C++ for the templates that predate the per-language \
         directories. A template somewhere else would be lexed with the wrong \
         comment rules — reading its prose as code, or its code as prose. Add \
         the directory to BY_SEGMENT in {}.",
        undeclared.join("\n"),
        file!(),
    );
}

// ---------------------------------------------------------------------------
// The discriminator itself
// ---------------------------------------------------------------------------

/// A designation in a comment is invisible; the same designation in code is
/// a hit. In every language the sweep reads.
///
/// ⚠ This is the assertion the whole gate rests on. Every other test here
/// would pass on a stripper that blanked entire files — that is what a
/// vacuous green looks like in this shape of gate, and this is what refuses
/// it. The same cases run in the other direction too, because a stripper
/// that blanked nothing would turn every citation in `requirement_manifest.rs`
/// into a violation and make the rule unkeepable.
#[test]
fn the_position_split_is_what_this_gate_asks() {
    struct Case {
        lang: Lang,
        what: &'static str,
        /// Source whose ONLY designation sits in a comment.
        prose: &'static str,
        /// Source whose ONLY designation sits in a position that runs.
        code: &'static str,
    }

    let cases = &[
        Case {
            lang: Lang::Rust,
            what: "rust line comment",
            prose: "// this rule refuses ISO 13400-2\nlet x = 1;\n",
            code: "let doc = \"ISO 13400-2\";\n",
        },
        Case {
            lang: Lang::Rust,
            what: "rust block comment, nested",
            prose: "/* outer /* ISO 13400-2 */ still */\nlet x = 1;\n",
            code: "struct Iso13400Reader;\n",
        },
        Case {
            lang: Lang::CFamily,
            what: "c++ line comment",
            prose: "// see ISO 14229-1 §11\nint x = 1;\n",
            code: "const char *d = \"ISO 14229-1\";\n",
        },
        Case {
            lang: Lang::CFamily,
            what: "c++ raw string keeps its content",
            prose: "/* ISO 14229-1 */\nint x = 1;\n",
            code: "auto s = R\"json({\"doc\":\"ISO 14229-1\"})json\";\n",
        },
        Case {
            // The shape that turned this gate's first run red on a comment.
            // `sce/src/common/Uuid.cpp` separates the digits of a mask with
            // apostrophes; read as character literals, the third one ran to
            // the end of the file and took eighteen lines of prose with it.
            lang: Lang::CFamily,
            what: "c++ digit separator is not a character literal",
            prose: "auto m = 0x0000'FFFF'FFFF'FFFFULL;\n// RFC 9562 layout\nint x = 1;\n",
            code: "auto m = 0x0000'FFFF'FFFF'FFFFULL;\nconst char *d = \"RFC 9562\";\n",
        },
        Case {
            lang: Lang::Go,
            what: "go backtick string is not a comment",
            prose: "// ISO 13400-2 is cited here\nvar x = 1\n",
            code: "var s = `doc ISO 13400-2 here`\n",
        },
        Case {
            lang: Lang::Kotlin,
            what: "kotlin triple-quoted string",
            prose: "// ISO 13400-2\nval x = 1\n",
            code: "val s = \"\"\"doc ISO 13400-2\"\"\"\n",
        },
        Case {
            lang: Lang::Python,
            what: "python hash comment",
            prose: "# ISO 13400-2 says so\nx = 1\n",
            code: "d = 'ISO 13400-2'\n",
        },
        Case {
            lang: Lang::Python,
            what: "python docstring is prose, a call is not",
            prose: "\"\"\"Reads ISO 13400-2 manifests.\"\"\"\nx = 1\n",
            code: "load(doc='ISO 13400-2')\n",
        },
        Case {
            lang: Lang::Template(Target::CFamily),
            what: "jinja comment and the emitted language's comment",
            prose: "{# ISO 13400-2 #}\n// also ISO 14229-1\nint x = 1;\n",
            code: "{% if d %}const char *k = \"ISO 13400-2\";{% endif %}\n",
        },
        Case {
            lang: Lang::Template(Target::Python),
            what: "a python template's hash comment",
            prose: "# ISO 13400-2\nx = 1\n",
            code: "x = 'ISO 13400-2'\n",
        },
    ];

    let mut wrong = Vec::new();
    for case in cases {
        let in_prose = designations(&executable_text(case.prose, case.lang));
        if !in_prose.is_empty() {
            wrong.push(format!(
                "  {}: a designation in a COMMENT was reported as code ({:?}). \
                 A citation is a rule's evidence; reporting it makes the rule \
                 unkeepable.",
                case.what, in_prose
            ));
        }
        let in_code = designations(&executable_text(case.code, case.lang));
        if in_code.is_empty() {
            wrong.push(format!(
                "  {}: a designation in a position that RUNS was not \
                 reported. A stripper that blanks code turns this whole gate \
                 into a green that measured nothing.",
                case.what
            ));
        }
    }

    // `#[cfg(test)]` is production-looking text that is not production.
    let test_module = "#[cfg(test)]\nmod tests {\n    const D: &str = \"ISO 13400-2\";\n}\n";
    let seen = designations(&executable_text(test_module, Lang::Rust));
    if !seen.is_empty() {
        wrong.push(format!(
            "  rust cfg(test): a fixture inside a test module was reported as \
             production code ({seen:?})"
        ));
    }
    // ...and the same text outside one still is.
    let live = "const D: &str = \"ISO 13400-2\";\n";
    if designations(&executable_text(live, Lang::Rust)).is_empty() {
        wrong.push("  rust cfg(test): eliding test modules ate live code".to_string());
    }
    // An attribute on an item with no body must not swallow what follows it.
    let non_module = "#[cfg(test)]\nuse std::fmt;\nconst D: &str = \"ISO 13400-2\";\n";
    if designations(&executable_text(non_module, Lang::Rust)).is_empty() {
        wrong.push(
            "  rust cfg(test): an attribute on a bodyless item swallowed the \
             code after it"
                .to_string(),
        );
    }

    // This repository's own design document is cited as `RFC <section>`, and
    // must not read as an IETF designation. The exception is one token wide.
    if !designations("let m = \"RFC 5.E lines 1024-1073\";").is_empty() {
        wrong.push("  rfc: this repo's own RFC section read as a designation".to_string());
    }
    if designations("let m = \"an RFC 4122 canonical UUID\";").is_empty() {
        wrong.push("  rfc: a genuine IETF designation was missed".to_string());
    }

    assert!(
        wrong.is_empty(),
        "the position split this gate rests on is wrong:\n{}",
        wrong.join("\n"),
    );
}

/// The decision this gate makes, exercised where the tree cannot exercise
/// it — in the direction that refuses.
///
/// ⚠ The tree-wide sweep is green today and is meant to stay green, so it
/// can never show that the chain behind it still works: lex the file,
/// match the name, ask the registry whether this PATH admits it. A gate
/// whose only evidence is an empty result is measuring nothing, and this
/// suite has been caught by that shape more than once. Each row below runs
/// the same [`refusals_in`] the sweep runs, over a source written here.
///
/// The rows are paired on purpose. A registry that admitted everything
/// would pass every `admitted` row and fail every `refused` one; a
/// registry that admitted nothing would do the reverse. Only a registry
/// that is actually consulted, with the path, passes both.
#[test]
fn the_decision_is_exercised_where_it_fails() {
    struct Case {
        what: &'static str,
        path: &'static str,
        source: &'static str,
        lang: Lang,
        /// How many refusals this file should produce.
        refusals: usize,
    }

    let cases = &[
        Case {
            what: "mesh implements SOME/IP, so mesh may name it",
            path: "sce-build/src/mesh/transport/someip.rs",
            source: "let t = \"someip\";\n",
            lang: Lang::Rust,
            refusals: 0,
        },
        Case {
            what: "the closure path may not name it, implemented or not",
            path: "sce-build/src/requirements_report.rs",
            source: "let t = \"someip\";\n",
            lang: Lang::Rust,
            refusals: 1,
        },
        Case {
            what: "implementing SOME/IP does not license another standard",
            path: "sce-build/src/mesh/transport/someip.rs",
            source: "let d = \"ISO 26262-6\";\n",
            lang: Lang::Rust,
            refusals: 1,
        },
        Case {
            what: "a citation in the closure path is evidence, not a branch",
            path: "sce-build/src/requirement_manifest.rs",
            source: "// measured over ISO 13400-2:2019, 164 of 166\nlet n = 166;\n",
            lang: Lang::Rust,
            refusals: 0,
        },
        Case {
            what: "the mesh template emits the wire format and may name it",
            path: "tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2",
            source: "const char *t = \"someip\";\n",
            lang: Lang::Template(Target::CFamily),
            refusals: 0,
        },
        Case {
            what: "a core template does not inherit mesh's exemption",
            path: "tools/codegen/templates/state_machine.jinja2",
            source: "const char *t = \"someip\";\n",
            lang: Lang::Template(Target::CFamily),
            refusals: 1,
        },
        Case {
            what: "a designation outside every scope is refused",
            path: "sce-build/src/generator.rs",
            source: "let u = \"RFC 4122\";\n",
            lang: Lang::Rust,
            refusals: 1,
        },
        Case {
            what: "...and inside the scope that argues for it, admitted",
            path: "tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2",
            source: "const char *u = \"an RFC 4122 canonical UUID\";\n",
            lang: Lang::Template(Target::CFamily),
            refusals: 0,
        },
        Case {
            what: "a fixture inside a test module is not production",
            path: "sce-build/src/requirements_report.rs",
            source: "#[cfg(test)]\nmod t {\n  const D: &str = \"ISO 13400-2\";\n}\n",
            lang: Lang::Rust,
            refusals: 0,
        },
    ];

    let mut wrong = Vec::new();
    for case in cases {
        let got = refusals_in(case.path, case.source, case.lang);
        if got.len() != case.refusals {
            wrong.push(format!(
                "  {}\n    {} — expected {} refusal(s), got {}: {:?}",
                case.what,
                case.path,
                case.refusals,
                got.len(),
                got.iter().map(describe).collect::<Vec<_>>(),
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "the decision behind the sweep is wrong:\n{}",
        wrong.join("\n"),
    );
}

/// The generative half finds designations it was never told about.
///
/// The point of the shape is that a specification SCE has never seen is
/// caught the first time anyone writes its name, so none of these is in any
/// list in this file.
#[test]
fn a_designation_this_gate_has_never_seen_is_still_found() {
    const NEVER_SEEN: &[&str] = &[
        "IEC 61131-3",
        "SAE J1939",
        "IEEE 802.1Q",
        "ETSI TS 103 097",
        "ISO 26262-6",
        "ISO/IEC 14882",
        "AUTOSAR 4.4",
        "MISRA 2012",
        "ARINC 653",
        "iso8601",
        "parseIso13400",
    ];
    let missed: Vec<&str> = NEVER_SEEN
        .iter()
        .copied()
        .filter(|d| designations(&format!("let x = \"{d}\";")).is_empty())
        .collect();
    assert!(
        missed.is_empty(),
        "the generative half missed {missed:?} — these are not in any list \
         here, which is the point: a specification SCE has never seen must be \
         caught the first time its name reaches code. If the shape has to \
         narrow, say which designations it stops covering.",
    );

    // ...and does not fire on prose that merely looks like one.
    const NOT_DESIGNATIONS: &[&str] = &[
        "isolate",
        "isolation_level",
        "ISOLATION",
        "let omg = 1;",
        "din_rail",
    ];
    let false_positives: Vec<&str> = NOT_DESIGNATIONS
        .iter()
        .copied()
        .filter(|t| !designations(t).is_empty())
        .collect();
    assert!(
        false_positives.is_empty(),
        "the shape fired on {false_positives:?}, which name no standard — a \
         gate that cries wolf is one people learn to route around",
    );
}
