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
    /// An algorithm local's declaration. A record local opens a block —
    /// one nested `field = expr` line per field — so its line must name
    /// the word a shape closes it with.
    Var,
    /// A statechart datamodel variable's declaration. A `record:<alias>`
    /// variable opens a block — one nested `field = expr` line per field —
    /// so its line must name the word a shape closes it with.
    Data,
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
    NoTarget,
    NativeGuard,
    Index,
    Elif,
    Codec,
    Endian,
    InputLength,
    Big,
    Little,
    Native,
    TagField,
    TagFlag,
    PeekByte,
    Machine,
    EventSchema,
    Event,
    Child,
    Assign,
    Content,
    Const,
    Field,
    Procedure,
    Interpolation,
    Method,
    OutOfBounds,
    Linear,
    Bilinear,
    Clamp,
    Extrapolate,
    Error,
    /// ⚠ The arrow is a WORD, not punctuation glued to a value. An
    /// eventless transition begins with it, so it is the only thing
    /// that can name that line — and a lexicon that wanted a different
    /// arrow could not reach it while it was a literal.
    Arrow,
    Timer,
    BoundedCollection,
    Worker,
    /// The two a shape spends rather than the mapping: a block's open
    /// and close, for a shape that marks them instead of indenting.
    /// ⚠ They are WORDS and not shape-private literals for the same
    /// reason every other word is one — a reader in another language
    /// needs them named, and `반복문 시작` is precisely this pair.
    Begin,
    End,
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
        Word::Var,
        Word::Data,
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
        Word::NoTarget,
        Word::NativeGuard,
        Word::Index,
        Word::Elif,
        Word::Codec,
        Word::Endian,
        Word::InputLength,
        Word::Big,
        Word::Little,
        Word::Native,
        Word::TagField,
        Word::TagFlag,
        Word::PeekByte,
        Word::Machine,
        Word::EventSchema,
        Word::Event,
        Word::Child,
        Word::Assign,
        Word::Content,
        Word::Const,
        Word::Field,
        Word::Procedure,
        Word::Interpolation,
        Word::Method,
        Word::OutOfBounds,
        Word::Linear,
        Word::Bilinear,
        Word::Clamp,
        Word::Extrapolate,
        Word::Error,
        Word::Arrow,
        Word::Timer,
        Word::BoundedCollection,
        Word::Worker,
        Word::Begin,
        Word::End,
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
        Word::Var => "var",
        Word::Data => "data",
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
        // ⚠ A phrase in parentheses, and it is a WORD: the grammar
        // says "this transition names no target" in so many words,
        // because an absent target is the difference between changing
        // state and staying put. A lexicon that left it English would
        // leave the one clause a reviewer most needs to read untranslated.
        Word::NoTarget => "(no target)",
        Word::NativeGuard => "native-guard",
        Word::Index => "index",
        Word::Elif => "elif",
        Word::Codec => "codec",
        Word::Endian => "endian",
        Word::InputLength => "input-length",
        Word::Big => "big",
        Word::Little => "little",
        Word::Native => "native",
        Word::TagField => "tag-field",
        Word::TagFlag => "tag-flag",
        Word::PeekByte => "peek-byte",
        Word::Machine => "machine",
        Word::EventSchema => "event-schema",
        Word::Event => "event",
        Word::Child => "child",
        Word::Assign => "assign",
        Word::Content => "content",
        Word::Const => "const",
        Word::Field => "field",
        Word::Procedure => "procedure",
        Word::Interpolation => "interpolation",
        Word::Method => "method",
        Word::OutOfBounds => "out-of-bounds",
        Word::Linear => "linear",
        Word::Bilinear => "bilinear",
        Word::Clamp => "clamp",
        Word::Extrapolate => "extrapolate",
        Word::Error => "error",
        Word::Arrow => "->",
        Word::Timer => "timer",
        Word::BoundedCollection => "bounded-collection",
        Word::Worker => "worker",
        Word::Begin => "begin",
        Word::End => "end",
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

/// A page a shape will not write, and why.
///
/// ⚠ A refusal rather than a best effort, for the reason
/// [`crate::forge::pseudo`] refuses a document it cannot show in full:
/// a page that silently dropped a block's close would read like a page
/// that had no block there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub shape: &'static str,
    pub why: String,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the '{}' shape cannot write {}", self.shape, self.why)
    }
}

/// How lines and nesting are written.
pub trait Shape {
    /// How a page names this shape when it declares itself.
    fn name(&self) -> &'static str;

    /// Write the whole page.
    fn write(&self, nodes: &[Node], lexicon: &Lexicon) -> Result<String, Refusal>;

    /// This shape's page, back as the canonical one.
    ///
    /// ⚠ **Not a second reader.** `crate::forge::unpseudo` reads the
    /// canonical page — `indent` with `EN` — and it is the only thing
    /// that knows the grammar. A shape that had its own reader would
    /// be a second answer to "what does this document say", and the
    /// one consulted less often is the one that rots. So a shape owes
    /// exactly this: undo its own layout and its own words, and hand
    /// back the page the reader already reads.
    ///
    /// The law that follows, and the only one a new shape has to
    /// satisfy: `normalise(write(nodes)) == Indent.write(nodes, EN)`,
    /// byte for byte, over the corpus.
    fn normalise(&self, page: &str, lexicon: &Lexicon) -> Result<String, Refusal>;
}

/// The canonical page: the one the reader reads and every gate means.
pub fn canonical(nodes: &[Node]) -> String {
    Indent
        .write(nodes, &EN)
        .expect("the indent shape refuses nothing")
}

/// One line's words put back into the canonical spelling.
///
/// ⚠ Longest first. A lexicon may spell one word as another's prefix —
/// `on` and `on entry` do exactly that in `EN` — and replacing the
/// short one first would eat the long one's head and leave its tail as
/// text. Sorting by length is what makes the substitution independent
/// of the order the vocabulary happens to be listed in.
fn to_canonical_words(line: &str, lexicon: &Lexicon) -> String {
    if std::ptr::eq(lexicon.word as *const (), EN.word as *const ()) {
        return line.to_string();
    }
    let mut pairs: Vec<(&str, &str)> = Word::ALL
        .iter()
        .map(|&w| ((lexicon.word)(w), (EN.word)(w)))
        .collect();
    pairs.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));
    let mut out = line.to_string();
    for (from, to) in pairs {
        if from != to {
            out = out.replace(from, to);
        }
    }
    out
}

/// Whether the node at `i` is the one that opens the block under it.
///
/// ⚠ Read off the page rather than marked by the renderer. A marker
/// would be a second statement of the same fact, and the two would
/// drift the first time a renderer nested something without setting
/// it.
fn opens_a_block(nodes: &[Node], i: usize) -> bool {
    nodes
        .get(i + 1)
        .is_some_and(|next| next.depth > nodes[i].depth)
}

/// The parts of one line, joined.
fn join_parts(node: &Node, lexicon: &Lexicon, skip_first_word: bool) -> String {
    let mut out = String::new();
    let mut first = true;
    for (n, part) in node.parts.iter().enumerate() {
        if skip_first_word && n == 0 {
            continue;
        }
        if !first && !matches!(part, Part::Glued(_)) {
            out.push(' ');
        }
        first = false;
        match part {
            Part::Word(w) => out.push_str((lexicon.word)(*w)),
            Part::Text(t) | Part::Glued(t) => out.push_str(t),
        }
    }
    out
}

/// Nesting by indentation, two spaces per level: the default, and the
/// shape every existing gate and golden measures.
pub struct Indent;

impl Shape for Indent {
    fn name(&self) -> &'static str {
        "indent"
    }

    fn write(&self, nodes: &[Node], lexicon: &Lexicon) -> Result<String, Refusal> {
        let mut out = String::new();
        for node in nodes {
            for _ in 0..node.depth {
                out.push_str("  ");
            }
            out.push_str(&join_parts(node, lexicon, false));
            let _ = writeln!(out);
        }
        // ⚠ Never refuses. Indentation needs to know nothing about a
        // line, which is why it can serve a page whose words are only
        // partly known — and why it alone could be the default while
        // the renderer was being decomposed.
        Ok(out)
    }

    fn normalise(&self, page: &str, lexicon: &Lexicon) -> Result<String, Refusal> {
        // The layout is already canonical; only the words can differ.
        let mut out = String::new();
        for line in page.lines() {
            let indent = line.len() - line.trim_start().len();
            out.push_str(&line[..indent]);
            out.push_str(&to_canonical_words(line.trim_start(), lexicon));
            let _ = writeln!(out);
        }
        Ok(out)
    }
}

/// Nesting by an explicit close, and no indentation.
///
/// ```text
/// state s0 begin:
/// on e -> t [external]
/// state end
/// ```
///
/// ⚠ It needs the opener's WORD, because the close repeats it — that
/// is what makes an end marker readable at all, and it is why
/// `a_line_that_opens_a_block_carries_its_word` exists. A page whose
/// opener is raw is refused by name rather than closed with a guess.
///
/// ⚠ The open word goes right after the keyword, BEFORE the values,
/// not at the end of the line. Free text runs to the end of a line by
/// contract, so a marker after it would be a word the reader could not
/// tell from the value it follows.
pub struct Endmark;

impl Shape for Endmark {
    fn name(&self) -> &'static str {
        "endmark"
    }

    fn write(&self, nodes: &[Node], lexicon: &Lexicon) -> Result<String, Refusal> {
        let mut out = String::new();
        // The word of each block still open, innermost last. Its length
        // is the depth those lines sit at, which is how a line says
        // which blocks it has left.
        let mut open: Vec<Word> = Vec::new();
        let close = |out: &mut String, w: Word| {
            let _ = writeln!(out, "{} {}", (lexicon.word)(w), (lexicon.word)(Word::End));
        };

        for (i, node) in nodes.iter().enumerate() {
            while open.len() > node.depth {
                close(&mut out, open.pop().expect("len checked"));
            }
            if opens_a_block(nodes, i) {
                let Some(Part::Word(w)) = node.parts.first() else {
                    return Err(Refusal {
                        shape: "endmark",
                        why: format!(
                            "`{}`, which opens a block and names no word to \
                             close it with",
                            join_parts(node, lexicon, false)
                        ),
                    });
                };
                // ⚠ The rest of the line is carried VERBATIM, taken
                // off the written line rather than re-joined from the
                // parts. Re-joining puts a space where the canonical
                // line had none: `log:` glues its colon to the word,
                // and a normaliser reading text cannot know that the
                // space it sees was invented. Splitting the written
                // line keeps the question from arising.
                let spelling = (lexicon.word)(*w);
                let full = join_parts(node, lexicon, false);
                let rest = &full[spelling.len()..];
                let _ = writeln!(out, "{spelling} {}{rest}", (lexicon.word)(Word::Begin));
                open.push(*w);
            } else {
                let _ = writeln!(out, "{}", join_parts(node, lexicon, false));
            }
        }
        while let Some(w) = open.pop() {
            close(&mut out, w);
        }
        Ok(out)
    }

    /// ⚠ The depth is rebuilt from the markers, which is the whole
    /// point of them: this shape threw the indentation away, so the
    /// close lines are the only record of where a block ended. A close
    /// that names a word no open is waiting for is a page this shape
    /// did not write, and it is refused rather than guessed at.
    fn normalise(&self, page: &str, lexicon: &Lexicon) -> Result<String, Refusal> {
        let begin = (lexicon.word)(Word::Begin);
        let end = (lexicon.word)(Word::End);
        // ⚠ Longest spelling first, and matched as a STRING rather
        // than by counting tokens. A word may be spelled with a space
        // — `on entry` is — so "the second token is the open marker"
        // is false for exactly those lines, and the close that
        // followed then arrived with nothing open. The same hazard the
        // word substitution already handles by sorting; this is its
        // structural half, and leaving one fixed and the other naive
        // is how a defect grows a second face.
        let mut words: Vec<Word> = Word::ALL.to_vec();
        words.sort_by_key(|&w| std::cmp::Reverse((lexicon.word)(w).len()));

        let mut out = String::new();
        let mut depth = 0usize;

        for line in page.lines() {
            let opened = words
                .iter()
                .copied()
                .find(|&w| line.starts_with(&format!("{} {begin}", (lexicon.word)(w))));
            let closed = words
                .iter()
                .copied()
                .find(|&w| line == format!("{} {end}", (lexicon.word)(w)));

            if let Some(w) = closed {
                let _ = w;
                depth = depth.checked_sub(1).ok_or_else(|| Refusal {
                    shape: "endmark",
                    why: format!("`{line}`, a close with no block open above it"),
                })?;
                continue;
            }

            for _ in 0..depth {
                out.push_str("  ");
            }
            if let Some(w) = opened {
                // The canonical line is the same one without the open
                // marker: `<word> <rest…>`.
                // The canonical line is this one with the open marker
                // removed — the rest travelled verbatim, so putting it
                // back needs no separator decision.
                let spelling = (lexicon.word)(w);
                let rest = line
                    .strip_prefix(&format!("{spelling} {begin}"))
                    .unwrap_or_default();
                let _ = writeln!(out, "{}{}", (EN.word)(w), to_canonical_words(rest, lexicon));
                depth += 1;
            } else {
                let _ = writeln!(out, "{}", to_canonical_words(line, lexicon));
            }
        }
        if depth != 0 {
            return Err(Refusal {
                shape: "endmark",
                why: format!("a page that leaves {depth} block(s) unclosed"),
            });
        }
        Ok(out)
    }
}

/// A first Korean vocabulary.
///
/// ⚠⚠ **The words here are a judgement, not a measurement, and the
/// owner's to revise.** What this file can hold them to is only what is
/// wrong whatever the vocabulary turns out to be — no two words
/// spelled alike, no word left unspelled — and whether a page written
/// with them reads back, which the corpus answers. Which Korean term
/// belongs to a `while` is not a question a gate can settle, so it is
/// written down here to be argued with rather than buried.
///
/// Four of these are the owner's own: `반복문`, `시작`, `종료` and
/// `출력` come from the example this shape was built for.
///
/// ⚠ A protocol's own name is NOT translated — `udp`, `tcp`,
/// `raw_eth`, `dma` are what the platform calls them, and a page that
/// renamed them would be describing a different system. The rule is
/// the same one that keeps a document's ids untouched: a name the
/// author did not choose is not this surface's to change.
pub const KO: Lexicon = Lexicon {
    name: "ko",
    word: ko_word,
};

fn ko_word(w: Word) -> &'static str {
    match w {
        Word::Enum => "열거",
        Word::Variant => "열거값",
        Word::Strict => "엄격",
        Word::AtLine => "@줄",
        Word::Condition => "조건",
        Word::Transform => "변환",
        Word::Validator => "검증",
        Word::Lookup => "조회표",
        Word::Monitor => "감시",
        Word::If => "만약",
        Word::Else => "아니면",
        Word::Foreach => "반복문",
        Word::In => "범위",
        Word::Done => "완료",
        Word::While => "조건반복",
        Word::Max => "최대",
        Word::Var => "변수",
        Word::Data => "데이터",
        Word::Call => "호출",
        Word::Send => "전송",
        Word::Log => "출력",
        Word::OnEntry => "진입 시",
        Word::OnExit => "이탈 시",
        Word::OnInitial => "초기 시",
        Word::OnHistoryDefault => "이력기본 시",
        Word::State => "상태",
        Word::Final => "종료상태",
        Word::Test => "시험",
        Word::Param => "인자",
        Word::Arg => "인수",
        Word::From => "위치",
        Word::Expr => "식",
        Word::Observer => "관찰",
        Word::Domain => "영역",
        Word::Filter => "필터",
        Word::MovingAverage => "이동평균",
        Word::LowPass => "저역통과",
        Word::Debounce => "디바운스",
        Word::Window => "창",
        Word::Alpha => "알파",
        Word::Algorithm => "알고리즘",
        Word::ReturnsMax => "반환최대",
        Word::On => "사건",
        Word::When => "일때",
        Word::BufferPool => "버퍼풀",
        Word::Slots => "슬롯수",
        Word::Size => "크기",
        Word::Section => "섹션",
        Word::Align => "정렬",
        Word::Cache => "캐시",
        Word::Dma => "dma",
        Word::Maintain => "유지",
        Word::NonCacheable => "캐시불가",
        Word::None => "없음",
        Word::Link => "링크",
        Word::Class => "종류",
        Word::Framer => "프레이머",
        Word::Backpressure => "배압",
        Word::AcceptStageCopyRate => "스테이지복사율허용",
        Word::Udp => "udp",
        Word::Tcp => "tcp",
        Word::Serial => "serial",
        Word::Websocket => "websocket",
        Word::RawEth => "raw_eth",
        Word::Drop => "버림",
        Word::Block => "막음",
        Word::SignalEvent => "사건통지",
        Word::Invoke => "위임",
        Word::Parallel => "병렬",
        Word::Initial => "초기",
        Word::InitialChildren => "초기자식",
        Word::History => "이력",
        Word::Default => "기본",
        Word::Unhandled => "미처리",
        Word::NoTarget => "(목표 없음)",
        Word::NativeGuard => "네이티브가드",
        Word::Index => "색인",
        Word::Elif => "아니면만약",
        Word::Codec => "코덱",
        Word::Endian => "엔디언",
        Word::InputLength => "입력길이",
        Word::Big => "빅",
        Word::Little => "리틀",
        Word::Native => "네이티브",
        Word::TagField => "태그필드",
        Word::TagFlag => "태그플래그",
        Word::PeekByte => "미리보기바이트",
        Word::Machine => "기계",
        Word::EventSchema => "사건스키마",
        Word::Event => "사건이름",
        Word::Child => "자식",
        Word::Assign => "대입",
        Word::Content => "내용",
        Word::Const => "상수",
        Word::Field => "필드",
        Word::Procedure => "절차",
        Word::Interpolation => "보간",
        Word::Method => "방식",
        Word::OutOfBounds => "범위밖",
        Word::Linear => "선형",
        Word::Bilinear => "이중선형",
        Word::Clamp => "고정",
        Word::Extrapolate => "외삽",
        Word::Error => "오류",
        // ⚠ Unchanged. The arrow is a word so that a lexicon COULD
        // change it, and this one chooses not to: a Korean reader
        // reads `->` as an arrow already, and a different glyph would
        // be a change with a cost and no reading to gain.
        Word::Arrow => "->",
        Word::Timer => "타이머",
        Word::BoundedCollection => "유한컬렉션",
        Word::Worker => "작업자",
        Word::Begin => "시작",
        Word::End => "종료",
    }
}

/// Every shape a caller may choose.
///
/// ⚠ The registry, and the population every per-shape law is derived
/// from. A shape added here without a law is caught by the law's own
/// sweep, which walks THIS list rather than one written beside it —
/// the correction this session made five times over.
pub const SHAPES: &[&dyn Shape] = &[&Indent, &Endmark];

/// Every lexicon a caller may choose.
pub const LEXICONS: &[&Lexicon] = &[&EN, &KO];

// ── Choosing a pair, and the page saying which one it is ───────

/// The shape a caller named, if the registry has one.
///
/// ⚠ Looked up in [`SHAPES`] rather than matched on a list written
/// here, so a shape becomes selectable on the day it is registered and
/// not on the day somebody remembers this function.
pub fn shape_named(name: &str) -> Option<&'static dyn Shape> {
    SHAPES.iter().copied().find(|s| s.name() == name)
}

/// The lexicon a caller named, if the registry has one.
pub fn lexicon_named(name: &str) -> Option<&'static Lexicon> {
    LEXICONS.iter().copied().find(|l| l.name == name)
}

/// Every shape name, for a caller telling somebody what they may pick.
pub fn shape_names() -> Vec<&'static str> {
    SHAPES.iter().map(|s| s.name()).collect()
}

/// Every lexicon name, for the same reason.
pub fn lexicon_names() -> Vec<&'static str> {
    LEXICONS.iter().map(|l| l.name).collect()
}

/// What a page's first line begins with when it declares itself.
///
/// ⚠ Never translated, and not spelled by any lexicon. It has to be
/// legible BEFORE the lexicon is known — that is the whole point of
/// it — so putting it in a lexicon would make reading the declaration
/// require the answer the declaration carries.
pub const DECLARATION: &str = "#!sce-pseudo";

/// A page whose own first line says how to read it, and cannot be
/// obeyed.
///
/// ⚠ Refused rather than read as the default pair. A page that says
/// `lexicon=de` is a page written by something this build does not
/// have; reading it as `en` would succeed on every line whose words
/// happen to coincide and quietly mis-read the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undeclared {
    /// The declaration as it stands on the page.
    pub line: String,
    /// What is wrong with it, as a clause.
    pub why: String,
}

impl std::fmt::Display for Undeclared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the page declares '{}', and {}", self.line, self.why)
    }
}

/// Why a page could not be handed back as the canonical one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotCanonical {
    /// The page's first line cannot be obeyed.
    Undeclared(Undeclared),
    /// The shape the page names refused its own page.
    Refused(Refusal),
}

impl std::fmt::Display for NotCanonical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotCanonical::Undeclared(u) => write!(f, "{u}"),
            NotCanonical::Refused(r) => write!(f, "{r}"),
        }
    }
}

/// Whether this pair is the one a page with no declaration means.
///
/// ⚠ By name, because the registry's names are unique — see
/// `every_registered_shape_has_its_own_name` and
/// `every_registered_lexicon_has_its_own_name` — and a trait object
/// has no cheaper identity than that.
fn is_default(shape: &dyn Shape, lexicon: &Lexicon) -> bool {
    shape.name() == Indent.name() && lexicon.name == EN.name
}

/// The whole page: a declaration when one is owed, then the body.
///
/// # ⚠ Why the default page carries no declaration
///
/// One rule, and it is total: **a page declares every choice that is
/// not the default, and a page with no declaration made none.** So
/// every page still answers "how do I read you" without anybody
/// guessing — which is what an approved page being filed and handed
/// on requires — and the default page stays byte for byte what every
/// gate, every golden and every approval to date already means.
///
/// The declaration names BOTH halves even when only one differs, so a
/// reader never has to know what the defaults were on the day it was
/// written.
pub fn write_page(nodes: &[Node], shape: &dyn Shape, lexicon: &Lexicon) -> Result<String, Refusal> {
    let body = shape.write(nodes, lexicon)?;
    if is_default(shape, lexicon) {
        return Ok(body);
    }
    Ok(format!(
        "{DECLARATION} shape={} lexicon={}\n{body}",
        shape.name(),
        lexicon.name
    ))
}

/// The pair a page was written in, and the page after its declaration.
///
/// A page with no declaration is the default pair and is its own body.
pub fn read_page(page: &str) -> Result<(&'static dyn Shape, &'static Lexicon, &str), Undeclared> {
    let Some(rest) = page.strip_prefix(DECLARATION) else {
        return Ok((&Indent, &EN, page));
    };
    let (line, body) = match rest.split_once('\n') {
        Some((l, b)) => (l, b),
        None => (rest, ""),
    };
    let declared = |why: String| Undeclared {
        line: format!("{DECLARATION}{line}"),
        why,
    };

    let mut shape: Option<&'static dyn Shape> = None;
    let mut lexicon: Option<&'static Lexicon> = None;
    for field in line.split_whitespace() {
        let Some((key, value)) = field.split_once('=') else {
            return Err(declared(format!(
                "'{field}' is not a 'key=value' the declaration can carry"
            )));
        };
        match key {
            "shape" => match shape_named(value) {
                Some(s) => shape = Some(s),
                None => {
                    return Err(declared(format!(
                        "this build has no '{value}' shape — it has {}",
                        shape_names().join(", ")
                    )))
                }
            },
            "lexicon" => match lexicon_named(value) {
                Some(l) => lexicon = Some(l),
                None => {
                    return Err(declared(format!(
                        "this build has no '{value}' lexicon — it has {}",
                        lexicon_names().join(", ")
                    )))
                }
            },
            other => {
                return Err(declared(format!(
                    "'{other}' is not something a declaration says"
                )))
            }
        }
    }

    // ⚠ Both halves are required once a declaration is present. A page
    // that named only its shape would have to be read with whatever
    // this build calls the default lexicon, which is exactly the guess
    // the declaration exists to remove.
    match (shape, lexicon) {
        (Some(s), Some(l)) => Ok((s, l, body)),
        (None, _) => Err(declared("it does not say which shape".to_string())),
        (_, None) => Err(declared("it does not say which lexicon".to_string())),
    }
}

/// Any page, back as the canonical one the reader reads.
///
/// ⚠ This is the entry point a stored page goes through. It is what
/// makes a page in a chosen shape and a chosen language something that
/// can be filed, handed on, and later read back into the document —
/// rather than a rendering that is only legible to whoever asked for
/// it while they still remember what they asked for.
pub fn normalise_page(page: &str) -> Result<String, NotCanonical> {
    let (shape, lexicon, body) = read_page(page).map_err(NotCanonical::Undeclared)?;
    shape
        .normalise(body, lexicon)
        .map_err(NotCanonical::Refused)
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
        // ⚠ Walks the REGISTRY, so a lexicon added without a name for
        // some word, or with one it already spends, is caught the day
        // it is registered rather than the day somebody renders with
        // it.
        for lexicon in LEXICONS {
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
            Indent.write(&nodes, &EN).unwrap()
        );
    }

    #[test]
    fn a_glued_part_takes_no_separator() {
        let nodes = vec![Node {
            depth: 0,
            parts: vec![Part::Word(Word::Strict), Part::Glued(":".into())],
        }];
        assert_eq!("strict:\n", Indent.write(&nodes, &EN).unwrap());
    }

    /// The shape the second one exists for: blocks closed by name,
    /// and no indentation carrying the structure.
    #[test]
    fn endmark_closes_each_block_with_the_word_that_opened_it() {
        let nodes = vec![
            Node {
                depth: 0,
                parts: vec![
                    Part::Word(Word::Foreach),
                    Part::Text("a".into()),
                    Part::Word(Word::In),
                    Part::Text("0..9".into()),
                    Part::Glued(":".into()),
                ],
            },
            Node {
                depth: 1,
                parts: vec![Part::Word(Word::Log), Part::Text("a * b".into())],
            },
        ];
        assert_eq!(
            "foreach begin a in 0..9:\nlog a * b\nforeach end\n",
            Endmark.write(&nodes, &EN).unwrap()
        );
    }

    /// ⚠ The case the corpus found and eight lines had not: a word
    /// with punctuation glued to it. Re-joining the rest around the
    /// open marker put a space where the canonical page has none, and
    /// 624 pages differed by exactly that byte.
    #[test]
    fn endmark_keeps_punctuation_glued_to_the_word_that_opens() {
        let nodes = vec![
            Node {
                depth: 0,
                parts: vec![Part::Word(Word::Log), Part::Glued(":".into())],
            },
            Node {
                depth: 1,
                parts: vec![Part::Word(Word::Expr), Part::Text("x".into())],
            },
        ];
        let page = Endmark.write(&nodes, &EN).unwrap();
        assert_eq!("log begin:\nexpr x\nlog end\n", page);
        assert_eq!(canonical(&nodes), Endmark.normalise(&page, &EN).unwrap());
    }

    /// The page the whole axis was asked for.
    #[test]
    fn endmark_with_the_korean_lexicon_reads_as_the_owner_wrote_it() {
        let nodes = vec![
            Node {
                depth: 0,
                parts: vec![
                    Part::Word(Word::Foreach),
                    Part::Text("a".into()),
                    Part::Word(Word::In),
                    Part::Text("2..9".into()),
                    Part::Glued(":".into()),
                ],
            },
            Node {
                depth: 1,
                parts: vec![Part::Word(Word::Log), Part::Text("a * b".into())],
            },
        ];
        assert_eq!(
            "반복문 시작 a 범위 2..9:\n출력 a * b\n반복문 종료\n",
            Endmark.write(&nodes, &KO).unwrap()
        );
    }

    /// ⚠ The dependency, stated as a case. A raw opener cannot be
    /// closed by name, and the shape says so instead of guessing.
    #[test]
    fn endmark_refuses_a_block_whose_opener_names_no_word() {
        let nodes = vec![
            Node::raw(0, "something:".into()),
            Node {
                depth: 1,
                parts: vec![Part::Word(Word::Log)],
            },
        ];
        let refused = Endmark.write(&nodes, &EN).unwrap_err();
        assert_eq!("endmark", refused.shape);
        assert!(
            refused.why.contains("something:"),
            "the refusal names the line: {}",
            refused.why
        );
    }

    /// ⚠⚠ The law every shape and lexicon owes, walked over the
    /// REGISTRY rather than a list beside it: writing a page and
    /// normalising it gives back the canonical page, byte for byte.
    /// A pair added without satisfying it turns this red on the day it
    /// is registered.
    ///
    /// This case uses a small page; the corpus-wide one lives in
    /// `a_page_in_any_shape_normalises_to_the_canonical_one`, because
    /// eight lines cannot stand in for 691 documents.
    #[test]
    fn every_registered_pair_normalises_back_to_the_canonical_page() {
        let nodes = vec![
            Node {
                depth: 0,
                parts: vec![
                    Part::Word(Word::Machine),
                    Part::Text("m".into()),
                    Part::Text("(datamodel: ecmascript)".into()),
                ],
            },
            Node {
                depth: 1,
                parts: vec![
                    Part::Word(Word::State),
                    Part::Text("s0".into()),
                    Part::Glued(":".into()),
                ],
            },
            Node {
                depth: 2,
                parts: vec![
                    Part::Word(Word::On),
                    Part::Text("e".into()),
                    Part::Word(Word::Arrow),
                    Part::Text("t".into()),
                ],
            },
            Node {
                depth: 1,
                parts: vec![Part::Word(Word::Final), Part::Text("done".into())],
            },
        ];
        let want = canonical(&nodes);
        for shape in SHAPES {
            for lexicon in LEXICONS {
                let page = shape
                    .write(&nodes, lexicon)
                    .unwrap_or_else(|e| panic!("{} × {}: {e}", shape.name(), lexicon.name));
                let back = shape
                    .normalise(&page, lexicon)
                    .unwrap_or_else(|e| panic!("{} × {}: {e}", shape.name(), lexicon.name));
                assert_eq!(
                    want,
                    back,
                    "{} × {} does not normalise back to the canonical page.\n\
                     wrote:\n{page}",
                    shape.name(),
                    lexicon.name
                );
            }
        }
    }

    /// ⚠ Every shape in the registry is walked, not a list written
    /// here. A shape added without a name, or with one another shape
    /// already uses, is caught the day it is registered.
    #[test]
    fn every_registered_shape_has_its_own_name() {
        let mut seen: std::collections::BTreeSet<&str> = Default::default();
        for shape in SHAPES {
            let name = shape.name();
            assert!(!name.is_empty(), "a registered shape has no name");
            assert!(seen.insert(name), "two shapes are both called {name:?}");
        }
        assert!(
            seen.contains("indent"),
            "the default shape left the registry, so nothing offers it"
        );
    }

    /// A line the renderer has not been decomposed into words yet still
    /// reaches the page exactly as it did before.
    #[test]
    fn a_raw_line_is_written_unchanged() {
        let nodes = vec![Node::raw(2, "on entry:".into())];
        assert_eq!("    on entry:\n", Indent.write(&nodes, &EN).unwrap());
    }

    /// The name is how a caller asks for a lexicon, so two alike would
    /// make one of them unreachable and a page naming it ambiguous.
    #[test]
    fn every_registered_lexicon_has_its_own_name() {
        let mut seen: std::collections::BTreeSet<&str> = Default::default();
        for lexicon in LEXICONS {
            assert!(!lexicon.name.is_empty(), "a registered lexicon has no name");
            assert!(
                seen.insert(lexicon.name),
                "two lexicons are both called {:?}",
                lexicon.name
            );
        }
        assert!(
            seen.contains("en"),
            "the default lexicon left the registry, so nothing offers it"
        );
    }

    /// Every name the registries carry reaches its own entry.
    ///
    /// ⚠ Derived from the registries, not listed here: a shape added
    /// tomorrow is asked this question tomorrow.
    #[test]
    fn a_registered_name_is_how_a_caller_asks_for_it() {
        for shape in SHAPES {
            let found = shape_named(shape.name()).expect("a registered shape answers to its name");
            assert_eq!(shape.name(), found.name());
        }
        for lexicon in LEXICONS {
            let found =
                lexicon_named(lexicon.name).expect("a registered lexicon answers to its name");
            assert_eq!(lexicon.name, found.name);
        }
        assert!(shape_named("no-such-shape").is_none());
        assert!(lexicon_named("no-such-lexicon").is_none());
    }

    /// The whole point of the declaration: a page that has been filed
    /// and handed on still says how it is read.
    ///
    /// ⚠ The population is `SHAPES × LEXICONS`, so this is also the
    /// case that fails on the day a pair is registered whose
    /// declaration cannot be read back.
    #[test]
    fn a_page_says_which_pair_wrote_it() {
        let nodes = vec![
            Node {
                depth: 0,
                parts: vec![Part::Word(Word::Enum), Part::Text("colour".into())],
            },
            Node {
                depth: 1,
                parts: vec![Part::Text("red".into())],
            },
        ];
        for shape in SHAPES {
            for lexicon in LEXICONS {
                let Ok(page) = write_page(&nodes, *shape, lexicon) else {
                    continue;
                };
                let (back_shape, back_lexicon, _body) =
                    read_page(&page).expect("a page this module wrote is a page it can read");
                assert_eq!(
                    (shape.name(), lexicon.name),
                    (back_shape.name(), back_lexicon.name),
                    "the page did not hand back the pair that wrote it"
                );
                assert_eq!(
                    canonical(&nodes),
                    normalise_page(&page).expect("a declared page normalises"),
                    "{} x {} did not come back as the canonical page",
                    shape.name(),
                    lexicon.name
                );
            }
        }
    }

    /// The default page is unchanged, which is the safety net this
    /// whole extension rests on.
    #[test]
    fn the_default_pair_writes_no_declaration() {
        let nodes = vec![Node::raw(0, "enum colour".into())];
        let page = write_page(&nodes, &Indent, &EN).unwrap();
        assert_eq!(canonical(&nodes), page);
        assert!(
            !page.contains(DECLARATION),
            "the default page grew a declaration, so every golden moved"
        );
    }

    /// A declaration this build cannot obey is refused BY NAME.
    ///
    /// ⚠ The alternative is the one that has to be excluded: falling
    /// back to the default pair would read a page written elsewhere as
    /// though it were written here, and every line whose words happen
    /// to coincide would come back looking right.
    #[test]
    fn a_declaration_this_build_cannot_obey_is_refused() {
        // ⚠ `let Err(..) else` rather than `expect_err`, which would
        // want the Ok side to be `Debug` — and that side carries a
        // `&dyn Shape`, so the convenience would cost every shape a
        // derive it has no other use for.
        let Err(unknown) = read_page("#!sce-pseudo shape=indent lexicon=de\nenum colour\n") else {
            panic!("an unregistered lexicon was read as though this build had it");
        };
        assert!(unknown.why.contains("'de' lexicon"), "{unknown}");

        let Err(half) = read_page("#!sce-pseudo shape=endmark\nenum colour\n") else {
            panic!("half a declaration left the other half to a guess");
        };
        assert!(half.why.contains("which lexicon"), "{half}");

        let Err(junk) = read_page("#!sce-pseudo endmark ko\nenum colour\n") else {
            panic!("a declaration is key=value, not a pair of bare words");
        };
        assert!(junk.why.contains("key=value"), "{junk}");
    }
}
