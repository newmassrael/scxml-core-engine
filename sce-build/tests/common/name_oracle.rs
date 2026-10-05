// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What every backend's name oracle asks the same way.
//!
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated code does. The kinds whose generated code
//! is a FUNCTION the author's names are locals of (algorithm, condition, filter,
//! interpolation, lookup, observer, transform, validator) meet the language's
//! own names in one scope: a parameter called `len` beside `len(data)`, an input
//! called like the package a call goes through.
//!
//! Each backend's oracle renames every name each document declares to every
//! candidate, generates the result, and builds it. What differs is the build
//! (a compiler, a runner) and the language's own scope; what is the same lives
//! here, so that a backend's oracle is its build and its universe:
//!
//! - the documents ([`documents`]) and what each declares,
//! - the candidates ([`candidates`]): the language's own names, joined with
//!   every identifier the committed output of that kind uses, as read from the
//!   code with comments and literals taken out ([`names_by_kind`]), and a few
//!   shapes a name can take,
//! - the rewrite ([`rename`]), word by word and never a member access,
//! - and the answer grouped by cause ([`group`]), so that one fault shared by
//!   hundreds of renamings reads as one line.
//!
//! The candidates are derived, not listed: a list written by hand is stale the
//! day a template starts to use one more name.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use regex::Regex;

use super::source_lexing::{structure_mask, Lang};

/// The kinds whose generated code is a function an author's names are locals of.
pub const KINDS: &[&str] = &[
    "algorithm",
    "condition",
    "filter",
    "interpolation",
    "lookup",
    "observer",
    "transform",
    "validator",
];

/// The shapes a name can take, so a spelling that is wrong for a shape is found
/// for every author who writes in it.
pub const SHAPES: &[&str] = &["engineRpm", "Foo", "MY_CONST", "x1", "rawValue"];

/// One document of the corpus, and what it declares.
pub struct Doc {
    pub stem: String,
    pub kind: String,
    pub text: String,
    /// Every `<data id>`, `<sce:param name>` and `<sce:var name>`.
    pub declared: Vec<String>,
}

/// Every document of the eight kinds under `resource_dir`, in file order.
pub fn documents(resource_dir: &Path) -> Vec<Doc> {
    let kind_re = Regex::new(r#"sce:kind="([a-z_-]+)""#).expect("regex");
    let declared_re = Regex::new(
        r#"<(?:data|sce:param|sce:var)\b[^>]*?\b(?:id|name)="([A-Za-z_][A-Za-z0-9_]*)""#,
    )
    .expect("regex");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(resource_dir)
        .expect("read resources")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "scxml") && p.is_file())
        .collect();
    entries.sort();
    let mut docs = Vec::new();
    for path in entries {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let Some(kind) = kind_re.captures(&text).map(|c| c[1].to_string()) else {
            continue;
        };
        if !KINDS.contains(&kind.as_str()) {
            continue;
        }
        let declared: BTreeSet<String> = declared_re
            .captures_iter(&text)
            .map(|c| c[1].to_string())
            .collect();
        docs.push(Doc {
            stem: path.file_stem().unwrap().to_string_lossy().to_string(),
            kind,
            text,
            declared: declared.into_iter().collect(),
        });
    }
    docs
}

/// How a name is spelled once folded to snake_case, for the check that two of
/// an author's own names do not meet in one spelling.
pub fn snake(name: &str) -> String {
    let mut out = String::new();
    let mut previous_lower_or_digit = false;
    for c in name.chars() {
        if c.is_ascii_uppercase() && previous_lower_or_digit {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
        previous_lower_or_digit = c.is_ascii_lowercase() || c.is_ascii_digit();
    }
    out
}

/// A document is rewritten by replacing every whole-word occurrence of `from`
/// that is not a member access (`x.from`).
pub fn rename(text: &str, from: &str, to: &str) -> String {
    let pattern = Regex::new(&format!(r"(^|[^\w.])({})\b", regex::escape(from)))
        .expect("an identifier is a regex");
    pattern
        .replace_all(text, |caps: &regex::Captures| format!("{}{}", &caps[1], to))
        .into_owned()
}

/// `text` with its root named `unique`: each renaming is its own package or
/// module or translation unit, named by position and never by the candidate.
pub fn with_name(text: &str, unique: &str) -> String {
    let name_re = Regex::new(r#"(<scxml\b[^>]*?\bname=")([^"]*)(")"#).expect("regex");
    name_re
        .replace(text, |caps: &regex::Captures| {
            format!("{}{}{}", &caps[1], unique, &caps[3])
        })
        .into_owned()
}

/// The identifiers of `source` that are code: comments and literals are taken
/// out first, so a word in a comment or a string is not a name the code uses.
pub fn identifiers(source: &str, lang: Lang) -> BTreeSet<String> {
    let word = Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").expect("regex");
    word.find_iter(&structure_mask(source, lang))
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Every identifier the committed output of a kind uses, by kind: the kind a
/// generated file says it came from in its header. `extensions` are the files
/// of the backend (`["go"]`, `["h"]`), and `excluded` the suffixes of another
/// backend that end the same way (the C backend's `.c.h` is also a `.h`).
pub fn names_by_kind(
    expected_dir: &Path,
    extensions: &[&str],
    excluded: &[&str],
    lang: Lang,
) -> BTreeMap<String, BTreeSet<String>> {
    let kind_re = Regex::new(r#"sce:kind="([a-z_-]+)""#).expect("regex");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(expected_dir)
        .expect("read the committed outputs")
        .map(|e| e.expect("directory entry").path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                extensions.iter().any(|e| n.ends_with(&format!(".{e}")))
                    && !excluded.iter().any(|x| n.ends_with(x))
            })
        })
        .collect();
    entries.sort();
    let mut by_kind: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in entries {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let head: String = source.chars().take(600).collect();
        let Some(kind) = kind_re.captures(&head).map(|c| c[1].to_string()) else {
            continue;
        };
        if KINDS.contains(&kind.as_str()) {
            by_kind
                .entry(kind)
                .or_default()
                .extend(identifiers(&source, lang));
        }
    }
    by_kind
}

/// The names a document's declarations are renamed to: what the committed
/// output of its kind uses, the language's own `universe`, and the shapes.
pub fn candidates(
    doc: &Doc,
    from_outputs: &BTreeMap<String, BTreeSet<String>>,
    universe: &[&str],
) -> BTreeSet<String> {
    from_outputs
        .get(&doc.kind)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .chain(universe.iter().map(|s| s.to_string()))
        .chain(SHAPES.iter().map(|s| s.to_string()))
        .filter(|n| n != "_")
        .collect()
}

/// Whether the generator's refusal of a renaming is an answer about the name,
/// given in the document before any code exists. Every refusal in this corpus
/// is one; a refusal that is not names its cause where the totals are printed,
/// so that a bad rewrite cannot hide as "refused".
pub fn is_name_refusal(why: &str) -> bool {
    why.contains("cannot declare it")
        || why.contains("shadows")
        || why.contains("duplicate")
        || why.contains("read-only")
        || why.contains("not declared")
        || why.contains("unknown")
        || why.contains("so it would declare one name twice")
}

/// One accepted renaming that did not build, and why.
pub struct Failure {
    pub kind: String,
    pub stem: String,
    pub declared: String,
    pub candidate: String,
    pub why: String,
}

/// The failures grouped by what they say once the position and the candidate
/// are taken out of the message: how many renamings, which kinds, which
/// candidate names, and one example in full.
pub fn group(failures: &[Failure]) -> Vec<String> {
    let position = Regex::new(r"^\S+?:\d+(?::\d+)?:\s*(?:(?:fatal )?error:\s*)?").expect("regex");
    struct Template {
        renamings: usize,
        candidates: BTreeSet<String>,
        kinds: BTreeSet<String>,
        example: String,
    }
    let mut by_template: BTreeMap<String, Template> = BTreeMap::new();
    for f in failures {
        let bare = position.replace(&f.why, "").to_string();
        let template = bare
            .replace(&format!("{}_", f.candidate), "<NAME>_")
            .replace(&f.candidate, "<NAME>");
        let entry = by_template.entry(template).or_insert_with(|| Template {
            renamings: 0,
            candidates: BTreeSet::new(),
            kinds: BTreeSet::new(),
            example: format!(
                "{} `{}` renamed to {}: {}",
                f.stem, f.declared, f.candidate, f.why
            ),
        });
        entry.renamings += 1;
        entry.candidates.insert(f.candidate.clone());
        entry.kinds.insert(f.kind.clone());
    }
    let mut templates: Vec<(&String, &Template)> = by_template.iter().collect();
    templates.sort_by_key(|(_, t)| std::cmp::Reverse(t.renamings));
    templates
        .iter()
        .map(|(template, t)| {
            let shown: Vec<&str> = t.candidates.iter().take(24).map(String::as_str).collect();
            format!(
                "  {:>5} renamings, {} names, kinds [{}]: {template}\n         names: {}{}\n         e.g. {}",
                t.renamings,
                t.candidates.len(),
                t.kinds.iter().cloned().collect::<Vec<_>>().join(", "),
                shown.join(", "),
                if t.candidates.len() > shown.len() { ", ..." } else { "" },
                t.example
            )
        })
        .collect()
}

/// Run `build` for every item on as many threads as the machine has, and
/// collect the ones that answered with a reason. A build is a compiler process,
/// and tens of thousands of them are a wait of minutes on one thread.
pub fn run_parallel<F>(items: &[String], build: F) -> BTreeMap<String, String>
where
    F: Fn(&str) -> Option<String> + Sync,
{
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    run_parallel_on(items, threads, build)
}

/// [`run_parallel`] on at most `threads` threads, for a build that is heavy
/// enough that the machine's cores are not the limit (a JVM compiler holds
/// gigabytes).
pub fn run_parallel_on<F>(items: &[String], threads: usize, build: F) -> BTreeMap<String, String>
where
    F: Fn(&str) -> Option<String> + Sync,
{
    let next = AtomicUsize::new(0);
    let failed = std::sync::Mutex::new(BTreeMap::new());
    let threads = threads.max(1);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                if let Some(why) = build(item) {
                    failed
                        .lock()
                        .expect("no build panics while holding the lock")
                        .insert(item.clone(), why);
                }
            });
        }
    });
    failed.into_inner().expect("no build panics")
}
