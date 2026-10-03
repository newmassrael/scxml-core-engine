// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A printed sheet as marks on a page: the words, boxes, strokes and dots a
//! figure of a non-statechart kind is made of, already placed.
//!
//! This is the print diagram's second drawing surface. The statechart
//! figure is boxes joined by arrows ([`super::fit::Printed`]); a table, a
//! chart or a bit layout is none of that, so each of those is typeset into
//! marks and `svg::render_sheet` writes them out. Like the figure's
//! renderer it decides nothing — a second renderer drawing the same marks
//! draws the same sheet, and a test on the marks is a test on both.
//!
//! Units are points, the unit the fit was measured in. A mark's text is
//! the words the sheet says; a sheet is built only from words taken
//! through [`super::words`] and from values of the document, so it cannot
//! name one thing two ways.

use super::boxes::{Line, Style};

/// A shade a mark is drawn in. Named by role, not by colour, so a
/// renderer may choose its own and a greyscale print still tells them
/// apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ink {
    /// Text, outlines and the rule that closes a table's head.
    Black,
    /// What is stated without being drawn from the document's values: an
    /// axis, a reference line, a stretch the document leaves open.
    Muted,
    /// The hairline between two rows.
    Hairline,
    /// A light fill: the band a table's heading stands on, a box's body.
    Shade,
    /// The colour of the page itself: what a label is backed with so a
    /// line passing under it does not run through its letters.
    Paper,
    /// One of [`LEVELS`] light greys, 0 the lightest: how large a value is,
    /// shown as a shade under its number. Light, so the number stays
    /// legible on the darkest.
    Level(u8),
}

/// How many [`Ink::Level`]s there are.
pub const LEVELS: u8 = 5;

/// An outline: its ink and its width in points.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Outline {
    pub ink: Ink,
    pub width_pt: f64,
}

/// One thing drawn on a sheet.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "mark", rename_all = "kebab-case")]
pub enum Mark {
    /// One line of text whose line box starts at `top`.
    Text { x: f64, top: f64, line: Line },
    /// A rectangle, filled, outlined, or both.
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<Ink>,
        outline: Option<Outline>,
    },
    /// A straight stroke.
    Stroke {
        from: (f64, f64),
        to: (f64, f64),
        ink: Ink,
        width_pt: f64,
        dashed: bool,
    },
    /// A stroke through several points, in order.
    Polyline {
        points: Vec<(f64, f64)>,
        ink: Ink,
        width_pt: f64,
        dashed: bool,
    },
    /// A filled dot.
    Dot {
        x: f64,
        y: f64,
        radius: f64,
        ink: Ink,
    },
    /// A filled polygon: an arrowhead.
    Polygon { points: Vec<(f64, f64)>, ink: Ink },
}

impl Mark {
    /// This mark moved by `(dx, dy)`.
    pub fn shifted(mut self, dx: f64, dy: f64) -> Mark {
        match &mut self {
            Mark::Text { x, top, .. } => {
                *x += dx;
                *top += dy;
            }
            Mark::Rect { x, y, .. } | Mark::Dot { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            Mark::Stroke { from, to, .. } => {
                *from = (from.0 + dx, from.1 + dy);
                *to = (to.0 + dx, to.1 + dy);
            }
            Mark::Polyline { points, .. } | Mark::Polygon { points, .. } => {
                for p in points {
                    *p = (p.0 + dx, p.1 + dy);
                }
            }
        }
        self
    }
}

/// One printed page of a sheet.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Sheet {
    pub style: Style,
    pub width: f64,
    pub height: f64,
    pub marks: Vec<Mark>,
}

impl Sheet {
    /// The text of every [`Mark::Text`], in drawing order.
    pub fn words(&self) -> Vec<&str> {
        self.marks
            .iter()
            .filter_map(|m| match m {
                Mark::Text { line, .. } => Some(line.text.as_str()),
                _ => None,
            })
            .collect()
    }
}
