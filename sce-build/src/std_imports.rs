// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which standard-library units a piece of generated source reaches for.
//!
//! A Go file that names a package without importing it does not compile, and a
//! file that imports one it never names does not compile either. A C++ file that
//! names `std::string` without `<string>` compiles on the machine whose other
//! headers happen to pull it in and nowhere else. So the import block cannot be
//! written from the document's kind alone: whether `math`, `strconv` or
//! `<string>` is needed is a fact about the TEXT the expression emitters
//! produced, which is where `round(x)` (`math.Round`) and a string joined to an
//! integer (`strconv.FormatInt`, `std::to_string`) put it.
//!
//! Each template used to decide that for itself with `'math.' in body`, three
//! templates, three copies, and a test that fired on the text inside a string
//! literal as readily as on a call. Here it is decided once, from the program
//! text and not from characters that happen to spell it: a quoted string, a raw
//! string, a character literal and a comment are skipped, and a name counts only
//! where the language qualifies it (`pkg.` in Go, `std::` in C++) and it is not
//! itself the member of something else.
//!
//! The sets are tables, not patterns. A unit joins one by being added to
//! [`GO_PACKAGES`] or [`CPP_HEADERS`]; every template that asks reads the same
//! answer, through the one door that renders a template
//! (`forge::generator::LangCtx::render`).

/// The Go standard packages generated expression code reaches for, in the order
/// a Go import block lists them.
const GO_PACKAGES: &[&str] = &["math", "strconv"];

/// The C++ standard names generated expression code reaches for that are not
/// already decided from the model, each with the header that declares it, in
/// the order an include block lists them. `std::vector`, `std::span` and
/// `std::array` stay with the templates that know their element types; this
/// table is for what only the text can say.
const CPP_NAMES: &[(&str, &str)] = &[("string", "string"), ("to_string", "string")];

/// The headers of [`CPP_NAMES`], without repeats, in listing order.
pub const CPP_HEADERS: &[&str] = &["string"];

/// The Go packages in [`GO_PACKAGES`] that `source` uses, in that order.
pub fn go_used_in(source: &str) -> Vec<&'static str> {
    let mut found = [false; GO_PACKAGES.len()];
    scan(source, |names| {
        if let Name::Qualified {
            head,
            after_dot: false,
        } = names
        {
            if let Some(index) = GO_PACKAGES.iter().position(|p| *p == head) {
                found[index] = true;
            }
        }
    });
    GO_PACKAGES
        .iter()
        .zip(found)
        .filter_map(|(package, used)| used.then_some(*package))
        .collect()
}

/// The Go packages in [`GO_PACKAGES`] that `source` uses and does not import:
/// the quoted path of an import is what says a file already has one.
pub fn go_unimported_in(source: &str) -> Vec<&'static str> {
    go_used_in(source)
        .into_iter()
        .filter(|package| !source.contains(&format!("\"{package}\"")))
        .collect()
}

/// The C++ headers in [`CPP_HEADERS`] that `source` needs, in that order.
pub fn cpp_used_in(source: &str) -> Vec<&'static str> {
    let mut found = [false; CPP_HEADERS.len()];
    scan(source, |names| {
        if let Name::Std { member } = names {
            if let Some((_, header)) = CPP_NAMES.iter().find(|(name, _)| *name == member) {
                if let Some(index) = CPP_HEADERS.iter().position(|h| h == header) {
                    found[index] = true;
                }
            }
        }
    });
    CPP_HEADERS
        .iter()
        .zip(found)
        .filter_map(|(header, used)| used.then_some(*header))
        .collect()
}

/// What the scan reports: a name followed by `.` (a candidate Go package), and
/// the member of a `std::` qualification (a C++ name).
enum Name<'a> {
    /// An identifier immediately followed by `.`. `after_dot` says whether a
    /// `.` came just before it, which makes it a member and not a package.
    Qualified { head: &'a str, after_dot: bool },
    /// The identifier after `std::`.
    Std { member: &'a str },
}

/// Walk `source` once, calling `report` for each name of interest, with string
/// and character literals, raw strings and comments skipped.
fn scan<'a>(source: &'a str, mut report: impl FnMut(Name<'a>)) {
    let bytes = source.as_bytes();
    let mut i = 0;
    // Whether the previous significant token was a `.`.
    let mut after_dot = false;
    // Whether the previous significant tokens were `std` `::`.
    let mut after_std = false;
    let mut last_ident: Option<&str> = None;
    let mut colons = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' | b'\'' => {
                i = skip_quoted(bytes, i, c);
                after_dot = false;
                after_std = false;
                last_ident = None;
                colons = 0;
            }
            b'`' => {
                i = skip_raw(bytes, i);
                after_dot = false;
                after_std = false;
                last_ident = None;
                colons = 0;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
            }
            b'.' => {
                after_dot = true;
                after_std = false;
                last_ident = None;
                colons = 0;
                i += 1;
            }
            b':' => {
                colons += 1;
                if colons == 2 && last_ident == Some("std") {
                    after_std = true;
                }
                i += 1;
            }
            c if is_ident_start(c) => {
                let start = i;
                while i < bytes.len() && is_ident_continue(bytes[i]) {
                    i += 1;
                }
                let name = &source[start..i];
                if after_std {
                    report(Name::Std { member: name });
                }
                if bytes.get(i) == Some(&b'.') {
                    report(Name::Qualified {
                        head: name,
                        after_dot,
                    });
                }
                after_dot = false;
                after_std = false;
                last_ident = Some(name);
                colons = 0;
            }
            c if c.is_ascii_whitespace() => i += 1,
            _ => {
                after_dot = false;
                after_std = false;
                last_ident = None;
                colons = 0;
                i += 1;
            }
        }
    }
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_ident_continue(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// The index just past a quoted literal that opens at `open`, honouring `\`.
fn skip_quoted(bytes: &[u8], open: usize, quote: u8) -> usize {
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            c if c == quote => return i + 1,
            b'\n' => return i,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// The index just past a raw (backtick) string that opens at `open`.
fn skip_raw(bytes: &[u8], open: usize) -> usize {
    let mut i = open + 1;
    while i < bytes.len() && bytes[i] != b'`' {
        i += 1;
    }
    (i + 1).min(bytes.len())
}

#[cfg(test)]
mod tests {
    use super::{cpp_used_in, go_unimported_in, go_used_in};

    #[test]
    fn a_go_package_a_file_already_imports_is_not_missing() {
        let imported = "import (\n\t\"strconv\"\n)\nvar s = strconv.Itoa(1) + math.Pi";
        assert_eq!(go_unimported_in(imported), ["math"]);
        assert!(go_unimported_in("import \"math\"\nvar x = math.Pi").is_empty());
        assert_eq!(go_unimported_in("var s = strconv.Itoa(1)"), ["strconv"]);
    }

    #[test]
    fn a_go_call_names_its_package() {
        assert_eq!(go_used_in("return math.Floor(x)"), ["math"]);
        assert_eq!(
            go_used_in("a := strconv.FormatInt(int64(n), 10) + math.Sqrt(2)"),
            ["math", "strconv"]
        );
    }

    #[test]
    fn a_go_package_name_inside_a_literal_is_not_a_use() {
        assert!(go_used_in(r#"return "math.Floor and strconv.Itoa""#).is_empty());
        assert!(go_used_in("return `strconv.Itoa`").is_empty());
        assert!(go_used_in(r#"return "an \" then math.Pi""#).is_empty());
        assert!(go_used_in("// math.Floor is only a comment\nreturn 1").is_empty());
        assert!(go_used_in("/* strconv.Itoa */ return 1").is_empty());
    }

    #[test]
    fn a_go_member_of_something_else_is_not_a_package() {
        assert!(go_used_in("return cfg.math.Pi").is_empty());
        assert!(go_used_in("return m.strconv.Itoa(1)").is_empty());
    }

    #[test]
    fn a_go_name_that_only_starts_like_a_package_is_not_one() {
        assert!(go_used_in("return mathx.Floor(1) + mystrconv.Itoa(2)").is_empty());
    }

    #[test]
    fn go_nothing_used_is_an_empty_list() {
        assert!(go_used_in("return n + 1").is_empty());
        assert!(go_used_in("").is_empty());
    }

    #[test]
    fn a_cpp_std_name_needs_its_header() {
        assert_eq!(
            cpp_used_in("return std::string(\"E\") + std::to_string(n);"),
            ["string"]
        );
        assert_eq!(cpp_used_in("std::to_string(n)"), ["string"]);
        assert_eq!(cpp_used_in("std::string s;"), ["string"]);
    }

    #[test]
    fn a_cpp_std_name_inside_a_literal_or_comment_is_not_a_use() {
        assert!(cpp_used_in("const char* s = \"std::string is text here\";").is_empty());
        assert!(cpp_used_in("// std::to_string in a comment\nint x;").is_empty());
        assert!(cpp_used_in("/* std::string */ int x;").is_empty());
        assert!(cpp_used_in("char c = '\\'';").is_empty());
    }

    #[test]
    fn a_cpp_name_in_another_namespace_is_not_std() {
        assert!(cpp_used_in("return my::string(x) + other::to_string(y);").is_empty());
        assert!(cpp_used_in("return stdx::string(x);").is_empty());
    }

    #[test]
    fn a_cpp_std_name_outside_the_table_is_left_to_its_template() {
        // `std::vector` and `std::span` are included by the templates that
        // know their element types; this table is for what only text can say.
        assert!(cpp_used_in("std::vector<uint8_t> v;").is_empty());
    }

    #[test]
    fn cpp_nothing_used_is_an_empty_list() {
        assert!(cpp_used_in("return n + 1;").is_empty());
        assert!(cpp_used_in("").is_empty());
    }
}
