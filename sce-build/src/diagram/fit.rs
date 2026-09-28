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

impl Page {
    /// ISO A4, portrait, 15 mm margins.
    pub fn a4_portrait(min_pt: f64) -> Self {
        Page {
            width_mm: 210.0,
            height_mm: 297.0,
            margin_mm: 15.0,
            min_pt,
        }
    }

    /// The printable area, in points.
    pub fn area_pt(&self) -> (f64, f64) {
        (
            (self.width_mm - 2.0 * self.margin_mm) * PT_PER_MM,
            (self.height_mm - 2.0 * self.margin_mm) * PT_PER_MM,
        )
    }
}

/// One row of a figure's transition table.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Row {
    /// The number the arrow carries, 1-based within the figure.
    pub number: usize,
    pub transition: TransitionRef,
    /// `source -> target`, `event [cond]`.
    pub cells: [String; 2],
}

/// A figure ready to print: laid out, with its table and its total size.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Printed {
    pub laid: Laid,
    pub title: String,
    pub table: Vec<Row>,
    pub table_height: f64,
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
        let laid = layout::lay_out(model, figure, sized, style);
        let title = words::figure_title(lexicon, &figure.name)
            .ok_or(Refusal::Box(BoxError::NoPhrases(lexicon.name)))?;

        let mut table = Vec::new();
        for arrow in &figure.arrows {
            for (t, at) in arrow.transitions.iter().zip(&arrow.described_in) {
                if at != &figure.name {
                    continue;
                }
                let tr = &model.states[&t.source].transitions[t.index];
                let target = t.target.clone().unwrap_or_default();
                let mut what = if tr.event.is_empty() {
                    "*".to_string()
                } else {
                    tr.event.clone()
                };
                if !tr.cond.is_empty() {
                    what = format!("{what} [{}]", tr.cond);
                }
                table.push(Row {
                    number: table.len() + 1,
                    transition: t.clone(),
                    cells: [format!("{} -> {target}", t.source), what],
                });
            }
        }

        // The table is set in the body face; each row is one line whose width
        // is the number column plus both cells.
        let line = style.body_pt * style.leading;
        let mut table_width: f64 = 0.0;
        for row in &table {
            let text = format!("{}  {}  {}", row.number, row.cells[0], row.cells[1]);
            table_width = table_width.max(
                metrics::width_pt(Face::Mono, &text, style.body_pt)
                    .map_err(|e| Refusal::Box(BoxError::Unmeasured(e)))?,
            );
        }
        let table_height = line * table.len() as f64;
        let title_height = style.title_pt * style.leading;
        let title_width = metrics::width_pt(Face::Proportional, &title, style.title_pt)
            .map_err(|e| Refusal::Box(BoxError::Unmeasured(e)))?;
        let width = laid.width.max(table_width).max(title_width);
        let height = title_height + laid.height + line + table_height;
        if width > area.0 || height > area.1 {
            return Err(Refusal::DoesNotFit {
                figure: figure.name.clone(),
                need_pt: (width, height),
                area_pt: area,
            });
        }
        out.push(Printed {
            laid,
            title,
            table,
            table_height,
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
    /// every transition that has a target is one table row in exactly one
    /// figure — the numbered form of "described once".
    #[test]
    fn a_small_machine_fits_and_each_transition_has_one_row() {
        let m = parse(LOCK);
        let page = Page::a4_portrait(7.0);
        let printed = print(&m, &EN, page).expect("fits");
        let area = page.area_pt();
        let mut rows: Vec<&TransitionRef> = Vec::new();
        for p in &printed {
            assert!(p.width <= area.0 && p.height <= area.1, "{p:?}");
            for (i, r) in p.table.iter().enumerate() {
                assert_eq!(r.number, i + 1, "numbers run 1.. within a figure");
                rows.push(&r.transition);
            }
        }
        let with_target: usize = m
            .states
            .values()
            .map(|s| s.transitions.iter().map(|t| t.targets.len()).sum::<usize>())
            .sum();
        rows.sort();
        rows.dedup();
        assert_eq!(rows.len(), with_target, "one row per transition-target");
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
