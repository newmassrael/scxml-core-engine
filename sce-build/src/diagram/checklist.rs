// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The requirement checklist printed beside the figures: every requirement
//! of the specification's manifest, its outcome, and where the figures show
//! it.
//!
//! The figures answer "what does the machine do"; they cannot answer "what
//! did the specification ask for that the machine does not do" — a missing
//! requirement draws nothing. The manifest is the list only the
//! specification holds (Requirement-closure RFC §5.3), so a checklist row
//! for every entry of it is what makes an omission visible on paper: a
//! requirement whose row says "not shown" is one the reviewer will not
//! find in any figure.
//!
//! Nothing is re-decided here. The outcome is
//! [`crate::requirement_manifest::classify`]'s, the nodes that cite a
//! requirement are the requirement report's walk, and a node's place is
//! the figure that draws it — a state's box where the state is drawn as
//! itself, a transition's row in the table of the figure that describes
//! it. A row is flagged when the requirement rests on a value the author
//! guessed (`sce:assumed`) or marked undecided (`sce:unresolved`), the two
//! things the prototype page showed its reviewer first.

use super::boxes::{BoxError, Style};
use super::fit::{Page, Printed, Refusal};
use super::metrics::{self, Face};
use super::words::{self, Phrase};
use super::{Diagram, FigureName};
use crate::forge::page::Lexicon;
use crate::model::SCXMLModel;
use crate::provenance::MarkerKind;
use crate::requirement_manifest::{Classification, Outcome};
use crate::requirements_report::{walk_nodes, ActionSite, NodeSubject};

/// Where a requirement is shown.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Place {
    /// A state's box, in the figure that draws the state as itself.
    Box { figure: FigureName, state: String },
    /// A transition's row in a figure's table.
    Row { figure: FigureName, number: usize },
    /// The document as a whole — a top-level `<data>` or `<script>`,
    /// which no box draws.
    Document,
}

/// One requirement of the manifest.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Item {
    pub id: String,
    pub section: Option<String>,
    pub outcome: Outcome,
    /// Where the figures show it; empty when no figure does.
    pub places: Vec<Place>,
    /// A node citing it carries an `sce:assumed` marker.
    pub assumed: bool,
    /// A node citing it carries an `sce:unresolved` marker.
    pub open: bool,
}

/// One item per requirement of `classification`, in its order.
pub fn items(
    model: &SCXMLModel,
    classification: &Classification,
    diagram: &Diagram,
    printed: &[Printed],
) -> Vec<Item> {
    let nodes = walk_nodes(model);
    let home = |state: &str| match diagram.home_of(state) {
        Some(figure) => Place::Box {
            figure: figure.clone(),
            state: state.to_string(),
        },
        None => Place::Document,
    };
    let row = |source: &str, index: usize| {
        printed.iter().find_map(|p| {
            p.table
                .iter()
                .find(|r| r.source == source && r.index == index)
                .map(|r| Place::Row {
                    figure: p.laid.name.clone(),
                    number: r.number,
                })
        })
    };
    classification
        .outcomes
        .iter()
        .map(|o| {
            let mut places: Vec<Place> = Vec::new();
            let (mut assumed, mut open) = (false, false);
            for path in &o.node_paths {
                let Some(node) = nodes.iter().find(|n| &n.record.node_path == path) else {
                    continue;
                };
                for marker in node.subject.unresolved() {
                    match marker.kind {
                        MarkerKind::Assumed => assumed = true,
                        MarkerKind::Unresolved => open = true,
                    }
                }
                let place = match &node.subject {
                    NodeSubject::State(s) => home(&s.id),
                    NodeSubject::Transition { state, index, .. }
                    | NodeSubject::Action {
                        state,
                        site: ActionSite::Transition { index },
                        ..
                    } => row(&state.id, *index).unwrap_or_else(|| home(&state.id)),
                    NodeSubject::Action {
                        site: ActionSite::HistoryDefault { history },
                        ..
                    } => row(history, 0).unwrap_or_else(|| home(history)),
                    NodeSubject::Action { state, .. } | NodeSubject::Invoke { state, .. } => {
                        home(&state.id)
                    }
                    NodeSubject::Variable {
                        state: Some(state), ..
                    } => home(&state.id),
                    NodeSubject::Variable { state: None, .. }
                    | NodeSubject::GlobalScript { .. } => Place::Document,
                };
                if !places.contains(&place) {
                    places.push(place);
                }
            }
            Item {
                id: o.id.clone(),
                section: o.section.clone(),
                outcome: o.outcome,
                places,
                assumed,
                open,
            }
        })
        .collect()
}

/// One row of a printed checklist page: four cells, each one or more
/// lines, and whether the row needs the reviewer's attention.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Row {
    pub cells: [Vec<String>; 4],
    /// Not implemented, or resting on a guess or an undecided value.
    pub flagged: bool,
    /// Where the row starts, below the page's header.
    pub top: f64,
    pub height: f64,
}

/// One printed page of the checklist.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ChecklistPage {
    pub title: String,
    pub header: [String; 4],
    pub rows: Vec<Row>,
    /// Where each column starts.
    pub columns: [f64; 4],
    /// Where the first row starts.
    pub body_top: f64,
    pub style: Style,
    pub width: f64,
    pub height: f64,
}

/// The checklist, set on as many pages as it takes at the page's minimum
/// type size. Never shrunk: a row wider than the page is refused, as a
/// figure is.
pub fn pages(items: &[Item], lexicon: &Lexicon, page: Page) -> Result<Vec<ChecklistPage>, Refusal> {
    let style = Style::at(page.min_pt);
    let say = |p: Phrase| {
        words::phrase(lexicon, p).ok_or(Refusal::Box(BoxError::NoPhrases(lexicon.name)))
    };
    let title_of = |f: &FigureName| {
        words::figure_title(lexicon, f).ok_or(Refusal::Box(BoxError::NoPhrases(lexicon.name)))
    };
    let measure = |text: &str, size: f64| {
        metrics::width_pt(Face::Proportional, text, size)
            .map_err(|e| Refusal::Box(BoxError::Unmeasured(e)))
    };

    let header = [
        say(Phrase::Requirement)?.to_string(),
        say(Phrase::Section)?.to_string(),
        say(Phrase::Outcome)?.to_string(),
        say(Phrase::InFigures)?.to_string(),
    ];
    let line = style.body_pt * style.leading;
    let mut rows: Vec<Row> = Vec::new();
    for item in items {
        let mut outcome = vec![item.outcome.as_str().to_string()];
        if item.assumed {
            outcome.push(say(Phrase::Assumed)?.to_string());
        }
        if item.open {
            outcome.push(say(Phrase::Open)?.to_string());
        }
        let mut places = Vec::new();
        for place in &item.places {
            places.push(match place {
                Place::Box { figure, state } => format!("{}: {state}", title_of(figure)?),
                Place::Row { figure, number } => {
                    format!("{}: {} {number}", title_of(figure)?, say(Phrase::Row)?)
                }
                Place::Document => title_of(&FigureName::Document)?,
            });
        }
        if places.is_empty() {
            places.push(say(Phrase::NotShown)?.to_string());
        }
        let cells = [
            vec![item.id.clone()],
            vec![item.section.clone().unwrap_or_default()],
            outcome,
            places,
        ];
        let lines = cells.iter().map(Vec::len).max().unwrap_or(1);
        rows.push(Row {
            cells,
            flagged: item.outcome != Outcome::Implemented || item.assumed || item.open,
            top: 0.0,
            height: line * lines as f64,
        });
    }

    // Columns as wide as their widest cell, the header included, and
    // one and a half ems apart — two spaces of this face are only three
    // points at 7 pt, too little to tell one column from the next.
    let space = style.body_pt * 1.5;
    let mut widths = [0.0f64; 4];
    for (w, head) in widths.iter_mut().zip(&header) {
        *w = measure(head, style.body_pt)?;
    }
    for row in &rows {
        for (w, cell) in widths.iter_mut().zip(&row.cells) {
            for l in cell {
                *w = w.max(measure(l, style.body_pt)?);
            }
        }
    }
    let mut columns = [0.0f64; 4];
    for k in 1..4 {
        columns[k] = columns[k - 1] + widths[k - 1] + space;
    }
    let width = columns[3] + widths[3];
    let title = say(Phrase::Checklist)?.to_string();
    let title_height = style.title_pt * style.leading;
    let body_top = title_height + line * 2.0;
    let area = page.area_pt();
    let tallest = rows.iter().map(|r| r.height).fold(0.0, f64::max);
    if width > area.0 || body_top + tallest > area.1 {
        return Err(Refusal::ChecklistDoesNotFit {
            need_pt: (width, body_top + tallest),
            area_pt: area,
        });
    }

    // As many rows to a page as fit under its title and header. An empty
    // manifest still gets its one page, which says so by holding no row.
    let mut groups: Vec<(Vec<Row>, f64)> = vec![(Vec::new(), 0.0)];
    for mut row in rows {
        let (current, y) = groups.last_mut().expect("never empty");
        if body_top + *y + row.height > area.1 && !current.is_empty() {
            groups.push((Vec::new(), 0.0));
        }
        let (current, y) = groups.last_mut().expect("never empty");
        row.top = *y;
        *y += row.height;
        current.push(row);
    }
    Ok(groups
        .into_iter()
        .map(|(rows, y)| ChecklistPage {
            title: title.clone(),
            header: header.clone(),
            rows,
            columns,
            body_top,
            style,
            width,
            height: body_top + y,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::fit::print;
    use crate::diagram::split;
    use crate::forge::page::{EN, KO};
    use crate::parser::SCXMLParser;
    use crate::requirement_manifest::{classify, RequirementManifest};

    fn fixture() -> (SCXMLModel, RequirementManifest) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/requirement_closure");
        let model = SCXMLParser::new()
            .parse_file(
                dir.join("doip_nl_connection_states.scxml")
                    .to_str()
                    .unwrap(),
            )
            .expect("parses");
        let manifest =
            RequirementManifest::load(&dir.join("iso13400_2_nl_socket_handling.manifest.json"))
                .expect("loads");
        (model, manifest)
    }

    /// Every requirement of the manifest has a row, in its order; a
    /// `missing` one is shown nowhere, and every other place named is a
    /// box or a row the printed figures really carry.
    #[test]
    fn every_requirement_has_a_row_and_its_places_exist() {
        let (m, manifest) = fixture();
        let classification = classify(&m, &manifest);
        let page = Page::named("a3-landscape", 7.0).unwrap();
        let printed = print(&m, &EN, page).expect("fits");
        let list = items(&m, &classification, &split(&m, 1), &printed);
        let ids: Vec<&str> = list.iter().map(|i| i.id.as_str()).collect();
        let expected: Vec<&str> = classification
            .outcomes
            .iter()
            .map(|o| o.id.as_str())
            .collect();
        assert_eq!(ids, expected);
        assert!(
            list.iter().any(|i| i.outcome == Outcome::Missing),
            "the fixture has a gap"
        );
        assert!(
            list.iter().any(|i| !i.places.is_empty()),
            "and something shown"
        );
        for item in &list {
            if item.outcome == Outcome::Missing {
                assert!(item.places.is_empty(), "{item:?}");
            }
            for place in &item.places {
                match place {
                    Place::Box { figure, state } => assert!(
                        printed.iter().any(|p| &p.laid.name == figure
                            && p.laid.frame.iter().chain(&p.laid.inner).any(|b| matches!(
                                &b.sized.kind,
                                super::super::boxes::BoxKind::State(s) if s == state
                            ))),
                        "{place:?}"
                    ),
                    Place::Row { figure, number } => assert!(
                        printed.iter().any(|p| &p.laid.name == figure
                            && p.table.iter().any(|r| r.number == *number)),
                        "{place:?}"
                    ),
                    Place::Document => {}
                }
            }
        }
    }

    /// Pages hold the rows in order, none dropped or doubled, each page
    /// within the area — in both lexicons, and at a type size that forces
    /// more than one page.
    #[test]
    fn the_pages_hold_every_row_once_within_the_area() {
        let (m, manifest) = fixture();
        let classification = classify(&m, &manifest);
        let printed = print(&m, &EN, Page::named("a3-landscape", 7.0).unwrap()).expect("fits");
        let list = items(&m, &classification, &split(&m, 1), &printed);
        // Ten times the manifest cannot fit one page at this size.
        let many: Vec<Item> = (0..10).flat_map(|_| list.iter().cloned()).collect();
        for (lexicon, items) in [(&EN, &list), (&KO, &list), (&EN, &many)] {
            let page = Page::named("a4-landscape", 7.0).unwrap();
            let set = pages(items, lexicon, page).expect("fits");
            let rows: Vec<&[Vec<String>; 4]> = set
                .iter()
                .flat_map(|p| p.rows.iter().map(|r| &r.cells))
                .collect();
            assert_eq!(rows.len(), items.len());
            for (row, item) in rows.iter().zip(items.iter()) {
                assert_eq!(row[0], std::slice::from_ref(&item.id), "in order");
            }
            for p in &set {
                assert!(!p.rows.is_empty());
                assert!(p.width <= page.area_pt().0 && p.height <= page.area_pt().1);
            }
            if items.len() > list.len() {
                assert!(set.len() > 1, "a long checklist takes more pages");
            }
        }
    }
}
