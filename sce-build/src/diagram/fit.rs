// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Whether each figure fits the page it is printed on, at the smallest type
//! the reader can read — and the table of transitions under it.
//!
//! The prototype this replaces measured it the hard way: on A4 portrait only
//! two of its five figures stayed at 7 pt or larger, because the renderer
//! shrank a figure until it fitted. So the page and the minimum type size
//! are INPUTS here, and a figure that does not fit at that size is refused
//! with its measured size — never shrunk below what the reader was promised.
//! Figures are already flat (one container and its direct children), so
//! there is no deeper fold to try: the refusal names the container whose
//! children do not fit, which is where an author would split the machine.
//!
//! Each transition a figure describes gets a number on its arrow and a row
//! in the table under the figure, which spends height rather than the width
//! a label on the arrow would (the prototype's rule 5).

use super::boxes::{self, BoxError, Style};
use super::layout::{self, Laid};
use super::metrics::{self, Face};
use super::route::{self, Marker, Point, Routed};
use super::words;
use super::{split, FigureName, TransitionRef};
use crate::forge::page::Lexicon;
use crate::model::SCXMLModel;

/// Points per millimetre.
const PT_PER_MM: f64 = 72.0 / 25.4;

/// A printed page, and the smallest type allowed on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Page {
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_mm: f64,
    pub min_pt: f64,
}

/// The page sizes a caller may name, as (name, width mm, height mm). One
/// list, so the CLI's choices and any other caller's are the same.
pub const PAGES: &[(&str, f64, f64)] = &[
    ("a4-portrait", 210.0, 297.0),
    ("a4-landscape", 297.0, 210.0),
    ("a3-portrait", 297.0, 420.0),
    ("a3-landscape", 420.0, 297.0),
    ("letter-portrait", 215.9, 279.4),
    ("letter-landscape", 279.4, 215.9),
];

/// The margin every named page keeps on each side.
const MARGIN_MM: f64 = 15.0;

impl Page {
    /// ISO A4, portrait, 15 mm margins.
    pub fn a4_portrait(min_pt: f64) -> Self {
        Self::named("a4-portrait", min_pt).expect("a4-portrait is in PAGES")
    }

    /// The page `PAGES` calls `name`, or `None` for a name it does not
    /// carry.
    pub fn named(name: &str, min_pt: f64) -> Option<Self> {
        PAGES
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|&(_, width_mm, height_mm)| Page {
                width_mm,
                height_mm,
                margin_mm: MARGIN_MM,
                min_pt,
            })
    }

    /// The printable area, in points.
    pub fn area_pt(&self) -> (f64, f64) {
        (
            (self.width_mm - 2.0 * self.margin_mm) * PT_PER_MM,
            (self.height_mm - 2.0 * self.margin_mm) * PT_PER_MM,
        )
    }
}

/// One row of a figure's transition table: one transition, however many
/// targets it has — each of its arrows carries this row's number, so the
/// transition is described once.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Row {
    /// The number its arrows carry, 1-based within the figure.
    pub number: usize,
    /// The state (or history) the transition belongs to, and its position
    /// there — the same two fields as [`TransitionRef`].
    pub source: String,
    pub index: usize,
    /// The transition as the statechart page writes it: its head, then its
    /// traceability and actions (see [`TransitionRef::page_lines`]).
    pub lines: Vec<String>,
    /// Where the row starts, below `Printed::table_top`.
    pub top: f64,
}

impl Row {
    /// Whether `t` is one of this row's arrows.
    pub fn describes(&self, t: &TransitionRef) -> bool {
        self.source == t.source && self.index == t.index
    }
}

/// A figure ready to print: laid out and routed, with its table and its
/// total size. Everything a renderer needs is here, placed; a renderer
/// decides nothing (see [`super::svg`]).
///
/// The printed figure's own coordinates run from its top-left corner: the
/// title, then the drawing — whose laid-out coordinates are moved by
/// `drawing_at` — then the table from `table_top`.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Printed {
    pub laid: Laid,
    pub arrows: Vec<Routed>,
    pub marker: Option<Marker>,
    pub title: String,
    pub drawing_at: Point,
    pub table: Vec<Row>,
    pub table_top: f64,
    /// Where each of the table's three columns starts: number, source,
    /// the transition's lines.
    pub columns: [f64; 3],
    pub style: Style,
    pub width: f64,
    pub height: f64,
}

/// Why a document could not be printed.
#[derive(Debug)]
pub enum Refusal {
    Box(BoxError),
    /// A figure larger than the page at the minimum type size.
    DoesNotFit {
        figure: FigureName,
        need_pt: (f64, f64),
        area_pt: (f64, f64),
    },
    /// A requirement checklist row wider or taller than the page at the
    /// minimum type size (see [`super::checklist::pages`]).
    ChecklistDoesNotFit {
        need_pt: (f64, f64),
        area_pt: (f64, f64),
    },
    /// A sheet of a non-statechart kind — a table whose columns or one row,
    /// or a picture, do not fit the page at the minimum type size (see
    /// [`super::table::set`], [`super::canvas::Canvas::finish`]). `what` is
    /// the sheet's own title.
    SheetDoesNotFit {
        what: String,
        need_pt: (f64, f64),
        area_pt: (f64, f64),
    },
    /// A document that cannot be read as a tree of named values — the
    /// model carrying a shape the field table has no reading for.
    NotATree(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Box(e) => e.fmt(f),
            Refusal::DoesNotFit {
                figure,
                need_pt,
                area_pt,
            } => write!(
                f,
                "the figure {figure:?} needs {:.0} x {:.0} pt at the minimum type size, \
                 and the page gives {:.0} x {:.0} pt; it is not shrunk below that size — \
                 split the container's children across more states",
                need_pt.0, need_pt.1, area_pt.0, area_pt.1
            ),
            Refusal::ChecklistDoesNotFit { need_pt, area_pt } => write!(
                f,
                "a requirement checklist row needs {:.0} x {:.0} pt at the minimum type size, \
                 and the page gives {:.0} x {:.0} pt; it is not shrunk below that size",
                need_pt.0, need_pt.1, area_pt.0, area_pt.1
            ),
            Refusal::SheetDoesNotFit {
                what,
                need_pt,
                area_pt,
            } => write!(
                f,
                "{what:?} needs {:.0} x {:.0} pt at the minimum type size, \
                 and the page gives {:.0} x {:.0} pt; it is not shrunk below that size",
                need_pt.0, need_pt.1, area_pt.0, area_pt.1
            ),
            Refusal::NotATree(detail) => write!(
                f,
                "the document cannot be read as a tree of fields: {detail}"
            ),
        }
    }
}

/// Every figure of `model`, laid out, tabled and checked against `page`.
pub fn print(model: &SCXMLModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Printed>, Refusal> {
    let style = Style::at(page.min_pt);
    let diagram = split(model, 1);
    let area = page.area_pt();
    let mut out = Vec::new();
    for figure in &diagram.figures {
        let sized = boxes::boxes(model, &diagram, figure, lexicon, style).map_err(Refusal::Box)?;
        let laid = layout::lay_out(model, figure, sized, style).map_err(Refusal::Box)?;
        let title = words::figure_title(lexicon, &figure.name)
            .ok_or(Refusal::Box(BoxError::NoPhrases(lexicon.name)))?;

        let line = style.body_pt * style.leading;
        let mut table: Vec<Row> = Vec::new();
        let mut rows_height = 0.0;
        for arrow in &figure.arrows {
            for (t, at) in arrow.transitions.iter().zip(&arrow.described_in) {
                if at != &figure.name || table.iter().any(|r| r.describes(t)) {
                    continue;
                }
                let lines = t.page_lines(model, lexicon).map_err(Refusal::Box)?;
                let height = line * lines.len().max(1) as f64;
                table.push(Row {
                    number: table.len() + 1,
                    source: t.source.clone(),
                    index: t.index,
                    lines,
                    top: rows_height,
                });
                rows_height += height;
            }
        }

        let (arrows, marker) =
            route::route(figure, &laid, &table, lexicon, style).map_err(Refusal::Box)?;

        // The drawing's extent: its boxes, and whatever its arrows, their
        // labels and the initial marker reach beyond them — a label is part
        // of the figure the page has to hold.
        let (mut x0, mut y0, mut x1, mut y1) = (0.0f64, 0.0f64, laid.width, laid.height);
        let mut reach = |(x, y): Point| {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        };
        for a in &arrows {
            a.path.iter().copied().for_each(&mut reach);
            reach(a.label_at);
            reach((a.label_at.0 + a.label_size.0, a.label_at.1 + a.label_size.1));
        }
        if let Some(m) = &marker {
            reach((m.dot.0 - m.radius, m.dot.1 - m.radius));
        }

        // The table is set in the mono face: the number, the source, and
        // the transition's page lines one under another, each column as
        // wide as its widest cell and two spaces from the next.
        let mono = |text: &str| {
            metrics::width_pt(Face::Mono, text, style.body_pt)
                .map_err(|e| Refusal::Box(BoxError::Unmeasured(e)))
        };
        let space = mono("  ")?;
        let mut widths = [0.0f64; 3];
        for row in &table {
            widths[0] = widths[0].max(mono(&row.number.to_string())?);
            widths[1] = widths[1].max(mono(&row.source)?);
            for l in &row.lines {
                widths[2] = widths[2].max(mono(l)?);
            }
        }
        let columns = [0.0, widths[0] + space, widths[0] + widths[1] + 2.0 * space];
        let table_width = if table.is_empty() {
            0.0
        } else {
            columns[2] + widths[2]
        };

        let title_height = style.title_pt * style.leading;
        let title_width = metrics::width_pt(Face::Proportional, &title, style.title_pt)
            .map_err(|e| Refusal::Box(BoxError::Unmeasured(e)))?;
        let drawing_at = (-x0, title_height + line - y0);
        let table_top = drawing_at.1 + y1 + line;
        let width = (x1 - x0).max(table_width).max(title_width);
        let height = table_top + rows_height;
        if width > area.0 || height > area.1 {
            return Err(Refusal::DoesNotFit {
                figure: figure.name.clone(),
                need_pt: (width, height),
                area_pt: area,
            });
        }
        out.push(Printed {
            laid,
            arrows,
            marker,
            title,
            drawing_at,
            table,
            table_top,
            columns,
            style,
            width,
            height,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::page::EN;
    use crate::parser::SCXMLParser;

    fn parse(body: &str) -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(body, "fit")
            .expect("parses")
    }

    const LOCK: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="released">
  <state id="released" initial="unlocked">
    <history id="where" type="shallow"><transition target="unlocked"/></history>
    <state id="unlocked"><transition event="lock.request" target="locked"/></state>
    <state id="relocking" initial="waiting">
      <state id="waiting"><transition event="timer" target="armed"/></state>
      <state id="armed"><transition event="expire" target="locked"/></state>
    </state>
    <transition event="speed.high" target="relocking"/>
  </state>
  <state id="locked"><transition event="unlock.request" target="unlocked"/></state>
</scxml>"##;

    /// A small machine fits A4 at 7 pt, every figure inside the area, and
    /// every transition that has a target — a history's default included —
    /// is one table row in exactly one figure, written as the page writes
    /// it: the numbered form of "described once".
    #[test]
    fn a_small_machine_fits_and_each_transition_has_one_row() {
        let m = parse(LOCK);
        let page = Page::a4_portrait(7.0);
        let printed = print(&m, &EN, page).expect("fits");
        let area = page.area_pt();
        let mut rows: Vec<(String, usize)> = Vec::new();
        for p in &printed {
            assert!(p.width <= area.0 && p.height <= area.1, "{p:?}");
            for (i, r) in p.table.iter().enumerate() {
                assert_eq!(r.number, i + 1, "numbers run 1.. within a figure");
                let t = TransitionRef {
                    source: r.source.clone(),
                    index: r.index,
                    target: None,
                };
                assert_eq!(r.lines, t.page_lines(&m, &EN).expect("renders"));
                rows.push((r.source.clone(), r.index));
            }
        }
        let expected: Vec<(String, usize)> = crate::diagram::transitions(&m)
            .into_iter()
            .filter(|(_, _, targets)| !targets.is_empty())
            .map(|(s, i, _)| (s.to_string(), i))
            .collect();
        let (mut got, mut want) = (rows.clone(), expected.clone());
        got.sort();
        want.sort();
        assert_eq!(got, want, "one row per transition, none twice");
        assert!(
            want.iter().any(|(s, _)| s == "where"),
            "the history's default is a transition too: {want:?}"
        );
    }

    /// A container with more children than a row of A4 holds, at a type
    /// size too large to share one, is refused with its measured size —
    /// not shrunk.
    #[test]
    fn a_figure_too_large_for_the_page_is_refused_not_shrunk() {
        let mut body = String::from(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">"#,
        );
        for i in 0..40 {
            body.push_str(&format!(
                r#"<state id="a_state_with_a_long_descriptive_name_{i}"><transition event="go" target="a_state_with_a_long_descriptive_name_{}"/></state>"#,
                (i + 1) % 40
            ));
        }
        body = body.replace(
            r#"initial="s0""#,
            r#"initial="a_state_with_a_long_descriptive_name_0""#,
        );
        body.push_str("</scxml>");
        let m = parse(&body);
        match print(&m, &EN, Page::a4_portrait(14.0)) {
            Err(Refusal::DoesNotFit {
                figure,
                need_pt,
                area_pt,
            }) => {
                assert_eq!(figure, FigureName::Document);
                assert!(
                    need_pt.0 > area_pt.0 || need_pt.1 > area_pt.1,
                    "{need_pt:?} {area_pt:?}"
                );
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
