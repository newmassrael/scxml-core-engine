// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A backend added to `Language` must be decided at every site that decides
// per backend, and every run that spans "all the backends" must take them
// from `Language::ALL`.
//
// `Language::ALL`'s own doc says why: a seventh backend lands there once and
// every all-backends site widens with it, while a site that restates the six
// covers some of a new backend and silently skips the rest. The compiler
// enforces half of that on its own — an exhaustive `match` without a
// wildcard refuses to build until the new variant has an arm. The other half
// it cannot see, and two shapes of it were measured on 2026-10-20, before
// the Swift backend was started (the owner's requirement for it: the same
// structure as the other backends, no single-source-of-truth violation, and
// judged by structure rather than by results):
//
//   * a `match` that names `Language` variants in some arms and catches the
//     rest with `_` or a binding: the new variant falls into the catch-all,
//     compiles, and inherits whatever the catch-all says without anyone
//     deciding it. 79 such blocks were in `sce-build/src`.
//   * a list of backend names written out by hand: a new backend is not in
//     it, so the run it drives skips that backend and stays green. 41 files
//     carried one.
//
// Both are found by parsing the sources with `syn`, not by searching their
// text: a search cannot tell a pattern from the prose that describes it, and
// this repository has had a source check pass on a deleted call because the
// call's name survived in a comment above it.
//
// A list of fewer backends than every one but one is not refused here. It
// says which backends something applies to, and whether a new backend joins
// it is the decision the backend parity check makes against the registries
// that own those answers.
//
// Those registries are the one kind of written-out list that is right. A
// set a backend JOINS when it gains a capability — `STATIC_DATAMODEL_BACKENDS`
// says "a backend joins this list in the commit that teaches its templates
// the model" — must not be `Language::ALL`, because a new backend would then
// be admitted to a capability it does not have. Such a set is an item-level
// `const` or `static` in the generator, typed by `Language`, whose doc says
// how a backend joins it; that is the shape this check exempts, and the only
// one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Expr, ExprMatch, ImplItemFn, ItemFn, ItemImpl, Pat, Token};

/// Lower bound on the generator sources parsed: a walk that found nothing
/// would leave both checks passing on an empty set.
const MIN_SOURCE_FILES: usize = 150;

/// Lower bound on the test sources parsed, for the same reason.
const MIN_TEST_FILES: usize = 300;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries =
            fs::read_dir(&next).unwrap_or_else(|e| panic!("read {}: {e}", next.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn parse(path: &Path) -> syn::File {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    syn::parse_file(&text)
        .unwrap_or_else(|e| panic!("{} does not parse as Rust: {e}", path.display()))
}

fn relative(path: &Path) -> String {
    path.strip_prefix(crate_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// The backend a pattern or expression path names: `Language::Go`, or
/// `Self::Go` inside `impl Language`.
fn language_of_path(path: &syn::Path, inside_impl_language: bool) -> Option<Language> {
    let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    let [.., owner, variant] = segments.as_slice() else {
        return None;
    };
    if owner != "Language" && !(inside_impl_language && owner == "Self") {
        return None;
    }
    Language::ALL
        .iter()
        .copied()
        .find(|language| format!("{language:?}") == *variant)
}

/// Whether a pattern names a backend anywhere inside it.
fn names_language(pat: &Pat, inside_impl_language: bool) -> bool {
    match pat {
        Pat::Path(p) => language_of_path(&p.path, inside_impl_language).is_some(),
        Pat::Or(or) => or
            .cases
            .iter()
            .any(|case| names_language(case, inside_impl_language)),
        Pat::Paren(p) => names_language(&p.pat, inside_impl_language),
        Pat::Reference(r) => names_language(&r.pat, inside_impl_language),
        Pat::Ident(i) => i
            .subpat
            .as_ref()
            .is_some_and(|(_, sub)| names_language(sub, inside_impl_language)),
        _ => false,
    }
}

/// Whether a pattern takes every value: `_`, `..`, or a plain binding.
fn catches_all(pat: &Pat) -> bool {
    match pat {
        Pat::Wild(_) | Pat::Rest(_) => true,
        Pat::Ident(i) => {
            i.subpat.is_none() && i.by_ref.is_none() && {
                // A lower-case identifier with no sub-pattern binds; an upper-case
                // one is a constant or a unit variant brought into scope.
                i.ident
                    .to_string()
                    .starts_with(|c: char| c.is_lowercase() || c == '_')
            }
        }
        Pat::Paren(p) => catches_all(&p.pat),
        Pat::Reference(r) => catches_all(&r.pat),
        _ => false,
    }
}

/// An arm's pattern split into the columns a tuple scrutinee has, each
/// alternative of a top-level `|` taken on its own.
fn columns(pat: &Pat) -> Vec<Vec<&Pat>> {
    match pat {
        Pat::Or(or) => or.cases.iter().flat_map(columns).collect(),
        Pat::Tuple(t) => vec![t.elems.iter().collect()],
        Pat::Paren(p) => columns(&p.pat),
        other => vec![vec![other]],
    }
}

/// Whether a `match` decides by backend in some column and lets a backend it
/// does not name fall into a catch-all in that same column.
fn falls_through(m: &ExprMatch, inside_impl_language: bool) -> bool {
    let rows: Vec<Vec<&Pat>> = m.arms.iter().flat_map(|arm| columns(&arm.pat)).collect();
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    (0..width).any(|col| {
        let cells = || rows.iter().filter_map(move |row| row.get(col).copied());
        cells().any(|p| names_language(p, inside_impl_language)) && cells().any(catches_all)
    })
}

/// The function a site sits in, which is how a site is named: a `syn` span
/// carries no line number without a proc-macro2 feature the workspace does
/// not enable, and a function name survives the edits that move lines.
#[derive(Default)]
struct Context {
    functions: Vec<String>,
    impl_language: Vec<bool>,
}

impl Context {
    fn here(&self, file: &str) -> String {
        match self.functions.last() {
            Some(function) => format!("{file} :: {function}"),
            None => format!("{file} :: (item level)"),
        }
    }
    fn inside_impl_language(&self) -> bool {
        self.impl_language.last().copied().unwrap_or(false)
    }
}

/// The arguments of a macro call when they parse as expressions, so
/// `assert_eq!(x, [..])` and `vec![..]` are read like the code they hold.
fn macro_exprs(mac: &syn::Macro) -> Vec<Expr> {
    mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
        .map(|exprs| exprs.into_iter().collect())
        .unwrap_or_default()
}

struct Sites<'a> {
    file: &'a str,
    /// Whether this file is generator source, where a documented
    /// `Language`-typed registry is a decision rather than a restatement.
    registries_allowed: bool,
    context: Context,
    fall_throughs: Vec<String>,
    hand_lists: Vec<String>,
}

/// Whether an item-level constant is a capability registry: typed by
/// `Language` and documented, so that the doc can say how a backend joins.
fn is_registry(attrs: &[syn::Attribute], ty: &syn::Type) -> bool {
    let documented = attrs.iter().any(|a| a.path().is_ident("doc"));
    let typed_by_language = quote_type(ty).contains("Language");
    documented && typed_by_language
}

/// The identifiers a type is spelled with, for asking which type it names.
fn quote_type(ty: &syn::Type) -> String {
    struct Idents(String);
    impl<'ast> Visit<'ast> for Idents {
        fn visit_ident(&mut self, ident: &'ast syn::Ident) {
            self.0.push_str(&ident.to_string());
            self.0.push(' ');
        }
    }
    let mut idents = Idents(String::new());
    idents.visit_type(ty);
    idents.0
}

impl<'a> Sites<'a> {
    fn new(file: &'a str, registries_allowed: bool) -> Self {
        Self {
            file,
            registries_allowed,
            context: Context::default(),
            fall_throughs: Vec::new(),
            hand_lists: Vec::new(),
        }
    }

    /// Records a list that names every backend, or every backend but one.
    fn check_list<'e>(&mut self, elements: impl Iterator<Item = &'e Expr>) {
        let named: BTreeSet<String> = elements
            .filter_map(|e| language_of_element(e, self.context.inside_impl_language()))
            .map(|language| language.canonical_name().to_string())
            .collect();
        if named.len() + 1 >= Language::ALL.len() {
            let names: Vec<&str> = named.iter().map(String::as_str).collect();
            self.hand_lists.push(format!(
                "{} [{}]",
                self.context.here(self.file),
                names.join(", ")
            ));
        }
    }
}

/// The backend a list element names: a string the CLI accepts as one
/// (`"go"`, `"c"`), or a `Language::` path.
fn language_of_element(element: &Expr, inside_impl_language: bool) -> Option<Language> {
    match element {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => s.value().parse::<Language>().ok(),
        Expr::Path(p) => language_of_path(&p.path, inside_impl_language),
        Expr::Reference(r) => language_of_element(&r.expr, inside_impl_language),
        _ => None,
    }
}

impl<'ast> Visit<'ast> for Sites<'_> {
    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        let is_language = matches!(&*item.self_ty, syn::Type::Path(p)
            if p.path.segments.last().is_some_and(|s| s.ident == "Language"));
        self.context
            .impl_language
            .push(is_language && item.trait_.is_none());
        visit::visit_item_impl(self, item);
        self.context.impl_language.pop();
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.context.functions.push(item.sig.ident.to_string());
        visit::visit_item_fn(self, item);
        self.context.functions.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        self.context.functions.push(item.sig.ident.to_string());
        visit::visit_impl_item_fn(self, item);
        self.context.functions.pop();
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if self.registries_allowed && is_registry(&item.attrs, &item.ty) {
            return;
        }
        visit::visit_item_const(self, item);
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if self.registries_allowed && is_registry(&item.attrs, &item.ty) {
            return;
        }
        visit::visit_item_static(self, item);
    }

    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        // `Language::ALL` is the one place the backends are written out.
        if self.context.inside_impl_language() && item.ident == "ALL" {
            return;
        }
        visit::visit_impl_item_const(self, item);
    }

    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        if falls_through(m, self.context.inside_impl_language()) {
            self.fall_throughs.push(self.context.here(self.file));
        }
        visit::visit_expr_match(self, m);
    }

    fn visit_expr_array(&mut self, array: &'ast syn::ExprArray) {
        self.check_list(array.elems.iter());
        visit::visit_expr_array(self, array);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let exprs = macro_exprs(mac);
        // `vec![a, b, c]` is a list whose elements arrive as the macro's
        // arguments, not as an array expression.
        if mac.path.is_ident("vec") {
            self.check_list(exprs.iter());
        }
        for expr in &exprs {
            self.visit_expr(expr);
        }
        visit::visit_macro(self, mac);
    }
}

/// Scans each file; `registries_allowed` says whether the files are
/// generator source, where a documented `Language` registry is exempt.
fn scan(files: &[PathBuf], registries_allowed: bool) -> (Vec<String>, Vec<String>) {
    let mut fall_throughs = Vec::new();
    let mut hand_lists = Vec::new();
    for path in files {
        let name = relative(path);
        let mut sites = Sites::new(&name, registries_allowed);
        sites.visit_file(&parse(path));
        fall_throughs.extend(sites.fall_throughs);
        hand_lists.extend(sites.hand_lists);
    }
    (fall_throughs, hand_lists)
}

fn scan_text(text: &str) -> (Vec<String>, Vec<String>) {
    let file = syn::parse_file(text).expect("the sample parses");
    let mut sites = Sites::new("sample.rs", true);
    sites.visit_file(&file);
    (sites.fall_throughs, sites.hand_lists)
}

/// The scanners find what they exist to find, and leave alone what they do
/// not: without this the two checks below could pass by seeing nothing.
#[test]
fn the_scanners_tell_a_decision_from_a_fall_through() {
    let sample = r#"
        fn wildcard(l: Language) -> u8 { match l { Language::Go => 1, _ => 0 } }
        fn binding(l: Language) -> u8 { match l { Language::Go => 1, other => other as u8 } }
        fn tuple(l: Language, k: u8) -> u8 { match (l, k) { (Language::Go, 0) => 1, (_, 0) => 2, _ => 3 } }
        fn kind_wildcard_only(l: Language, k: u8) -> u8 { match (l, k) { (Language::Go, _) => 1, (Language::Rust | Language::Cpp | Language::Kotlin | Language::Python | Language::C11, _) => 2 } }
        fn exhaustive(l: Language) -> u8 { match l { Language::Rust | Language::Cpp => 0, Language::Kotlin | Language::Go | Language::Python | Language::C11 => 1 } }
        fn unrelated(x: Option<u8>) -> u8 { match x { Some(v) => v, _ => 0 } }
        impl Language { fn inner(self) -> u8 { match self { Self::Go => 1, _ => 0 } } }
        fn lists() {
            for l in ["rust", "cpp", "kotlin", "go", "python", "c"] {}
            assert_eq!(x, ["rust", "cpp", "kotlin", "go", "python"]);
            let typed = vec![Language::Rust, Language::Cpp, Language::Kotlin, Language::Go, Language::Python, Language::C11];
            for l in ["rust", "kotlin"] {}
            let words = ["a", "b", "c"];
        }
        /// A backend joins this set in the commit that teaches it the capability.
        const REGISTRY: &[Language] = &[Language::Rust, Language::Cpp, Language::Kotlin, Language::Go, Language::Python, Language::C11];
        const UNDOCUMENTED: &[Language] = &[Language::Rust, Language::Cpp, Language::Kotlin, Language::Go, Language::Python, Language::C11];
        /// Documented, but a list of names rather than of backends.
        const NAMES: &[&str] = &["rust", "cpp", "kotlin", "go", "python", "c11"];
    "#;
    let (fall_throughs, hand_lists) = scan_text(sample);
    assert_eq!(
        fall_throughs,
        [
            "sample.rs :: wildcard",
            "sample.rs :: binding",
            "sample.rs :: tuple",
            "sample.rs :: inner",
        ],
        "the fall-through scanner must flag the catch-all in the backend column, by `_`, by a \
         binding, in a tuple and through `Self::` inside `impl Language`, and nothing else"
    );
    assert_eq!(
        hand_lists,
        [
            "sample.rs :: lists [c11, cpp, go, kotlin, python, rust]",
            "sample.rs :: lists [cpp, go, kotlin, python, rust]",
            "sample.rs :: lists [c11, cpp, go, kotlin, python, rust]",
            "sample.rs :: (item level) [c11, cpp, go, kotlin, python, rust]",
            "sample.rs :: (item level) [c11, cpp, go, kotlin, python, rust]",
        ],
        "the list scanner must flag the six-name list, the five-name list inside a macro, the \
         typed `vec!`, the undocumented registry and the documented list of names; and leave the \
         two-name list, the unrelated words and the documented `Language` registry alone"
    );
}

/// Every `match` in the generator that decides by backend decides for every
/// backend: no `_` or binding catches one it does not name.
#[test]
fn no_backend_falls_through_a_catch_all_in_the_generator() {
    let files = rust_files(&crate_root().join("src"));
    assert!(
        files.len() >= MIN_SOURCE_FILES,
        "found only {} source files under sce-build/src, so this check is not reading the generator",
        files.len()
    );
    let (fall_throughs, _) = scan(&files, true);
    assert!(
        fall_throughs.is_empty(),
        "{} `match` block(s) decide by `Language` in some arms and catch the remaining backends \
         with `_` or a binding. A backend added to `Language` would fall into the catch-all, \
         compile, and inherit its arm without anyone deciding it. Name every backend the arm \
         stands for, so that adding one is a compile error at this site:\n  {}",
        fall_throughs.len(),
        fall_throughs.join("\n  ")
    );
}

/// No run that spans every backend writes the backends out: it takes them
/// from `Language::ALL`, or from it less a named exception.
#[test]
fn no_list_of_every_backend_is_written_by_hand() {
    let tests = rust_files(&crate_root().join("tests"));
    assert!(
        tests.len() >= MIN_TEST_FILES,
        "found only {} test files under sce-build/tests, so this check is not reading them",
        tests.len()
    );
    let (_, mut hand_lists) = scan(&tests, false);
    hand_lists.extend(scan(&rust_files(&crate_root().join("src")), true).1);
    assert!(
        hand_lists.is_empty(),
        "{} list(s) name every backend, or every backend but one, by hand. A backend added to \
         `Language` is not in such a list, so whatever the list drives skips it and stays green. \
         Take the backends from `Language::ALL`, and write an exception as a filter with the \
         reason beside it, so that a new backend is included until someone excludes it:\n  {}",
        hand_lists.len(),
        hand_lists.join("\n  ")
    );
}
