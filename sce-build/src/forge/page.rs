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
    Condition,
    Transform,
    Validator,
    Lookup,
    Monitor,
    If,
    Else,
    Foreach,
    In,
    Done,
    While,
    Max,
    Call,
    Send,
    Log,
    /// ⚠ These four are spelled with a space. The reader matches them
    /// as a whole line rather than dispatching on a first word, so a
    /// spelling of two words is what the grammar already has — see the
    /// lexicon test for why that is allowed and what checks it.
    OnEntry,
    OnExit,
    OnInitial,
    OnHistoryDefault,
    State,
    Final,
    Test,
    Param,
    Arg,
    From,
    Expr,
    Observer,
    Domain,
    Filter,
    /// ⚠ A kind name the grammar spends, not a value the document
    /// wrote: `moving-average` is this vocabulary's word for a filter
    /// shape, so a lexicon names it like any other. Leaving it a
    /// literal would have left a page half-translated for no reason a
    /// reader could see.
    MovingAverage,
    LowPass,
    Debounce,
    Window,
    Alpha,
    Algorithm,
    ReturnsMax,
    On,
    When,
    BufferPool,
    Slots,
    Size,
    Section,
    Align,
    Cache,
    Dma,
    Maintain,
    NonCacheable,
    None,
    Link,
    Class,
    Framer,
    Backpressure,
    AcceptStageCopyRate,
    Udp,
    Tcp,
    Serial,
    Websocket,
    RawEth,
    Drop,
    Block,
    SignalEvent,
    Invoke,
    Parallel,
    Initial,
    InitialChildren,
    History,
    Default,
    Unhandled,
}

impl Word {
    /// Every word, so a gate can walk the vocabulary without a list of
    /// its own. ⚠ Derived here rather than written twice: a hand-kept
    /// list is how a word gets added and checked by nothing.
    pub const ALL: &'static [Word] = &[
        Word::Enum,
        Word::Variant,
        Word::Strict,
        Word::AtLine,
        Word::Condition,
        Word::Transform,
        Word::Validator,
        Word::Lookup,
        Word::Monitor,
        Word::If,
        Word::Else,
        Word::Foreach,
        Word::In,
        Word::Done,
        Word::While,
        Word::Max,
        Word::Call,
        Word::Send,
        Word::Log,
        Word::OnEntry,
        Word::OnExit,
        Word::OnInitial,
        Word::OnHistoryDefault,
        Word::State,
        Word::Final,
        Word::Test,
        Word::Param,
        Word::Arg,
        Word::From,
        Word::Expr,
        Word::Observer,
        Word::Domain,
        Word::Filter,
        Word::MovingAverage,
        Word::LowPass,
        Word::Debounce,
        Word::Window,
        Word::Alpha,
        Word::Algorithm,
        Word::ReturnsMax,
        Word::On,
        Word::When,
        Word::BufferPool,
        Word::Slots,
        Word::Size,
        Word::Section,
        Word::Align,
        Word::Cache,
        Word::Dma,
        Word::Maintain,
        Word::NonCacheable,
        Word::None,
        Word::Link,
        Word::Class,
        Word::Framer,
        Word::Backpressure,
        Word::AcceptStageCopyRate,
        Word::Udp,
        Word::Tcp,
        Word::Serial,
        Word::Websocket,
        Word::RawEth,
        Word::Drop,
        Word::Block,
        Word::SignalEvent,
        Word::Invoke,
        Word::Parallel,
        Word::Initial,
        Word::InitialChildren,
        Word::History,
        Word::Default,
        Word::Unhandled,
    ];
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
        Word::Condition => "condition",
        Word::Transform => "transform",
        Word::Validator => "validator",
        Word::Lookup => "lookup",
        Word::Monitor => "monitor",
        Word::If => "if",
        Word::Else => "else",
        Word::Foreach => "foreach",
        Word::In => "in",
        Word::Done => "done",
        Word::While => "while",
        Word::Max => "max",
        Word::Call => "call",
        Word::Send => "send",
        Word::Log => "log",
        Word::OnEntry => "on entry",
        Word::OnExit => "on exit",
        Word::OnInitial => "on initial",
        Word::OnHistoryDefault => "on history-default",
        Word::State => "state",
        Word::Final => "final",
        Word::Test => "test",
        Word::Param => "param",
        Word::Arg => "arg",
        Word::From => "from",
        Word::Expr => "expr",
        Word::Observer => "observer",
        Word::Domain => "domain",
        Word::Filter => "filter",
        Word::MovingAverage => "moving-average",
        Word::LowPass => "low-pass",
        Word::Debounce => "debounce",
        Word::Window => "window",
        Word::Alpha => "alpha",
        Word::Algorithm => "algorithm",
        Word::ReturnsMax => "returns-max",
        Word::On => "on",
        Word::When => "when",
        Word::BufferPool => "buffer-pool",
        Word::Slots => "slots",
        Word::Size => "size",
        Word::Section => "section",
        Word::Align => "align",
        Word::Cache => "cache",
        Word::Dma => "dma",
        Word::Maintain => "maintain",
        Word::NonCacheable => "non-cacheable",
        Word::None => "none",
        Word::Link => "link",
        Word::Class => "class",
        Word::Framer => "framer",
        Word::Backpressure => "backpressure",
        Word::AcceptStageCopyRate => "accept-stage-copy-rate",
        Word::Udp => "udp",
        Word::Tcp => "tcp",
        Word::Serial => "serial",
        Word::Websocket => "websocket",
        // ⚠ An underscore, where every neighbour uses a hyphen. It is
        // what the grammar already writes and what the reader already
        // matches, so this records the inconsistency rather than
        // quietly fixing it — a spelling change here is a change to
        // every page and to the reader, not a tidy-up.
        Word::RawEth => "raw_eth",
        Word::Drop => "drop",
        Word::Block => "block",
        Word::SignalEvent => "signal-event",
        Word::Invoke => "invoke",
        Word::Parallel => "parallel",
        Word::Initial => "initial",
        Word::InitialChildren => "initial-children",
        Word::History => "history",
        Word::Default => "default",
        Word::Unhandled => "unhandled",
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
    /// Text appended with NO separator before it.
    ///
    /// ⚠ For punctuation the construct glues to what precedes it —
    /// `else:` is one word and a colon, and joining parts with a space
    /// would write `else :`. It is a part rather than a rule the shape
    /// applies because deciding WHERE a glyph glues means knowing which
    /// construct is being written, and a shape may not know that.
    Glued(String),
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
                if !first && !matches!(part, Part::Glued(_)) {
                    out.push(' ');
                }
                first = false;
                match part {
                    Part::Word(w) => out.push_str((lexicon.word)(*w)),
                    Part::Text(t) | Part::Glued(t) => out.push_str(t),
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

    /// The one thing a lexicon can be wrong about ON ITS OWN.
    ///
    /// ⚠ This case used to also refuse a word with a space in it, on
    /// the reasoning that a reader finds a clause by its first word.
    /// That reasoning is a PROXY and it is false here: this grammar
    /// also has clauses the reader matches as a whole line — `on
    /// entry:` is one — so a single-word rule refuses spellings the
    /// grammar accepts. The opposite rule, "no word may be a prefix of
    /// another", is false in the other direction: `on` and `on entry`
    /// coexist today because the REST of the line tells them apart.
    ///
    /// ▶ So ambiguity is not a property of a lexicon alone; it is a
    /// property of the lexicon against the grammar. The lexicon's own
    /// rule is what is wrong whatever the grammar turns out to be —
    /// two words with one spelling, and a word with no spelling — and
    /// whether a lexicon can actually be READ is answered where it can
    /// be measured: the round trip, run per lexicon over the corpus.
    /// A proxy that is cheap to check and wrong is worse than a real
    /// check that costs a corpus sweep.
    #[test]
    fn a_lexicon_names_every_word_and_names_no_two_alike() {
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

    #[test]
    fn a_glued_part_takes_no_separator() {
        let nodes = vec![Node {
            depth: 0,
            parts: vec![Part::Word(Word::Strict), Part::Glued(":".into())],
        }];
        assert_eq!("strict:\n", Indent.write(&nodes, &EN));
    }

    /// A line the renderer has not been decomposed into words yet still
    /// reaches the page exactly as it did before.
    #[test]
    fn a_raw_line_is_written_unchanged() {
        let nodes = vec![Node::raw(2, "on entry:".into())];
        assert_eq!("    on entry:\n", Indent.write(&nodes, &EN));
    }
}
