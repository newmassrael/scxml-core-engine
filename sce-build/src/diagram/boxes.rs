// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What each box of a figure says, and how large that makes it.
//!
//! A box's text is the page's text: the state's name, the page's own word
//! for its kind, its `<onentry>` / `<onexit>` blocks through
//! [`crate::forge::pseudo::action_lines`], its targetless transitions. The
//! size follows from the text by [`super::metrics`], so a box is exactly as
//! large as what it has to say at the chosen point size — the input the
//! layout and the page-fit rule both rest on.

use super::metrics::{self, Face, Unmeasured};
use super::words::{self, Phrase};
use super::{Diagram, End, Figure, FigureName};
use crate::forge::page::{Lexicon, Word};
use crate::forge::pseudo::{action_lines, Unsupported};
use crate::model::SCXMLModel;

/// Type sizes and spacing, in points.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Style {
    /// A box's title line.
    pub title_pt: f64,
    /// Every other line.
    pub body_pt: f64,
    /// Line advance as a multiple of the point size.
    pub leading: f64,
    /// Space between a box's border and its text.
    pub padding: f64,
}

impl Style {
    /// A printed specification at `min_pt`: titles one step larger.
    pub fn at(min_pt: f64) -> Self {
        Style {
            title_pt: min_pt + 1.0,
            body_pt: min_pt,
            leading: 1.3,
            padding: min_pt * 0.6,
        }
    }
}

/// What a box stands for.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "state", rename_all = "kebab-case")]
pub enum BoxKind {
    /// A state drawn as itself.
    State(String),
    /// A compound drawn folded; opened in the figure its line names.
    Folded(String),
    /// A state of another figure, stood in for at this figure's edge.
    Elsewhere(String),
}

/// How a box is outlined — decided here, so no renderer re-reads the model
/// and two renderers cannot draw one state two ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outline {
    /// An atomic or compound state.
    Plain,
    /// A `<final>` state: a double border.
    Final,
    /// A `<parallel>` state: a dashed border (and so nothing else is dashed).
    Parallel,
    /// A folded compound: a plain border with a folded corner.
    Folded,
    /// A `<history>` pseudo-state: a fully rounded border.
    History,
    /// A state of another figure: grey fill, solid border (rule 4 — dashed
    /// already means parallel).
    Elsewhere,
}

/// One line of a box.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Line {
    pub text: String,
    pub face: Face,
    pub size_pt: f64,
}

/// One box, sized.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SizedBox {
    pub kind: BoxKind,
    pub outline: Outline,
    pub lines: Vec<Line>,
    pub width: f64,
    pub height: f64,
}

/// Why a figure's boxes could not be sized.
#[derive(Debug)]
pub enum BoxError {
    /// A character with no measured width.
    Unmeasured(Unmeasured),
    /// A construct the page refuses to render.
    Unsupported(Unsupported),
    /// A lexicon the figure's phrases do not cover.
    NoPhrases(&'static str),
    /// An arrow end with no box in the laid-out figure — the split and the
    /// layout disagreeing about what the figure draws.
    Unplaced(End),
    /// A state or transition source the figure names and the machine holds
    /// neither as a state nor as a `<history>` — the split and the model
    /// disagreeing.
    NotInModel(String),
}

impl std::fmt::Display for BoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoxError::Unmeasured(u) => u.fmt(f),
            BoxError::Unsupported(u) => write!(f, "{u:?}"),
            BoxError::NoPhrases(name) => write!(f, "the figure has no phrases in lexicon {name:?}"),
            BoxError::Unplaced(end) => write!(f, "the arrow end {end:?} has no box in its figure"),
            BoxError::NotInModel(id) => write!(
                f,
                "the figure names '{id}', which the machine holds neither as a state nor as a history"
            ),
        }
    }
}

/// Every box `figure` draws, in drawing order — its states, then its folded
/// compounds, then the states of other figures it reaches — each sized.
pub fn boxes(
    model: &SCXMLModel,
    diagram: &Diagram,
    figure: &Figure,
    lexicon: &Lexicon,
    style: Style,
) -> Result<Vec<SizedBox>, BoxError> {
    let phrase = |p: Phrase| words::phrase(lexicon, p).ok_or(BoxError::NoPhrases(lexicon.name));
    let title_of = |name: &FigureName| {
        words::figure_title(lexicon, name).ok_or(BoxError::NoPhrases(lexicon.name))
    };
    let title = |text: String| Line {
        text,
        face: Face::Proportional,
        size_pt: style.title_pt,
    };
    let prose = |text: String| Line {
        text,
        face: Face::Proportional,
        size_pt: style.body_pt,
    };
    let code = |text: String| Line {
        text,
        face: Face::Mono,
        size_pt: style.body_pt,
    };
    let word = |w: Word| (lexicon.word)(w);

    let mut out = Vec::new();
    for sid in &figure.states {
        // §scxml-3.10: a `<history>` is drawn where its parent is opened,
        // as a pseudo-state — its id and type. Its default transition is
        // an arrow like any other and is described in the table.
        if let Some(h) = model.history_states.get(sid) {
            let head = format!("{sid}  {} {}", word(Word::History), h.history_type);
            out.push(sized(
                BoxKind::State(sid.clone()),
                Outline::History,
                vec![title(head)],
                style,
            )?);
            continue;
        }
        let s = model
            .states
            .get(sid)
            .ok_or_else(|| BoxError::NotInModel(sid.clone()))?;
        let mut head = sid.clone();
        let mut outline = Outline::Plain;
        if s.is_final {
            head = format!("{head}  {}", word(Word::Final));
            outline = Outline::Final;
        } else if s.is_parallel {
            head = format!("{head}  {}", word(Word::Parallel));
            outline = Outline::Parallel;
        }
        let mut lines = vec![title(head)];
        for (heading, blocks) in [
            (Word::OnEntry, &s.on_entry_blocks),
            (Word::OnExit, &s.on_exit_blocks),
        ] {
            for block in blocks {
                lines.push(prose(word(heading).to_string()));
                for l in action_lines(block, lexicon).map_err(BoxError::Unsupported)? {
                    lines.push(code(format!("  {l}")));
                }
            }
        }
        // A targetless transition stays in its state, so it is described
        // in the state's box — in full, as the page writes it.
        for t in figure.in_place.iter().filter(|t| &t.source == sid) {
            for l in t.page_lines(model, lexicon)? {
                lines.push(code(l));
            }
        }
        out.push(sized(BoxKind::State(sid.clone()), outline, lines, style)?);
    }
    for f in &figure.folded {
        let opens = format!(
            "{} {}",
            phrase(Phrase::OpensIn)?,
            title_of(&FigureName::Inside(f.clone()))?
        );
        out.push(sized(
            BoxKind::Folded(f.clone()),
            Outline::Folded,
            vec![title(f.clone()), prose(opens)],
            style,
        )?);
    }
    for e in &figure.elsewhere {
        let mut lines = vec![prose(e.clone())];
        if let Some(home) = diagram.home_of(e) {
            lines.push(prose(format!("({})", title_of(home)?)));
        }
        out.push(sized(
            BoxKind::Elsewhere(e.clone()),
            Outline::Elsewhere,
            lines,
            style,
        )?);
    }
    Ok(out)
}

fn sized(
    kind: BoxKind,
    outline: Outline,
    lines: Vec<Line>,
    style: Style,
) -> Result<SizedBox, BoxError> {
    let mut width: f64 = 0.0;
    let mut height = 0.0;
    for l in &lines {
        width =
            width.max(metrics::width_pt(l.face, &l.text, l.size_pt).map_err(BoxError::Unmeasured)?);
        height += l.size_pt * style.leading;
    }
    Ok(SizedBox {
        kind,
        outline,
        lines,
        width: width + 2.0 * style.padding,
        height: height + 2.0 * style.padding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::split;
    use crate::forge::page::{EN, KO};
    use crate::parser::SCXMLParser;

    const DOC: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
        initial="released" datamodel="ecmascript">
  <state id="released" initial="unlocked">
    <state id="unlocked">
      <onentry><send event="indicator.update" target="#_parent"/></onentry>
      <transition event="tick"/>
      <transition event="lock.request" target="locked"/>
    </state>
    <state id="relocking" initial="waiting">
      <state id="waiting"><transition event="t" target="armed"/></state>
      <state id="armed"><transition event="x" target="locked"/></state>
    </state>
    <transition event="speed.high" target="relocking"/>
  </state>
  <final id="locked"/>
</scxml>"##;

    fn model() -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(DOC, "boxes")
            .expect("parses")
    }

    /// Every box a figure draws is there, once, sized from its own text —
    /// and the page's words are what that text is made of.
    #[test]
    fn a_figure_draws_a_sized_box_for_everything_it_shows() {
        let m = model();
        let d = split(&m, 1);
        let released = &d.figures[1];
        let b = boxes(&m, &d, released, &EN, Style::at(7.0)).expect("sized");
        let kinds: Vec<&BoxKind> = b.iter().map(|b| &b.kind).collect();
        assert_eq!(
            kinds,
            [
                &BoxKind::State("released".into()),
                &BoxKind::State("unlocked".into()),
                &BoxKind::Folded("relocking".into()),
                &BoxKind::Elsewhere("locked".into()),
            ]
        );
        let unlocked = &b[1];
        let text: Vec<&str> = unlocked.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(text[0], "unlocked");
        assert_eq!(text[1], "on entry", "the page's word: {text:?}");
        assert!(
            text.iter().any(|t| t.contains("indicator.update")),
            "{text:?}"
        );
        let tick = crate::diagram::TransitionRef {
            source: "unlocked".into(),
            index: 0,
            target: None,
        }
        .page_lines(&m, &EN)
        .expect("renders");
        assert!(
            tick.iter().all(|l| text.contains(&l.as_str())) && tick[0].contains("tick"),
            "the targetless transition, as the page writes it: {text:?}"
        );
        for b in &b {
            assert!(b.width > 0.0 && b.height > 0.0, "{b:?}");
        }
    }

    /// Size follows the text: the same figure at a larger point size is
    /// larger in both directions, and the Korean page words size too.
    #[test]
    fn size_follows_the_text_and_the_type_size() {
        let m = model();
        let d = split(&m, 1);
        let fig = &d.figures[1];
        let small = boxes(&m, &d, fig, &EN, Style::at(7.0)).expect("sized");
        let large = boxes(&m, &d, fig, &EN, Style::at(10.0)).expect("sized");
        for (s, l) in small.iter().zip(&large) {
            assert!(l.width > s.width && l.height > s.height, "{s:?} vs {l:?}");
        }
        boxes(&m, &d, fig, &KO, Style::at(7.0)).expect("the Korean page words are measured");
    }
}
