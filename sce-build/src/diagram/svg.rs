// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A printed figure as SVG.
//!
//! This renderer decides nothing: every word, box, outline, arrow and
//! coordinate is already in [`Printed`], placed. It only writes them out,
//! so a second renderer (the GUI) drawing the same `Printed` draws the
//! same figure, and a test on `Printed` is a test on both.
//!
//! Units are points throughout — the unit the fit was measured in — and
//! numbers are written with two decimals, so the same figure is the same
//! bytes on every machine.

use super::boxes::{Line, Outline};
use super::fit::Printed;
use super::layout::Placed;
use super::metrics::Face;
use crate::parser::{escape_xml_attribute as attr, escape_xml_text as text};
use std::fmt::Write as _;

const INK: &str = "#000000";
const BRIEF_INK: &str = "#707070";
const ELSEWHERE_FILL: &str = "#e6e6e6";
const PAPER: &str = "#ffffff";

/// `printed` as one standalone SVG document.
pub fn render(printed: &Printed) -> String {
    let s = printed.style;
    let stroke = s.body_pt * 0.1;
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="0 0 {w} {h}">"#,
        w = n(printed.width),
        h = n(printed.height),
    );
    let head = s.body_pt * 0.8;
    let _ = writeln!(
        out,
        r#"<defs><marker id="head" viewBox="0 0 10 10" refX="10" refY="5" markerUnits="userSpaceOnUse" markerWidth="{m}" markerHeight="{m}" orient="auto"><path d="M0,0 L10,5 L0,10 z"/></marker></defs>"#,
        m = n(head),
    );
    let _ = writeln!(
        out,
        r#"<rect width="{}" height="{}" fill="{PAPER}"/>"#,
        n(printed.width),
        n(printed.height)
    );
    write_line(
        &mut out,
        &Line {
            text: printed.title.clone(),
            face: Face::Proportional,
            size_pt: s.title_pt,
        },
        0.0,
        0.0,
        s.leading,
    );

    let (dx, dy) = printed.drawing_at;
    let _ = writeln!(out, r#"<g transform="translate({},{})">"#, n(dx), n(dy));
    let laid = &printed.laid;
    if let Some(frame) = &laid.frame {
        outline(
            &mut out,
            frame.sized.outline,
            (0.0, 0.0, laid.body_width, laid.body_height),
            stroke,
        );
        write_lines(&mut out, frame, s.padding, s.leading);
        let _ = writeln!(
            out,
            r#"<line x1="0" y1="{y}" x2="{w}" y2="{y}" stroke="{INK}" stroke-width="{sw}"/>"#,
            y = n(frame.sized.height),
            w = n(laid.body_width),
            sw = n(stroke),
        );
    }
    for p in laid.inner.iter().chain(&laid.edge) {
        outline(
            &mut out,
            p.sized.outline,
            (p.x, p.y, p.sized.width, p.sized.height),
            stroke,
        );
        write_lines(&mut out, p, s.padding, s.leading);
    }
    if let Some(m) = &printed.marker {
        let _ = writeln!(
            out,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{INK}"/>"#,
            n(m.dot.0),
            n(m.dot.1),
            n(m.radius)
        );
        polyline(&mut out, &m.path, INK, stroke);
    }
    for a in &printed.arrows {
        let ink = if a.brief { BRIEF_INK } else { INK };
        polyline(&mut out, &a.path, ink, stroke);
        let _ = writeln!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{PAPER}"/>"#,
            n(a.label_at.0),
            n(a.label_at.1),
            n(a.label_size.0),
            n(a.label_size.1)
        );
        for (i, l) in a.label.iter().enumerate() {
            write_line(
                &mut out,
                &Line {
                    text: l.clone(),
                    face: Face::Proportional,
                    size_pt: s.body_pt,
                },
                a.label_at.0,
                a.label_at.1 + i as f64 * s.body_pt * s.leading,
                s.leading,
            );
        }
    }
    out.push_str("</g>\n");

    for (i, row) in printed.table.iter().enumerate() {
        let y = printed.table_top + i as f64 * s.body_pt * s.leading;
        let cells = [
            row.number.to_string(),
            row.cells[0].clone(),
            row.cells[1].clone(),
        ];
        for (x, cell) in printed.columns.iter().zip(cells) {
            write_line(
                &mut out,
                &Line {
                    text: cell,
                    face: Face::Mono,
                    size_pt: s.body_pt,
                },
                *x,
                y,
                s.leading,
            );
        }
    }
    out.push_str("</svg>\n");
    out
}

/// Two decimals: enough for print, and the same bytes everywhere.
fn n(v: f64) -> String {
    let r = format!("{v:.2}");
    // `-0.00` and `0.00` are one coordinate; write it one way.
    if r == "-0.00" {
        "0.00".to_string()
    } else {
        r
    }
}

fn outline(out: &mut String, o: Outline, (x, y, w, h): (f64, f64, f64, f64), stroke: f64) {
    let radius = stroke * 30.0;
    let (fill, dash) = match o {
        Outline::Elsewhere => (ELSEWHERE_FILL, ""),
        Outline::Parallel => (PAPER, r#" stroke-dasharray="4 2""#),
        _ => (PAPER, ""),
    };
    let _ = writeln!(
        out,
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{r}" fill="{fill}" stroke="{INK}" stroke-width="{sw}"{dash}/>"#,
        n(x),
        n(y),
        n(w),
        n(h),
        r = n(radius),
        sw = n(stroke),
    );
    match o {
        Outline::Final => {
            let inset = stroke * 15.0;
            let _ = writeln!(
                out,
                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{r}" fill="none" stroke="{INK}" stroke-width="{sw}"/>"#,
                n(x + inset),
                n(y + inset),
                n(w - 2.0 * inset),
                n(h - 2.0 * inset),
                r = n((radius - inset).max(0.0)),
                sw = n(stroke),
            );
        }
        Outline::Folded => {
            // A folded corner, top right.
            let c = stroke * 40.0;
            let _ = writeln!(
                out,
                r#"<path d="M{},{} L{},{} L{},{}" fill="none" stroke="{INK}" stroke-width="{sw}"/>"#,
                n(x + w - c),
                n(y),
                n(x + w - c),
                n(y + c),
                n(x + w),
                n(y + c),
                sw = n(stroke),
            );
        }
        _ => {}
    }
}

fn write_lines(out: &mut String, p: &Placed, padding: f64, leading: f64) {
    let mut y = p.y + padding;
    for l in &p.sized.lines {
        write_line(out, l, p.x + padding, y, leading);
        y += l.size_pt * leading;
    }
}

/// One line whose line box starts at `top`: the baseline sits the half
/// leading plus the face's ascender below it.
fn write_line(out: &mut String, l: &Line, x: f64, top: f64, leading: f64) {
    let baseline = top + l.size_pt * (leading - 1.0) / 2.0 + l.size_pt * l.face.ascent();
    let _ = writeln!(
        out,
        r#"<text x="{}" y="{}" font-family="{}" font-size="{}" xml:space="preserve">{}</text>"#,
        n(x),
        n(baseline),
        attr(l.face.family()),
        n(l.size_pt),
        text(&l.text)
    );
}

fn polyline(out: &mut String, points: &[(f64, f64)], ink: &str, stroke: f64) {
    let pts: Vec<String> = points
        .iter()
        .map(|(x, y)| format!("{},{}", n(*x), n(*y)))
        .collect();
    let _ = writeln!(
        out,
        r#"<polyline points="{}" fill="none" stroke="{ink}" stroke-width="{}" marker-end="url(#head)"/>"#,
        pts.join(" "),
        n(stroke)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::fit::{print, Page};
    use crate::forge::page::{EN, KO};
    use crate::parser::SCXMLParser;

    const DOC: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="released">
  <state id="released" initial="unlocked">
    <state id="unlocked"><transition event="lock.request" target="locked"/></state>
    <state id="relocking" initial="waiting">
      <state id="waiting"><transition event="timer" target="armed"/></state>
      <state id="armed"><transition event="expire" target="locked"/></state>
    </state>
    <transition event="speed.high" target="relocking"/>
  </state>
  <state id="locked">
    <transition event="unlock.request" target="unlocked"/>
    <transition event="self" target="locked"/>
  </state>
</scxml>"##;

    fn figures(lexicon: &crate::forge::page::Lexicon) -> Vec<String> {
        let m = SCXMLParser::new().parse_string(DOC, "svg").expect("parses");
        print(&m, lexicon, Page::a4_portrait(7.0))
            .expect("fits")
            .iter()
            .map(render)
            .collect()
    }

    /// Every figure is well-formed XML, and what `Printed` says is written:
    /// each box's first line, each table row, each arrow label.
    #[test]
    fn every_printed_word_reaches_the_svg() {
        let m = SCXMLParser::new().parse_string(DOC, "svg").expect("parses");
        let printed = print(&m, &EN, Page::a4_portrait(7.0)).expect("fits");
        for p in &printed {
            let svg = render(p);
            let doc = roxmltree::Document::parse(&svg).expect("well-formed");
            let texts: Vec<&str> = doc
                .descendants()
                .filter(|n| n.has_tag_name("text"))
                .filter_map(|n| n.text())
                .collect();
            let boxes = p.laid.frame.iter().chain(&p.laid.inner).chain(&p.laid.edge);
            for b in boxes {
                for l in &b.sized.lines {
                    assert!(
                        texts.contains(&l.text.as_str()),
                        "{:?} in {texts:?}",
                        l.text
                    );
                }
            }
            for r in &p.table {
                assert!(texts.contains(&r.cells[0].as_str()), "{r:?}");
                assert!(texts.contains(&r.cells[1].as_str()), "{r:?}");
            }
            for a in &p.arrows {
                for l in &a.label {
                    assert!(texts.contains(&l.as_str()), "{l:?} in {texts:?}");
                }
            }
            assert!(texts.contains(&p.title.as_str()));
            let arrows = doc
                .descendants()
                .filter(|n| n.attribute("marker-end").is_some())
                .count();
            let marker = usize::from(p.marker.is_some());
            assert_eq!(arrows, p.arrows.len() + marker, "one drawn line per arrow");
        }
    }

    /// Same document, same bytes — in both page languages.
    #[test]
    fn the_svg_is_deterministic() {
        assert_eq!(figures(&EN), figures(&EN));
        assert_eq!(figures(&KO), figures(&KO));
    }

    /// Every label, box and arrow lies inside the SVG's own canvas.
    #[test]
    fn nothing_is_drawn_outside_the_canvas() {
        let m = SCXMLParser::new().parse_string(DOC, "svg").expect("parses");
        for p in print(&m, &EN, Page::a4_portrait(7.0)).expect("fits") {
            let (dx, dy) = p.drawing_at;
            let inside = |(x, y): (f64, f64)| {
                let (x, y) = (x + dx, y + dy);
                x >= -1e-9 && y >= -1e-9 && x <= p.width + 1e-9 && y <= p.height + 1e-9
            };
            for a in &p.arrows {
                for &pt in &a.path {
                    assert!(inside(pt), "{pt:?} of {a:?}");
                }
                let far = (a.label_at.0 + a.label_size.0, a.label_at.1 + a.label_size.1);
                assert!(inside(a.label_at) && inside(far), "{a:?}");
            }
            for b in p.laid.inner.iter().chain(&p.laid.edge) {
                assert!(inside((b.x, b.y)), "{b:?}");
                assert!(inside((b.x + b.sized.width, b.y + b.sized.height)), "{b:?}");
            }
        }
    }
}
