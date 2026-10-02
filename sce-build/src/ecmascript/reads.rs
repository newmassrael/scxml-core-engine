// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The names an expression reads, as the lexer sees them.
//!
//! A question some analyzers ask of an expression is not whether it is
//! well-formed (the parser answers that, and refuses a bad one on its own
//! terms) but WHICH of the document's data items it touches: a `<send>`'s
//! `targetexpr` that reads `callerTarget` depends on whatever `callerTarget`
//! is, and if that is a question the specification left open, so is the send.
//!
//! ⚠ Deliberately the lexer and not the AST. The caller intersects the answer
//! with the document's declared data, so a name that is not a read of a data
//! item (an object key, a function parameter) costs nothing when it happens to
//! be spelled like one, and an expression the parser would refuse still has its
//! names listed here, which is the better behaviour for an analyzer that must
//! say what a malformed route depends on. A member name after `.` is not a read
//! of a variable and is left out.

use crate::forge::expr::{tokenize_as, LexMode, Token};

/// Every identifier `source` names as a value, in the order first written. Empty
/// when the text does not tokenize: nothing is then known to be read.
pub fn identifiers_read(source: &str) -> Vec<String> {
    let Ok(tokens) = tokenize_as(source, LexMode::EcmaScript) else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    let mut after_dot = false;
    for token in &tokens {
        if let Token::Ident(name) = token {
            if !after_dot && !names.contains(name) {
                names.push(name.clone());
            }
        }
        after_dot = matches!(token, Token::Dot);
    }
    names
}

#[cfg(test)]
mod tests {
    use super::identifiers_read;

    #[test]
    fn names_are_listed_once_in_the_order_written() {
        assert_eq!(
            identifiers_read("callerKind + '/' + callerTarget + callerKind"),
            ["callerKind", "callerTarget"]
        );
    }

    #[test]
    fn a_member_after_a_dot_is_not_a_read_of_a_variable() {
        assert_eq!(
            identifiers_read("config.target + _event.origin"),
            ["config", "_event"]
        );
    }

    #[test]
    fn a_literal_reads_nothing_and_text_that_does_not_tokenize_reads_nothing() {
        assert!(identifiers_read("'#_parent'").is_empty());
        assert!(identifiers_read("").is_empty());
        assert!(identifiers_read("'unterminated").is_empty());
    }
}
