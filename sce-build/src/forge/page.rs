// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The page between the model and the text: words, a lexicon, a shape.
//!
//! [`crate::forge::pseudo`] used to build its output as strings, so the
//! grammar's words were 220 string literals scattered through it and
//! the layout was decided at each one. That is renderable and it is not
//! selectable: a reader who wants the page in their own language, or a
//! reviewer who reads blocks better than indentation, cannot be given
//! either without editing the renderer.
//!
//! So the renderer now emits [`Node`]s — a depth, and the parts of a
//! line — and two parameters turn those into text:
//!
//!   * a [`Lexicon`] says what each [`Word`] is called
//!   * a [`Shape`] says how lines and nesting are written
//!
//! # ⚠ What a shape may and may not do
//!
//! A shape **joins** parts and decides nesting. It may not touch a
//! part's text. That is the rule the whole surface rests on: a value
//! reaches the page as the author spelled it, and a page whose shape
//! inflected, wrapped or case-folded a value could not be read back to
//! the value. Surrounding is free; altering is not.
//!
//! ⚠ A shape is also **kind-agnostic** — it sees words and text, never
//! "this is a codec". A shape that knew kinds would have to be revised
//! every time a document kind is added, and the kind-specific mapping
//! would stop being one copy.
//!
//! # ⚠ Where punctuation lives, which is not where the first sketch put it
//!
//! Measured while building this: the grammar's punctuation is
//! **per construct** — `enum <name>: <type>`, `field <id>: <type> at
//! byte <n>`, `-> <target> [<type>]` — and a kind-agnostic shape cannot
//! know which construct it is looking at, so it cannot place those
//! glyphs. Handing the shape that job would either make it kind-aware
//! or change the default page, and the default page is what every gate
//! and every approval already means.
//!
//! So punctuation belongs to the mapping and travels inside
//! [`Part::Text`]. A shape's freedom is over **words, separators
//! between parts, and how nesting opens and closes** — which is what a
//! second shape actually needs, and no more.

use std::fmt::Write as _;

/// A word the grammar spends, as opposed to text a document wrote.
///
/// ⚠ An enum rather than a table of strings so that a lexicon cannot
/// be partial: every lexicon answers with an exhaustive `match`, and a
/// word added here stops the build until each one has a name for it.
/// A missing word would otherwise reach the page in the default
/// language and read as though the page were mixed on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Word {
    Enum,
    Variant,
    Strict,
    AtLine,
}

impl Word {
    /// Every word, so a gate can walk the vocabulary without a list of
    /// its own. ⚠ Derived here rather than written twice: a hand-kept
    /// list is how a word gets added and checked by nothing.
    pub const ALL: &'static [Word] = &[Word::Enum, Word::Variant, Word::Strict, Word::AtLine];
}

/// What the words are called.
pub struct Lexicon {
    /// How a page names this lexicon when it declares itself.
    pub name: &'static str,
    /// ⚠ A function with an exhaustive `match`, not a map: a map can be
    /// missing a key at run time and a `match` cannot be missing an arm
    /// at build time.
    pub word: fn(Word) -> &'static str,
}

/// The default, and the one every gate and every approval means.
pub const EN: Lexicon = Lexicon {
    name: "en",
    word: en_word,
};

fn en_word(w: Word) -> &'static str {
    match w {
        Word::Enum => "enum",
        Word::Variant => "variant",
        Word::Strict => "strict",
        Word::AtLine => "@line",
    }
}

/// One piece of a line: a word the grammar owns, or text the mapping
/// built.
///
/// ⚠ `Text` may carry punctuation the construct glues to a value
/// (`nrc: uint8`). That is not a shape decision — see the module note.
/// What it may never do is arrive altered: whatever the mapping put
/// here reaches the page unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Word(Word),
    Text(String),
}

/// One line of the page, before a shape has written it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub depth: usize,
    pub parts: Vec<Part>,
}

impl Node {
    /// A line this module has not been taught the words of yet.
    ///
    /// ⚠ Migration scaffolding, and it is deliberately visible: a shape
    /// that needs to know a line's leading word — `endmark`, to close
    /// the block it opened — cannot serve a node built this way. The
    /// second shape therefore cannot ship while any of these remain,
    /// which is the dependency stated rather than discovered.
    pub fn raw(depth: usize, text: String) -> Node {
        Node {
            depth,
            parts: vec![Part::Text(text)],
        }
    }
}

/// How lines and nesting are written.
pub trait Shape {
    /// How a page names this shape when it declares itself.
    fn name(&self) -> &'static str;

    /// Write the whole page.
    fn write(&self, nodes: &[Node], lexicon: &Lexicon) -> String;
}

/// Nesting by indentation, two spaces per level: the default, and the
/// shape every existing gate and golden measures.
pub struct Indent;

impl Shape for Indent {
    fn name(&self) -> &'static str {
        "indent"
    }

    fn write(&self, nodes: &[Node], lexicon: &Lexicon) -> String {
        let mut out = String::new();
        for node in nodes {
            for _ in 0..node.depth {
                out.push_str("  ");
            }
            let mut first = true;
            for part in &node.parts {
                if !first {
                    out.push(' ');
                }
                first = false;
                match part {
                    Part::Word(w) => out.push_str((lexicon.word)(*w)),
                    Part::Text(t) => out.push_str(t),
                }
            }
            let _ = writeln!(out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠ The condition a reader depends on: it finds a clause by its
    /// first word, so two words that read the same are two clauses it
    /// cannot tell apart, and a word with a space in it is two words.
    /// Checked over the vocabulary itself rather than a list, so a word
    /// added without a name here is caught by the same case.
    #[test]
    fn a_lexicon_names_every_word_distinctly_and_without_whitespace() {
        for lexicon in [&EN] {
            let mut seen: std::collections::BTreeMap<&str, Word> = Default::default();
            for &w in Word::ALL {
                let name = (lexicon.word)(w);
                assert!(
                    !name.is_empty(),
                    "lexicon {} leaves {w:?} unnamed, so that clause would \
                     vanish from the page",
                    lexicon.name
                );
                assert!(
                    !name.chars().any(char::is_whitespace),
                    "lexicon {} calls {w:?} {name:?}, which is two words to a \
                     reader that finds a clause by its first one",
                    lexicon.name
                );
                if let Some(other) = seen.insert(name, w) {
                    panic!(
                        "lexicon {} calls both {other:?} and {w:?} {name:?}, so \
                         a page cannot say which one it meant",
                        lexicon.name
                    );
                }
            }
        }
    }

    #[test]
    fn indent_writes_two_spaces_a_level_and_one_space_between_parts() {
        let nodes = vec![
            Node {
                depth: 0,
                parts: vec![Part::Word(Word::Enum), Part::Text("nrc: uint8".into())],
            },
            Node {
                depth: 1,
                parts: vec![
                    Part::Word(Word::Variant),
                    Part::Text("reject = 0x10".into()),
                    Part::Word(Word::AtLine),
                    Part::Text("29".into()),
                ],
            },
        ];
        assert_eq!(
            "enum nrc: uint8\n  variant reject = 0x10 @line 29\n",
            Indent.write(&nodes, &EN)
        );
    }

    /// A line the renderer has not been decomposed into words yet still
    /// reaches the page exactly as it did before.
    #[test]
    fn a_raw_line_is_written_unchanged() {
        let nodes = vec![Node::raw(2, "on entry:".into())];
        assert_eq!("    on entry:\n", Indent.write(&nodes, &EN));
    }
}
