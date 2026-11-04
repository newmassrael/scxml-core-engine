// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A text the generator writes where kotlinc keeps it in a class-file
//! attribute goes through `filters::kotlin_string_val`, or is a fixed
//! vocabulary named here with the reason it stays short.
//!
//! A string constant of a JVM class file holds at most 65,535 bytes of
//! modified UTF-8 (JVM Specification, section 4.4.7). kotlinc cuts a longer
//! string itself wherever the string is an expression, and does not where it
//! writes the string into an attribute: the `ConstantValue` attribute of a
//! `const val`'s field, and an annotation's argument. The evidence, class and
//! method names of the compiler the build pins, is written at
//! `kotlin_string_val` in `sce-build/src/filters.rs`, which is the one place
//! a text no generator bounds may enter either.
//!
//! So this census finds every such place the Kotlin backend writes a value
//! into:
//!
//! - in the Kotlin templates, an interpolation inside the string literal that
//!   initializes a `const val`, or inside the argument list of an annotation;
//! - in the generator's Rust source, a format string that writes a
//!   `const val` whose type is `String` or is itself filled in, with a value
//!   filled in, or an annotation with a value filled in inside its arguments.
//!
//! Each is refused unless [`FIXED_VOCABULARY`] names it, and an entry there
//! that names nothing the tree still writes is refused too, so the list
//! describes the tree as it is.
//!
//! # What this does not hold
//!
//! - A Rust format string spread over several source lines is read one line
//!   at a time; every one the generator writes today is on one line.
//! - Rust test modules (`#[cfg(test)] mod … { … }`) are skipped: what they
//!   write is a test's expectation, not generated Kotlin.
//! - A literal in a function body or a property initializer, which kotlinc
//!   cuts itself; `backends/kotlin/tests/fixtures/jvm_long_text/` is the
//!   document the Kotlin lane compiles and runs to hold that.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use common::rust_source::{code_mask, code_only};
use common::source_lexing::{language_of, Lang};
use regex::Regex;
use sce_build::template_lexing::{interpolations, without_template_prose, Class, Syntax};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Where kotlinc keeps a value the generator writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Attribute {
    /// The initializer of a `const val`: its field's `ConstantValue`.
    ConstVal,
    /// An argument of an annotation.
    Annotation,
}

/// One place a value lands in a class-file attribute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    /// Repository-relative path of the template or Rust file.
    file: String,
    /// The constant's name or the annotation's name, as the file spells it.
    name: String,
    attribute: Attribute,
    /// 1-based line, for the failure message.
    line: usize,
}

/// A constant that takes a value which is not free text, and why it stays
/// within one class-file constant whatever the document says.
struct Fixed {
    file: &'static str,
    name: &'static str,
    attribute: Attribute,
    reason: &'static str,
}

const FIXED_VOCABULARY: &[Fixed] = &[
    Fixed {
        file: "tools/codegen/templates/kotlin/state_machine.kt.jinja2",
        name: "SOURCE_HASH",
        attribute: Attribute::ConstVal,
        reason: "the SHA-256 digest of the machine's sources in hexadecimal: \
                 64 ASCII bytes whatever the document holds",
    },
    Fixed {
        file: "tools/codegen/templates/kotlin/transition_function.kt.jinja2",
        name: "SOURCE_HASH",
        attribute: Attribute::ConstVal,
        reason: "the SHA-256 digest of the document in hexadecimal: 64 ASCII \
                 bytes whatever the document holds",
    },
    Fixed {
        file: "tools/codegen/templates/forge/kotlin/queue.kt.jinja2",
        name: "ALLOCATOR_PROGRESS_WORD",
        attribute: Attribute::ConstVal,
        reason: "a word of the progress vocabulary (`wait-free`, `lock-free`, \
                 `blocking`), read into an enum before it is written back",
    },
    Fixed {
        file: "tools/codegen/templates/forge/kotlin/queue.kt.jinja2",
        name: "DECLARED_PROGRESS",
        attribute: Attribute::ConstVal,
        reason: "a word of the progress vocabulary, written from the enum the \
                 document's `<sce:progress>` was read into (`queue_progress_word`)",
    },
    Fixed {
        file: "tools/codegen/templates/forge/kotlin/queue.kt.jinja2",
        name: "PUSH_PROGRESS",
        attribute: Attribute::ConstVal,
        reason: "a word of the progress vocabulary the selected algorithm gives \
                 push: a `&'static str` of the selection table",
    },
    Fixed {
        file: "tools/codegen/templates/forge/kotlin/queue.kt.jinja2",
        name: "POP_PROGRESS",
        attribute: Attribute::ConstVal,
        reason: "a word of the progress vocabulary the selected algorithm gives \
                 pop: a `&'static str` of the selection table",
    },
    Fixed {
        file: "tools/codegen/templates/forge/kotlin/queue.kt.jinja2",
        name: "ALGORITHM",
        attribute: Attribute::ConstVal,
        reason: "the name of the algorithm the selection table gave the \
                 contract: a `&'static str` of that table",
    },
    Fixed {
        file: "tools/codegen/templates/forge/kotlin/broadcast_log.kt.jinja2",
        name: "DECLARED_PROGRESS",
        attribute: Attribute::ConstVal,
        reason: "a word of the progress vocabulary, written from the enum the \
                 document's `<sce:progress>` was read into",
    },
    Fixed {
        file: "sce-build/src/forge/generator.rs",
        name: "{name}",
        attribute: Attribute::ConstVal,
        reason: "a scalar `<sce:const>`: `const_fold::ConstValue` holds integers, \
                 reals and bools only, so the constant is never a `String`",
    },
    Fixed {
        file: "sce-build/src/bin/sce_codegen.rs",
        name: "DisplayName",
        attribute: Attribute::Annotation,
        reason: "the Kotlin W3C test class's name for JUnit: a W3C test number \
                 and the section number the W3C manifest gives it",
    },
];

/// A site the Rust scan finds that writes no Kotlin, and which language it
/// writes instead: the scan reads every format string of the generator, and
/// an annotation's spelling is shared with languages no JVM compiles.
const NOT_KOTLIN: &[Fixed] = &[Fixed {
    file: "sce-build/src/bin/sce_codegen.rs",
    name: "Test",
    attribute: Attribute::Annotation,
    reason: "a Swift Testing attribute, written by the Swift W3C test writer; \
             the Swift compiler keeps no class-file constant",
}];

/// The door itself: `kotlin_string_val` writes a `const val` only once
/// `jvm_constant_chunks` found the text to be one piece.
const THE_DOOR: (&str, &str) = ("sce-build/src/filters.rs", "{name}");

/// Every file under `dir`, recursively, sorted.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in
            std::fs::read_dir(&next).unwrap_or_else(|e| panic!("read {}: {e}", next.display()))
        {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("a path under the repository")
        .to_string_lossy()
        .replace('\\', "/")
}

/// Whether a template emits Kotlin: by the directory the shared derivation
/// reads, by a `.kt` / `.kts` name, or by a `kotlin_` macro's name.
fn is_kotlin_template(rel: &str) -> bool {
    if !rel.ends_with(".jinja2") {
        return false;
    }
    let file = rel.rsplit('/').next().unwrap_or(rel);
    language_of(rel) == Some(Lang::Template(Syntax::Kotlin))
        || file.contains(".kt.")
        || file.contains(".kts.")
        || file.starts_with("kotlin_")
}

/// What the census read, so a scan that read nothing is a red.
#[derive(Default)]
struct Reading {
    sites: BTreeSet<Site>,
    templates: usize,
    literal_interpolations: usize,
    door_calls: usize,
    rust_files: usize,
}

fn const_val_head() -> Regex {
    Regex::new(r"\bconst\s+val\s+([A-Za-z_][A-Za-z0-9_]*)[^=]*=\s*").expect("static regex")
}

fn open_annotation() -> Regex {
    Regex::new(r"@(?:[a-z]+:)?([A-Za-z_][A-Za-z0-9_.]*)\s*\([^()]*$").expect("static regex")
}

/// The Kotlin templates' sites: an interpolation in a literal that
/// initializes a `const val`, or in an annotation's open argument list.
fn read_templates(root: &Path, reading: &mut Reading) {
    let head = const_val_head();
    let annotation = open_annotation();
    let dangling_head =
        Regex::new(r"\bconst\s+val\s+([A-Za-z_][A-Za-z0-9_]*)[^=]*=\s*$").expect("static regex");
    for path in files_under(&root.join("tools/codegen/templates")) {
        let rel = relative(root, &path);
        if !is_kotlin_template(&rel) {
            continue;
        }
        let template = std::fs::read_to_string(&path).expect("a template reads as UTF-8");
        reading.templates += 1;
        reading.door_calls += template.matches("kotlin_string_val(").count();
        let chars: Vec<char> = without_template_prose(&template).chars().collect();
        for site in interpolations(&template, Syntax::Kotlin) {
            if !matches!(site.context, Class::Literal | Class::RawLiteral) {
                continue;
            }
            reading.literal_interpolations += 1;
            let line_start = chars[..site.start]
                .iter()
                .rposition(|&c| c == '\n')
                .map_or(0, |at| at + 1);
            let before: String = chars[line_start..site.start].iter().collect();
            let mut found = |name: &str, attribute: Attribute| {
                reading.sites.insert(Site {
                    file: rel.clone(),
                    name: name.to_string(),
                    attribute,
                    line: site.line,
                });
            };
            if let Some(m) = head.captures(&before) {
                found(&m[1], Attribute::ConstVal);
            } else if before.trim_start().starts_with('"') && line_start > 0 {
                // `const val NAME: String =` with its literal on the next line.
                let previous_end = line_start - 1;
                let previous_start = chars[..previous_end]
                    .iter()
                    .rposition(|&c| c == '\n')
                    .map_or(0, |at| at + 1);
                let previous: String = chars[previous_start..previous_end].iter().collect();
                if let Some(m) = dangling_head.captures(&previous) {
                    found(&m[1], Attribute::ConstVal);
                }
            }
            if let Some(m) = annotation.captures(&before) {
                found(&m[1], Attribute::Annotation);
            }
        }
    }
}

/// Whether a Rust format string fills a value in: a `{` that is not the `{{`
/// of a literal brace.
fn fills_in(text: &str) -> bool {
    text.replace("{{", "").replace("}}", "").contains('{')
}

/// The lines of a Rust file that are not inside a `#[cfg(test)] mod`.
fn production_lines(src: &str) -> Vec<(usize, String)> {
    let only: Vec<String> = code_only(src).lines().map(str::to_string).collect();
    let mask: Vec<String> = code_mask(src).lines().map(str::to_string).collect();
    let mut kept = Vec::new();
    let mut i = 0;
    while i < only.len() {
        let opens_test_module = mask[i].trim() == "#[cfg(test)]"
            && mask[i + 1..]
                .iter()
                .find(|l| !l.trim().is_empty())
                .is_some_and(|l| {
                    let l = l.trim_start();
                    (l.starts_with("mod ") || l.starts_with("pub mod ")) && l.contains('{')
                });
        if opens_test_module {
            let mut depth = 0i64;
            let mut opened = false;
            i += 1;
            while i < mask.len() {
                for c in mask[i].chars() {
                    match c {
                        '{' => {
                            depth += 1;
                            opened = true;
                        }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                i += 1;
                if opened && depth == 0 {
                    break;
                }
            }
            continue;
        }
        kept.push((i + 1, only[i].clone()));
        i += 1;
    }
    kept
}

/// The generator's Rust sites: a format string that writes a text-valued
/// `const val` with a value filled in, or an annotation whose arguments
/// take one.
fn read_rust(root: &Path, reading: &mut Reading) {
    let const_val =
        Regex::new(r"const\s+val\s+(\S+?)\s*(?::\s*([^=]+?))?\s*=\s*(.*)").expect("static regex");
    let annotation =
        Regex::new(r"@(?:[a-z]+:)?([A-Za-z_][A-Za-z0-9_.]*)\(([^()]*)\)").expect("static regex");
    for path in files_under(&root.join("sce-build/src")) {
        let rel = relative(root, &path);
        if !rel.ends_with(".rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("a Rust file reads as UTF-8");
        reading.rust_files += 1;
        for (line, text) in production_lines(&src) {
            let text = text.replace("\\\"", "\"");
            if let Some(m) = const_val.captures(&text) {
                let declared = m.get(2).map(|t| t.as_str().trim());
                let holds_text = match declared {
                    None => true,
                    Some(ty) => ty == "String" || fills_in(ty),
                };
                if holds_text && fills_in(&m[3]) {
                    reading.sites.insert(Site {
                        file: rel.clone(),
                        name: m[1].to_string(),
                        attribute: Attribute::ConstVal,
                        line,
                    });
                }
            }
            for m in annotation.captures_iter(&text) {
                if fills_in(&m[2]) {
                    reading.sites.insert(Site {
                        file: rel.clone(),
                        name: m[1].to_string(),
                        attribute: Attribute::Annotation,
                        line,
                    });
                }
            }
        }
    }
}

fn reading() -> Reading {
    let root = repo_root();
    let mut reading = Reading::default();
    read_templates(&root, &mut reading);
    read_rust(&root, &mut reading);
    reading
}

impl Fixed {
    fn names(&self, site: &Site) -> bool {
        self.file == site.file && self.name == site.name && self.attribute == site.attribute
    }
}

fn is_fixed(site: &Site) -> bool {
    FIXED_VOCABULARY
        .iter()
        .chain(NOT_KOTLIN)
        .any(|f| f.names(site))
}

fn is_the_door(site: &Site) -> bool {
    site.attribute == Attribute::ConstVal && (site.file.as_str(), site.name.as_str()) == THE_DOOR
}

#[test]
fn every_text_a_class_file_attribute_takes_goes_through_the_door() {
    let reading = reading();
    // Floors, because the failure that matters most is a scan that reads
    // nothing: no site found reports a clean tree as loudly as every site
    // going through the door. 55 Kotlin templates, 5 calls of the door, about
    // 400 literal interpolations and 248 Rust files were read by a rough count
    // when this was set.
    assert!(
        reading.templates >= 40,
        "read only {} Kotlin template(s); the template walk or the Kotlin \
         derivation is broken, not the tree",
        reading.templates
    );
    assert!(
        reading.door_calls >= 5,
        "found only {} call(s) of kotlin_string_val in the Kotlin templates; \
         the event-schema and timer units write five",
        reading.door_calls
    );
    assert!(
        reading.literal_interpolations >= 150,
        "found only {} interpolation(s) inside a Kotlin string literal; the \
         template lexer stopped seeing literals",
        reading.literal_interpolations
    );
    assert!(
        reading.rust_files >= 100,
        "read only {} Rust file(s) under sce-build/src",
        reading.rust_files
    );
    let unbounded: Vec<String> = reading
        .sites
        .iter()
        .filter(|site| !is_fixed(site) && !is_the_door(site))
        .map(|site| {
            format!(
                "{}:{}: {:?} `{}` takes a value the generator does not bound",
                site.file, site.line, site.attribute, site.name
            )
        })
        .collect();
    assert!(
        unbounded.is_empty(),
        "a value reaches a class-file attribute kotlinc does not cut \
         (65,535 bytes of modified UTF-8 at most):\n  {}\n\n\
         Write a text constant as `{{{{ kotlin_string_val(\"NAME\", text) }}}}` \
         (sce-build/src/filters.rs), which writes a `val` built at class init \
         past the limit. Name it in FIXED_VOCABULARY only when its value is a \
         closed vocabulary, with the reason.",
        unbounded.join("\n  ")
    );
}

#[test]
fn every_fixed_vocabulary_entry_names_a_site_the_tree_writes() {
    let reading = reading();
    let stale: Vec<String> = FIXED_VOCABULARY
        .iter()
        .chain(NOT_KOTLIN)
        .filter(|f| !reading.sites.iter().any(|s| f.names(s)))
        .map(|f| format!("{} `{}` ({})", f.file, f.name, f.reason))
        .collect();
    assert!(
        stale.is_empty(),
        "FIXED_VOCABULARY or NOT_KOTLIN names a site the tree no longer \
         writes; remove it:\n  {}",
        stale.join("\n  ")
    );
    assert!(
        reading.sites.iter().any(is_the_door),
        "the census no longer finds kotlin_string_val's own `const val` in \
         {}: the Rust scan is broken, or the door moved and THE_DOOR with it",
        THE_DOOR.0
    );
    for f in FIXED_VOCABULARY.iter().chain(NOT_KOTLIN) {
        assert!(
            f.reason.len() > 40,
            "{} `{}`: an entry's reason says why the value stays short",
            f.file,
            f.name
        );
    }
}

/// The two readers find what they are written to find, on text built here.
#[test]
fn the_readers_find_a_constant_and_an_annotation_and_nothing_else() {
    let head = const_val_head();
    let annotation = open_annotation();
    assert_eq!(
        &head.captures("const val NAME: String = \"").unwrap()[1],
        "NAME"
    );
    assert_eq!(&head.captures("    const val X = \"").unwrap()[1], "X");
    assert!(head.captures("val NAME: String = \"").is_none());
    assert_eq!(&annotation.captures("@JvmName(\"").unwrap()[1], "JvmName");
    assert_eq!(
        &annotation.captures("@file:JvmName(\"").unwrap()[1],
        "JvmName"
    );
    assert!(annotation.captures("@Suppress(\"X\") val a = \"").is_none());
    assert!(fills_in("const val {name}: String = \"{}\""));
    assert!(!fills_in("fun f() {{ return 1 }}"));
    let lines = production_lines(
        "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn b() { let _ = \"}\"; }\n}\nfn c() {}\n",
    );
    let kept: Vec<&str> = lines.iter().map(|(_, l)| l.trim()).collect();
    assert_eq!(kept, ["fn a() {}", "fn c() {}"]);
}
