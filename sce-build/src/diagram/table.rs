// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The table typesetter: tables of cells set into [`Sheet`]s at the page's
//! minimum type size, and never shrunk below it.
//!
//! Every table a figure of a non-statechart kind needs — a lookup's
//! entries, a codec's fields, the field-by-field reading of any document —
//! is set here, so a column is as wide as its widest cell, a cell that
//! does not fit its column wraps rather than shrinks, and a table that
//! cannot be set in the page's width at all says so, with the measured
//! size, instead of being drawn smaller than the reader was promised.
//!
//! # How a table is set
//!
//! - A column is as wide as its widest line, up to the width the page
//!   leaves. When the columns together are wider than the page, the
//!   widest are narrowed to one common width (the narrow ones keep their
//!   natural width) and their cells wrap — so a short column never wastes
//!   its place on a page that a long one needs.
//! - No column is narrowed below [`MIN_COLUMN_EM`] ems. When the minimum
//!   widths together still do not fit, the table is NOT set; the caller's
//!   `fallback` tables are set in its place, or the page is refused.
//! - A heading and the table's head stay with the table's first row. A
//!   table longer than the page continues on the next one under its
//!   heading again, marked as continued, with its head repeated.
//!
//! Nothing here reads time, randomness or the machine: a width is a sum of
//! the font table's advances ([`super::metrics`]), so the same table is the
//! same marks on every machine.

use super::boxes::{BoxError, Line, Style};
use super::fit::{Page, Refusal};
use super::metrics::{self, Face};
use super::sheet::{Ink, Mark, Sheet};

/// The gap between two columns, in ems — one and a half, since two spaces
/// of this face are only three points at 7 pt, too little to tell one
/// column from the next.
const GAP_EM: f64 = 1.5;

/// The narrowest a column is set, in ems: ten Latin characters of the
/// monospaced face, or six Hangul syllables.
pub const MIN_COLUMN_EM: f64 = 6.0;

/// Where a line may break besides at a space: after these, so an
/// identifier wraps at its seams and not in the middle of a word.
const BREAK_AFTER: [char; 4] = [' ', '_', ',', '/'];

/// A tab is set as this many spaces: the font table measures no tab, and a
/// script's indentation is a layout, not a character a reader looks at.
const TAB_SPACES: &str = "    ";

/// The slack a width comparison allows, in points.
const EPSILON: f64 = 1e-9;

/// One cell: the text, which may hold line breaks, and the face it is set
/// in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub text: String,
    pub face: Face,
}

impl Cell {
    /// Text of the document — a name, an expression, a value — set so its
    /// columns line up.
    pub fn mono(text: impl Into<String>) -> Self {
        Cell {
            text: text.into(),
            face: Face::Mono,
        }
    }

    /// A word the sheet itself says.
    pub fn prose(text: impl Into<String>) -> Self {
        Cell {
            text: text.into(),
            face: Face::Proportional,
        }
    }
}

/// A table: a heading, a head row, and rows of as many cells as the head
/// has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub heading: Cell,
    pub head: Vec<Cell>,
    pub rows: Vec<Vec<Cell>>,
    /// What to set in this table's place when its columns do not fit the
    /// page at their minimum width. Empty when nothing says the same thing
    /// in another shape; a tight table is then refused.
    pub fallback: Vec<Table>,
}

/// The text of a cell as hard lines: `\r\n` read as `\n`, a tab as spaces.
fn hard_lines(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .replace('\t', TAB_SPACES)
        .split('\n')
        .map(str::to_string)
        .collect()
}

fn advance_th(face: Face, c: char, text: &str) -> Result<u64, Refusal> {
    metrics::advance(face, c).map(u64::from).ok_or_else(|| {
        Refusal::Box(BoxError::Unmeasured(metrics::Unmeasured {
            character: c,
            text: text.to_string(),
        }))
    })
}

fn thousandths(face: Face, text: &str) -> Result<u64, Refusal> {
    text.chars().map(|c| advance_th(face, c, text)).sum()
}

fn measure(face: Face, text: &str, size_pt: f64) -> Result<f64, Refusal> {
    Ok(thousandths(face, text)? as f64 * size_pt / 1000.0)
}

/// The widest hard line of `text`, in points.
fn natural_width(cell: &Cell, size_pt: f64) -> Result<f64, Refusal> {
    let mut widest = 0.0f64;
    for line in hard_lines(&cell.text) {
        widest = widest.max(measure(cell.face, &line, size_pt)?);
    }
    Ok(widest)
}

/// `text` as lines no wider than `width_pt` when set in `face` at
/// `size_pt`. A hard line break stays one; a line too long breaks after
/// its last space (or other seam) that fits, and inside a word only when
/// the word alone is wider than the column. Indentation is kept; the
/// spaces a break leaves at a line's end are dropped.
pub fn wrap(face: Face, text: &str, size_pt: f64, width_pt: f64) -> Result<Vec<String>, Refusal> {
    let over = |th: u64| th as f64 * size_pt / 1000.0 > width_pt + EPSILON;
    let mut out = Vec::new();
    for hard in hard_lines(text) {
        let mut current = String::new();
        let mut current_th: u64 = 0;
        // Byte offset just after the last place `current` may break.
        let mut seam: Option<usize> = None;
        for c in hard.chars() {
            let a = advance_th(face, c, &hard)?;
            if !current.is_empty() && over(current_th + a) {
                match seam.take() {
                    Some(i) if i < current.len() => {
                        let rest = current.split_off(i);
                        out.push(current.trim_end().to_string());
                        current = rest;
                        current_th = thousandths(face, &current)?;
                    }
                    _ => {
                        out.push(current.trim_end().to_string());
                        current.clear();
                        current_th = 0;
                    }
                }
            }
            current.push(c);
            current_th += a;
            if BREAK_AFTER.contains(&c) {
                seam = Some(current.len());
            }
        }
        out.push(current);
    }
    Ok(out)
}

/// Column widths: the natural widths when they fit `budget`; otherwise the
/// widest columns cut to one common width, none below its minimum. The
/// minimums must fit `budget` already.
fn solve(natural: &[f64], minimum: &[f64], budget: f64) -> Vec<f64> {
    if natural.iter().sum::<f64>() <= budget {
        return natural.to_vec();
    }
    let at = |cap: f64| -> Vec<f64> {
        natural
            .iter()
            .zip(minimum)
            .map(|(n, m)| n.min(cap).max(*m))
            .collect()
    };
    let (mut low, mut high) = (0.0f64, natural.iter().copied().fold(0.0, f64::max));
    for _ in 0..64 {
        let middle = (low + high) / 2.0;
        if at(middle).iter().sum::<f64>() <= budget {
            low = middle;
        } else {
            high = middle;
        }
    }
    at(low)
}

/// A table whose lines are broken for the widths it is set at.
struct Laid<'t> {
    table: &'t Table,
    heading: Vec<String>,
    heading_continued: Vec<String>,
    head: Vec<Vec<String>>,
    rows: Vec<Vec<Vec<String>>>,
    xs: Vec<f64>,
    width: f64,
}

/// Whether a table could be set in the width, or how much it needs at
/// least.
enum Fit<'t> {
    Set(Laid<'t>),
    Tight { need: f64 },
}

impl Laid<'_> {
    fn lines_of(cells: &[Vec<String>]) -> usize {
        cells.iter().map(Vec::len).max().unwrap_or(1)
    }
}

fn lay<'t>(
    table: &'t Table,
    continued: &str,
    style: Style,
    avail: f64,
) -> Result<Fit<'t>, Refusal> {
    let size = style.body_pt;
    let columns = table.head.len();
    let gaps = columns.saturating_sub(1) as f64 * GAP_EM * size;

    let mut natural = vec![0.0f64; columns];
    for row in std::iter::once(&table.head).chain(&table.rows) {
        debug_assert_eq!(row.len(), columns, "a row has a cell per column");
        for (w, cell) in natural.iter_mut().zip(row) {
            *w = w.max(natural_width(cell, size)?);
        }
    }
    let minimum: Vec<f64> = natural
        .iter()
        .map(|n| n.min(MIN_COLUMN_EM * size))
        .collect();
    let needed = minimum.iter().sum::<f64>() + gaps;
    if needed > avail + EPSILON {
        return Ok(Fit::Tight { need: needed });
    }
    let widths = solve(&natural, &minimum, avail - gaps);

    let wrap_row = |row: &[Cell]| -> Result<Vec<Vec<String>>, Refusal> {
        row.iter()
            .zip(&widths)
            .map(|(cell, w)| wrap(cell.face, &cell.text, size, *w))
            .collect()
    };
    let head = wrap_row(&table.head)?;
    let rows = table
        .rows
        .iter()
        .map(|r| wrap_row(r))
        .collect::<Result<Vec<_>, _>>()?;
    let mut xs = Vec::with_capacity(columns);
    let mut x = 0.0;
    for w in &widths {
        xs.push(x);
        x += w + GAP_EM * size;
    }
    let width = widths.iter().sum::<f64>() + gaps;
    let heading = wrap(table.heading.face, &table.heading.text, size, avail)?;
    let heading_continued = wrap(
        table.heading.face,
        &format!("{} ({continued})", table.heading.text),
        size,
        avail,
    )?;
    Ok(Fit::Set(Laid {
        table,
        heading,
        heading_continued,
        head,
        rows,
        xs,
        width,
    }))
}

/// The page being filled: its marks so far and the next free height.
struct Flow {
    marks: Vec<Mark>,
    y: f64,
}

/// `tables` set under `title`, on as many sheets as it takes.
///
/// `continued` is the word a heading carries on the page after the one it
/// started on. Refused whole — nothing returned — when a table whose
/// columns cannot fit the page has no fallback, or a single row is taller
/// than the page: a sheet missing a table reads like a complete one.
pub fn set(
    title: &str,
    continued: &str,
    tables: &[Table],
    page: Page,
) -> Result<Vec<Sheet>, Refusal> {
    let style = Style::at(page.min_pt);
    let (avail_w, avail_h) = page.area_pt();
    let line = style.body_pt * style.leading;
    let too_wide = |need: f64| Refusal::SheetDoesNotFit {
        what: title.to_string(),
        need_pt: (need, line),
        area_pt: (avail_w, avail_h),
    };

    let mut laid: Vec<Laid> = Vec::new();
    for table in tables {
        match lay(table, continued, style, avail_w)? {
            Fit::Set(l) => laid.push(l),
            Fit::Tight { need } => {
                if table.fallback.is_empty() {
                    return Err(too_wide(need));
                }
                for alternative in &table.fallback {
                    match lay(alternative, continued, style, avail_w)? {
                        Fit::Set(l) => laid.push(l),
                        Fit::Tight { need } => return Err(too_wide(need)),
                    }
                }
            }
        }
    }

    let title_width = measure(Face::Proportional, title, style.title_pt)?;
    let width = laid.iter().map(|l| l.width).fold(title_width, f64::max);
    let body_top = style.title_pt * style.leading + line;
    let title_marks = |marks: &mut Vec<Mark>| {
        marks.push(Mark::Text {
            x: 0.0,
            top: 0.0,
            line: Line {
                text: title.to_string(),
                face: Face::Proportional,
                size_pt: style.title_pt,
            },
        });
    };

    let text_at = |marks: &mut Vec<Mark>, x: f64, y: f64, lines: &[String], face: Face| {
        for (i, text) in lines.iter().enumerate() {
            marks.push(Mark::Text {
                x,
                top: y + i as f64 * line,
                line: Line {
                    text: text.clone(),
                    face,
                    size_pt: style.body_pt,
                },
            });
        }
    };
    let put_head = |flow: &mut Flow, l: &Laid, again: bool| {
        let heading = if again {
            &l.heading_continued
        } else {
            &l.heading
        };
        let height = heading.len() as f64 * line;
        flow.marks.push(Mark::Rect {
            x: 0.0,
            y: flow.y,
            width,
            height,
            fill: Some(Ink::Shade),
            outline: None,
        });
        text_at(&mut flow.marks, 0.0, flow.y, heading, l.table.heading.face);
        flow.y += height;
        let head_height = Laid::lines_of(&l.head) as f64 * line;
        for ((x, lines), cell) in l.xs.iter().zip(&l.head).zip(&l.table.head) {
            text_at(&mut flow.marks, *x, flow.y, lines, cell.face);
        }
        flow.y += head_height;
        flow.marks.push(Mark::Stroke {
            from: (0.0, flow.y),
            to: (width, flow.y),
            ink: Ink::Black,
            width_pt: style.body_pt * 0.1,
            dashed: false,
        });
    };
    let put_row = |flow: &mut Flow, l: &Laid, r: usize| {
        let height = Laid::lines_of(&l.rows[r]) as f64 * line;
        for ((x, lines), cell) in l.xs.iter().zip(&l.rows[r]).zip(&l.table.rows[r]) {
            text_at(&mut flow.marks, *x, flow.y, lines, cell.face);
        }
        flow.y += height;
        // A hairline under each row, so a row of several lines reads as
        // one record and not as the start of the next.
        flow.marks.push(Mark::Stroke {
            from: (0.0, flow.y),
            to: (width, flow.y),
            ink: Ink::Hairline,
            width_pt: style.body_pt * 0.05,
            dashed: false,
        });
    };
    let new_flow = || {
        let mut marks = Vec::new();
        title_marks(&mut marks);
        Flow { marks, y: body_top }
    };

    let mut flows = vec![new_flow()];
    for l in &laid {
        let head_height = Laid::lines_of(&l.head) as f64 * line;
        let heading_height = l.heading.len().max(l.heading_continued.len()) as f64 * line;
        let row_height = |r: usize| Laid::lines_of(&l.rows[r]) as f64 * line;
        let tallest = (0..l.rows.len()).map(row_height).fold(0.0, f64::max);
        let alone = body_top + heading_height + head_height + tallest;
        if alone > avail_h + EPSILON {
            return Err(Refusal::SheetDoesNotFit {
                what: title.to_string(),
                need_pt: (l.width, alone),
                area_pt: (avail_w, avail_h),
            });
        }
        let first = if l.rows.is_empty() {
            0.0
        } else {
            row_height(0)
        };
        let flow = flows.last_mut().expect("never empty");
        let gap = if flow.y > body_top { line } else { 0.0 };
        let together = gap + l.heading.len() as f64 * line + head_height + first;
        if flow.y > body_top && flow.y + together > avail_h + EPSILON {
            flows.push(new_flow());
        } else {
            flow.y += gap;
        }
        put_head(flows.last_mut().expect("never empty"), l, false);
        for r in 0..l.rows.len() {
            let flow = flows.last_mut().expect("never empty");
            if flow.y + row_height(r) > avail_h + EPSILON {
                flows.push(new_flow());
                put_head(flows.last_mut().expect("never empty"), l, true);
            }
            put_row(flows.last_mut().expect("never empty"), l, r);
        }
    }
    Ok(flows
        .into_iter()
        .map(|f| Sheet {
            style,
            width,
            height: f.y,
            marks: f.marks,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(face: Face, texts: &[&str]) -> Vec<Cell> {
        texts
            .iter()
            .map(|t| Cell {
                text: (*t).to_string(),
                face,
            })
            .collect()
    }

    fn table(heading: &str, head: &[&str], rows: &[&[&str]]) -> Table {
        Table {
            heading: Cell::mono(heading),
            head: cells(Face::Proportional, head),
            rows: rows.iter().map(|r| cells(Face::Mono, r)).collect(),
            fallback: Vec::new(),
        }
    }

    fn width_of(face: Face, text: &str, size: f64) -> f64 {
        metrics::width_pt(face, text, size).expect("measured")
    }

    /// No line is wider than the column, and nothing but the spaces a
    /// break leaves is lost — in both faces, on identifiers, prose and a
    /// word wider than the column.
    #[test]
    fn a_wrapped_cell_fits_its_column_and_loses_no_character() {
        let samples = [
            "the quick brown fox jumps over the lazy dog",
            "very_long_identifier_name_with_many_seams_in_it",
            "unbreakable_but_much_too_wide_for_a_narrow_column",
            "x > 1 && y < 2 || z == 3",
        ];
        for face in [Face::Mono, Face::Proportional] {
            for sample in samples {
                for width in [40.0, 80.0, 160.0] {
                    let lines = wrap(face, sample, 7.0, width).expect("wraps");
                    for l in &lines {
                        assert!(
                            width_of(face, l, 7.0) <= width + 1e-6,
                            "{l:?} wider than {width} in {face:?}"
                        );
                    }
                    let strip = |s: &str| s.chars().filter(|c| *c != ' ').collect::<String>();
                    assert_eq!(strip(&lines.join("")), strip(sample), "{face:?} {width}");
                }
            }
        }
    }

    /// A hard break stays one, indentation is kept and a tab is set as
    /// spaces.
    #[test]
    fn hard_breaks_and_indentation_survive_wrapping() {
        let lines = wrap(Face::Mono, "a\n  b\n\tc", 7.0, 500.0).expect("wraps");
        assert_eq!(lines, vec!["a", "  b", "    c"]);
        assert_eq!(
            wrap(Face::Mono, "", 7.0, 50.0).expect("wraps"),
            vec![String::new()]
        );
    }

    /// A character the font table does not carry is refused, never given a
    /// width.
    #[test]
    fn an_unmeasured_character_is_refused() {
        let r = wrap(Face::Proportional, "go \u{0416}", 7.0, 500.0);
        assert!(
            matches!(r, Err(Refusal::Box(BoxError::Unmeasured(_)))),
            "{r:?}"
        );
    }

    /// Columns keep their natural width when there is room; when there is
    /// not, the narrow ones keep theirs and the widest take the cut, none
    /// below its minimum and the total within the budget.
    #[test]
    fn solved_widths_spare_the_narrow_columns() {
        let roomy = solve(&[10.0, 20.0], &[5.0, 5.0], 100.0);
        assert_eq!(roomy, vec![10.0, 20.0]);
        let tight = solve(&[10.0, 200.0, 40.0], &[10.0, 20.0, 20.0], 100.0);
        assert!(tight.iter().sum::<f64>() <= 100.0 + 1e-6, "{tight:?}");
        assert_eq!(tight[0], 10.0, "the narrow column is untouched");
        for ((w, n), m) in tight
            .iter()
            .zip([10.0, 200.0, 40.0])
            .zip([10.0, 20.0, 20.0])
        {
            assert!(*w >= m - 1e-9 && *w <= n + 1e-9, "{tight:?}");
        }
        assert!(tight[1] > tight[2], "the widest keeps the most: {tight:?}");
    }

    fn long_table(rows: usize) -> Table {
        let body: Vec<Vec<String>> = (0..rows)
            .map(|i| vec![i.to_string(), format!("value number {i}")])
            .collect();
        Table {
            heading: Cell::mono("document.entries"),
            head: cells(Face::Proportional, &["#", "value"]),
            rows: body
                .iter()
                .map(|r| r.iter().map(|t| Cell::mono(t.clone())).collect())
                .collect(),
            fallback: Vec::new(),
        }
    }

    /// A table longer than the page continues on the next with its
    /// heading marked and its head repeated; every row appears once, in
    /// order, and every page stays inside the area and at one width.
    #[test]
    fn a_long_table_continues_with_every_row_once() {
        let page = Page::a4_portrait(7.0);
        let sheets = set("title", "continued", &[long_table(400)], page).expect("fits");
        assert!(sheets.len() > 1, "400 rows cannot fit one page");
        let area = page.area_pt();
        let mut seen = Vec::new();
        for s in &sheets {
            assert!(s.width <= area.0 + 1e-6 && s.height <= area.1 + 1e-6);
            assert_eq!(s.width, sheets[0].width, "one width for the whole sheet");
            let words = s.words();
            assert_eq!(words[0], "title");
            assert!(words.contains(&"value"), "the head is on every page");
            for w in words {
                if let Some(n) = w.strip_prefix("value number ") {
                    seen.push(n.parse::<usize>().expect("a row number"));
                }
            }
        }
        assert_eq!(seen, (0..400).collect::<Vec<_>>());
        assert!(
            sheets[1].words().contains(&"document.entries (continued)"),
            "{:?}",
            sheets[1].words()
        );
    }

    /// Same tables, same marks.
    #[test]
    fn the_same_tables_set_to_the_same_marks() {
        let page = Page::a4_portrait(7.0);
        let a = set("t", "c", &[long_table(60)], page).expect("fits");
        let b = set("t", "c", &[long_table(60)], page).expect("fits");
        assert_eq!(a, b);
    }

    /// A table whose columns cannot fit at their minimum widths is set as
    /// its fallback, and refused when it has none.
    #[test]
    fn a_table_too_wide_for_the_page_falls_back_or_is_refused() {
        let page = Page::a4_portrait(7.0);
        let heads: Vec<String> = (0..30).map(|i| format!("column_{i:02}_name")).collect();
        let head_refs: Vec<&str> = heads.iter().map(String::as_str).collect();
        let row: Vec<&str> = head_refs.clone();
        let wide = table("document.rows", &head_refs, &[row.as_slice()]);
        let refused = set("t", "c", std::slice::from_ref(&wide), page);
        assert!(
            matches!(refused, Err(Refusal::SheetDoesNotFit { .. })),
            "{refused:?}"
        );
        let mut with_fallback = wide.clone();
        with_fallback.fallback = vec![table(
            "document.rows[0]",
            &["field", "value"],
            &[&["column_00_name", "x"]],
        )];
        let sheets = set("t", "c", &[with_fallback], page).expect("falls back");
        assert!(sheets[0].words().contains(&"document.rows[0]"));
        assert!(!sheets[0].words().contains(&"document.rows"));
    }

    /// A row taller than the page is refused with its measured height.
    #[test]
    fn a_row_taller_than_the_page_is_refused() {
        let page = Page::a4_portrait(7.0);
        let tall = "line\n".repeat(400);
        let t = table("h", &["a"], &[&[tall.as_str()]]);
        let r = set("t", "c", &[t], page);
        match r {
            Err(Refusal::SheetDoesNotFit {
                need_pt, area_pt, ..
            }) => {
                assert!(need_pt.1 > area_pt.1, "{need_pt:?} in {area_pt:?}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// A heading never stands alone at the foot of a page: it moves with
    /// its first row.
    #[test]
    fn a_heading_stays_with_its_first_row() {
        let page = Page::a4_portrait(7.0);
        let filler = long_table(100);
        let next = table("second", &["a"], &[&["only row"]]);
        let sheets = set("t", "c", &[filler, next], page).expect("fits");
        for s in &sheets {
            let words = s.words();
            if let Some(i) = words.iter().position(|w| *w == "second") {
                assert!(words[i..].contains(&"only row"), "{words:?}");
            }
        }
    }
}
