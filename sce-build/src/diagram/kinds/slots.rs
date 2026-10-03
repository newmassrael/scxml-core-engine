// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A row of slots: the picture of a buffer pool and of a bounded collection.
//!
//! Both kinds are a fixed number of equal places. The picture is that
//! number made countable — one cell for each, in a row, with the size of one
//! and the total written beside it — so "eight 256-byte slots" is seen and
//! not only read. The cells are not to scale: they are as many as the
//! document says, and as wide as their number needs.
//!
//! A row longer than [`WHOLE_ROW_LIMIT`] draws its first and last
//! [`EDGE_CELLS`] cells and one cell between them that says which numbers
//! it stands for (`8..23`), so no cell is left unaccounted for. The count
//! above the row is the whole document's, never the drawn cells'.

use super::{caption, dimension, facts, say, Picture};
use crate::diagram::canvas::Canvas;
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::metrics::Face;
use crate::diagram::sheet::Ink;
use crate::diagram::words::Phrase;
use crate::forge::model::{BoundedCollectionModel, BufferPoolModel, CapacitySource};
use crate::forge::page::Lexicon;

/// Beyond [`WHOLE_ROW_LIMIT`], this many cells are drawn at each end.
const EDGE_CELLS: u32 = 8;

/// A row of up to this many slots is drawn cell by cell: a row any longer
/// elides at least three, so the elision saves cells and is never a single
/// cell standing for a single slot.
const WHOLE_ROW_LIMIT: u32 = 2 * EDGE_CELLS + 2;

/// One drawn cell: slot `0`-based, or a run of slots not drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Cell {
    Slot(u32),
    Elided { from: u32, to: u32 },
}

impl Cell {
    fn label(&self) -> String {
        match self {
            Cell::Slot(i) => i.to_string(),
            Cell::Elided { from, to } => format!("{from}..{to}"),
        }
    }
}

/// The cells drawn for `count` slots.
fn cells(count: u32) -> Vec<Cell> {
    if count <= WHOLE_ROW_LIMIT {
        return (0..count).map(Cell::Slot).collect();
    }
    let mut out: Vec<Cell> = (0..EDGE_CELLS).map(Cell::Slot).collect();
    out.push(Cell::Elided {
        from: EDGE_CELLS,
        to: count - EDGE_CELLS - 1,
    });
    out.extend((count - EDGE_CELLS..count).map(Cell::Slot));
    out
}

/// Where a drawn row ended up.
struct Row {
    x0: f64,
    x1: f64,
    y1: f64,
    /// The width of one cell, which is also the first cell's.
    cell: f64,
}

fn row(c: &mut Canvas, (x0, y0): (f64, f64), count: u32) -> Result<Row, Refusal> {
    let cells = cells(count);
    let body = c.style().body_pt;
    let padding = 1.6 * body;
    // Slot cells share the width their widest number needs; the elided
    // cell is as wide as the range it names, so a long range does not
    // widen every cell of the row.
    let mut slot = 0.0f64;
    for cell in &cells {
        if matches!(cell, Cell::Slot(_)) {
            slot = slot.max(c.width_of(Face::Mono, &cell.label(), body)? + padding);
        }
    }
    let mut widths = Vec::with_capacity(cells.len());
    for cell in &cells {
        widths.push(match cell {
            Cell::Slot(_) => slot,
            Cell::Elided { .. } => slot.max(c.width_of(Face::Mono, &cell.label(), body)? + padding),
        });
    }
    let height = c.line_height() + 1.2 * body;
    let mut x = x0;
    for (cell, width) in cells.iter().zip(&widths) {
        let fill = matches!(cell, Cell::Elided { .. }).then_some(Ink::Shade);
        c.frame((x, y0, *width, height), fill);
        c.text_centered(
            x + width / 2.0,
            y0 + (height - c.line_height()) / 2.0,
            &cell.label(),
            Face::Mono,
        )?;
        x += width;
    }
    Ok(Row {
        x0,
        x1: x,
        y1: y0 + height,
        cell: widths[0],
    })
}

/// The row of slots with its dimensions, and `facts` in words under it.
fn picture(
    c: &mut Canvas,
    title: &str,
    count: u32,
    noun: &str,
    size_of_one: Option<(u64, &str)>,
    facts: &[(String, String)],
) -> Result<(), Refusal> {
    let line = c.line_height();
    let gap = line * 0.75;
    c.title(0.0, 0.0, title)?;
    let mut y = c.style().title_pt * c.style().leading + line;

    // Room above the row for the count's dimension line and its label.
    let above = y + line + gap;
    let r = row(c, (0.0, above + gap), count)?;
    dimension(c, (r.x0, r.x1), above, &format!("{count} {noun}"), true)?;
    y = r.y1 + gap;

    if let Some((bytes, unit)) = size_of_one {
        let below = y + gap;
        dimension(
            c,
            (r.x0, r.x0 + r.cell),
            below,
            &format!("{bytes} {unit}"),
            false,
        )?;
        let total = u64::from(count) * bytes;
        let lower = below + c.line_height() + gap * 2.0;
        dimension(c, (r.x0, r.x1), lower, &format!("{total} {unit}"), false)?;
        y = lower + c.line_height() + gap;
    }
    caption(c, 0.0, y + gap, facts)?;
    Ok(())
}

/// A buffer pool: its slots, the size of one and of all.
pub fn buffer_pool(
    m: &BufferPoolModel,
    lexicon: &Lexicon,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    if m.slot_count == 0 {
        return Ok(Vec::new());
    }
    let mut c = Canvas::new(page);
    let title = format!("{}: {}", m.name, say(lexicon, Phrase::Slots)?);
    let facts = facts(m, &["name", "slot_count", "slot_size"])?;
    picture(
        &mut c,
        &title,
        m.slot_count,
        say(lexicon, Phrase::Slots)?,
        Some((u64::from(m.slot_size), say(lexicon, Phrase::Bytes)?)),
        &facts,
    )?;
    Ok(vec![Picture {
        stem: "slots",
        sheet: c.finish(page)?,
    }])
}

/// A bounded collection: its capacity as that many cells. A capacity the
/// deploy configuration names has no number in the document to draw.
pub fn bounded_collection(
    m: &BoundedCollectionModel,
    lexicon: &Lexicon,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    let CapacitySource::CompileConst { value } = m.capacity else {
        return Ok(Vec::new());
    };
    if value == 0 {
        return Ok(Vec::new());
    }
    let mut c = Canvas::new(page);
    let title = format!("{}: {}", m.name, say(lexicon, Phrase::Entries)?);
    let facts = facts(m, &["name"])?;
    picture(
        &mut c,
        &title,
        value,
        say(lexicon, Phrase::Entries)?,
        None,
        &facts,
    )?;
    Ok(vec![Picture {
        stem: "slots",
        sheet: c.finish(page)?,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::sheet::Mark;
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn kind(name: &str) -> crate::forge::model::ForgeDocument {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join(format!("{name}.scxml"));
        let text = std::fs::read_to_string(&path).expect("reads");
        parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
            .expect("parses")
            .expect("not a statechart")
            .document
    }

    /// Up to the limit every slot is a cell; beyond it the first and last
    /// edge cells are drawn and one cell names the numbers between, so the
    /// numbers drawn and elided together are exactly `0..count`.
    #[test]
    fn every_slot_is_drawn_or_named_as_elided() {
        for count in [1u32, 8, 16, 18, 19, 24, 100, 4096] {
            let drawn = cells(count);
            let mut seen: Vec<u32> = Vec::new();
            for cell in &drawn {
                match cell {
                    Cell::Slot(i) => seen.push(*i),
                    Cell::Elided { from, to } => seen.extend(*from..=*to),
                }
            }
            assert_eq!(seen, (0..count).collect::<Vec<_>>(), "{count}");
            assert!(
                drawn.len() <= WHOLE_ROW_LIMIT as usize,
                "{count}: {} cells",
                drawn.len()
            );
        }
    }

    /// The buffer pool's own example: eight cells, with the document's
    /// slot size, count and total written in the page's words, and the
    /// scalar facts under the row.
    #[test]
    fn the_buffer_pool_example_is_eight_cells_with_its_dimensions() {
        let m = pool();
        let page = Page::a4_portrait(7.0);
        let pictures = buffer_pool(&m, &EN, page).expect("draws");
        assert_eq!(pictures.len(), 1);
        let sheet = &pictures[0].sheet;
        let words = sheet.words();
        for expected in ["buffer-pool: slots", "8 slots", "256 bytes", "2048 bytes"] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        let frames = sheet
            .marks
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    Mark::Rect {
                        outline: Some(_),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(frames, 8, "one cell for each slot");
        for fact in [
            "section",
            "sram1",
            "alignment",
            "32",
            "cache_policy",
            "none",
        ] {
            assert!(words.contains(&fact), "{fact}: {words:?}");
        }
        assert!(
            !words.contains(&"slot_count") && !words.contains(&"slot_size"),
            "what is drawn is not said twice: {words:?}"
        );
        let korean = buffer_pool(&m, &KO, page).expect("draws");
        assert_ne!(korean[0].sheet.words(), words, "in the page's language");
    }

    fn pool() -> BufferPoolModel {
        match kind("buffer-pool") {
            crate::forge::model::ForgeDocument::BufferPool(m) => m,
            other => panic!("{other:?}"),
        }
    }

    /// A collection whose capacity the deploy configuration names has no
    /// number to draw, and a constant one is that many cells.
    #[test]
    fn a_collection_is_drawn_only_when_the_document_gives_its_capacity() {
        let page = Page::a4_portrait(7.0);
        let crate::forge::model::ForgeDocument::BoundedCollection(mut m) =
            kind("bounded-collection")
        else {
            panic!("a bounded-collection example")
        };
        let drawn = bounded_collection(&m, &EN, page).expect("draws");
        assert_eq!(drawn.len(), 1);
        assert!(drawn[0].sheet.words().contains(&"8 entries"));
        m.capacity = CapacitySource::DeployKey { key: "k".into() };
        assert!(bounded_collection(&m, &EN, page).expect("draws").is_empty());
    }

    /// A pool of thousands of slots is still one short row, and says how
    /// many there are.
    #[test]
    fn a_very_long_row_stays_one_short_row_that_says_its_count() {
        let mut m = pool();
        m.slot_count = 4096;
        let page = Page::a4_portrait(7.0);
        let drawn = buffer_pool(&m, &EN, page).expect("draws");
        let words = drawn[0].sheet.words();
        assert!(words.contains(&"4096 slots"), "{words:?}");
        assert!(words.contains(&"8..4087"), "{words:?}");
    }
}
