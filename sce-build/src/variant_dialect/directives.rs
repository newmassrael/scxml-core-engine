// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the dialects that select with conditional directives share: C's and
//! C++'s `#if` ([`super::preprocessor`]) and Swift's ([`super::swift`]).
//!
//! Their resolvers read the condition of a directive in the grammar of the
//! selectors (`!`, `&&`, `||`, parentheses and the language's own atoms), keep
//! the group of the first condition that holds, and compare what a build reads
//! with what its variant generates alone as a run of token spellings, each with
//! the offset it stands at.

use std::collections::BTreeMap;

/// The end of a line, as a token of a resolved run: where a language reads the
/// end of a line (a directive's end, a Swift statement's), two runs that break
/// their lines at other places are other runs. No token is spelled so.
pub(crate) const NEWLINE: &str = "\n";

/// A condition over the features, as a resolver reads one written over the
/// spellings a build turns the features on with.
#[derive(Debug, Clone)]
pub(crate) enum Condition {
    Constant(bool),
    Feature(String),
    Not(Box<Condition>),
    All(Vec<Condition>),
    Any(Vec<Condition>),
}

impl Condition {
    /// Its value where `features` answers `Some(on)` for each feature it names,
    /// and `None` where it names one `features` does not answer for.
    pub(crate) fn value(&self, features: &dyn Fn(&str) -> Option<bool>) -> Option<bool> {
        Some(match self {
            Condition::Constant(value) => *value,
            Condition::Feature(name) => features(name)?,
            Condition::Not(operand) => !operand.value(features)?,
            Condition::All(operands) => {
                let mut all = true;
                for operand in operands {
                    all &= operand.value(features)?;
                }
                all
            }
            Condition::Any(operands) => {
                let mut any = false;
                for operand in operands {
                    any |= operand.value(features)?;
                }
                any
            }
        })
    }
}

/// An atom of a language's conditions: it reads one operand at the
/// expression's position, or answers `None` where none stands.
pub(crate) type Atom<'a> = dyn Fn(&mut Expression<'_>) -> Option<Condition> + 'a;

/// Reads a directive's condition in the grammar of the selectors: `||` over
/// `&&` over `!`, parentheses, and the atoms the language reads.
pub(crate) struct Expression<'e> {
    spellings: &'e [&'e str],
    at: usize,
}

impl<'e> Expression<'e> {
    /// `spellings`, all of them, read as one condition; `None` when they are
    /// not one in this grammar.
    pub(crate) fn read(spellings: &'e [&'e str], atom: &Atom<'_>) -> Option<Condition> {
        let mut expression = Expression { spellings, at: 0 };
        let condition = expression.any(atom)?;
        (expression.at == spellings.len()).then_some(condition)
    }

    /// The spelling at the position, not taken.
    pub(crate) fn peek(&self) -> Option<&'e str> {
        self.spellings.get(self.at).copied()
    }

    /// Take the spelling at the position.
    pub(crate) fn next(&mut self) -> Option<&'e str> {
        let spelling = self.peek()?;
        self.at += 1;
        Some(spelling)
    }

    /// Take the spelling at the position when it is `spelling`.
    pub(crate) fn eat(&mut self, spelling: &str) -> bool {
        let found = self.peek() == Some(spelling);
        if found {
            self.at += 1;
        }
        found
    }

    fn any(&mut self, atom: &Atom<'_>) -> Option<Condition> {
        let mut operands = vec![self.all(atom)?];
        while self.eat("||") {
            operands.push(self.all(atom)?);
        }
        Some(match operands.len() {
            1 => operands.pop().expect("one operand"),
            _ => Condition::Any(operands),
        })
    }

    fn all(&mut self, atom: &Atom<'_>) -> Option<Condition> {
        let mut operands = vec![self.unary(atom)?];
        while self.eat("&&") {
            operands.push(self.unary(atom)?);
        }
        Some(match operands.len() {
            1 => operands.pop().expect("one operand"),
            _ => Condition::All(operands),
        })
    }

    fn unary(&mut self, atom: &Atom<'_>) -> Option<Condition> {
        if self.eat("!") {
            return Some(Condition::Not(Box::new(self.unary(atom)?)));
        }
        if self.eat("(") {
            let inner = self.any(atom)?;
            return self.eat(")").then_some(inner);
        }
        atom(self)
    }
}

/// The value in one build of each distinct condition `conditions` holds, in
/// their order; a condition the build does not answer is false.
pub(crate) fn valuation(
    conditions: &BTreeMap<String, Condition>,
    features: &dyn Fn(&str) -> Option<bool>,
) -> Vec<bool> {
    conditions
        .values()
        .map(|condition| condition.value(features).unwrap_or(false))
        .collect()
}

/// Where the run `ours`, resolved from `ours_text`, first differs from
/// `theirs`, resolved from `theirs_text`, or `None` when they are one run.
pub(crate) fn difference(
    ours_text: &str,
    ours: &[(String, usize)],
    theirs_text: &str,
    theirs: &[(String, usize)],
) -> Option<String> {
    let differs = (0..ours.len().max(theirs.len())).find(|&at| {
        ours.get(at).map(|(spelling, _)| spelling) != theirs.get(at).map(|(spelling, _)| spelling)
    })?;
    Some(format!(
        "the build reads {} where the variant reads {}",
        quoted(ours_text, ours, differs),
        quoted(theirs_text, theirs, differs)
    ))
}

/// How many tokens a reported difference quotes on each side of it.
const QUOTED_CONTEXT: usize = 12;

/// The tokens around `at`, and the line the token at `at` is on.
fn quoted(text: &str, tokens: &[(String, usize)], at: usize) -> String {
    let from = at.saturating_sub(QUOTED_CONTEXT);
    let to = (at + QUOTED_CONTEXT).min(tokens.len());
    let words: Vec<&str> = tokens[from..to]
        .iter()
        .map(|(spelling, _)| match spelling.as_str() {
            NEWLINE => "⏎",
            other => other,
        })
        .collect();
    match tokens.get(at) {
        Some((_, offset)) => format!(
            "`{}` at line {}",
            words.join(" "),
            text[..*offset].matches('\n').count() + 1
        ),
        None => format!("`{}` and then the end", words.join(" ")),
    }
}
