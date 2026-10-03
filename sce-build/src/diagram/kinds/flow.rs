// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Inputs, what is computed from them, and what comes out: the picture of
//! a transform, a condition, a filter and a validator.
//!
//! Three columns of boxes. The inputs are on the left, as the document
//! declares them; the computation is in the middle — an expression, the
//! settings of a filter, the rules of a validator; the outputs are on the
//! right. An arrow joins an input to a computation only where the document's
//! own expression READS it (the parsed tree's identifiers, so `frame.len`
//! reads `frame` and never `len`), and a computation to its output where
//! the document gives the output that computation. An input no expression
//! reads is drawn with no arrow, which is the thing worth seeing.
//!
//! An expression that does not parse has no reading, so the document gets
//! no picture rather than one with arrows guessed from its text.

use super::{facts, say, wrapped, Block, Picture};
use crate::diagram::canvas::Canvas;
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::metrics::Face;
use crate::diagram::sheet::Ink;
use crate::diagram::words::Phrase;
use crate::forge::expr::read_identifiers;
use crate::forge::model::{ForgeDocument, ForgeField};
use crate::forge::page::Lexicon;

/// The widest an expression is set before it wraps, in points.
const EXPRESSION_W: f64 = 230.0;

/// The room between two columns, in body-text sizes: an arrow needs some.
const COLUMN_GAP_EM: f64 = 7.0;

/// One computation: its box, the inputs it reads, and the output it
/// gives.
struct Middle {
    block: Block,
    reads: Vec<usize>,
    gives: Option<usize>,
}

/// The three columns, before they are placed.
struct Flow {
    inputs: Vec<Block>,
    middles: Vec<Middle>,
    outputs: Vec<Block>,
}

/// A field as a box: its name, and its type.
fn field_block(c: &Canvas, f: &ForgeField) -> Result<Block, Refusal> {
    Block::new(
        c,
        vec![
            (f.id.clone(), Face::Mono),
            (f.sce_type.as_attr(), Face::Mono),
        ],
    )
}

/// The indexes into `inputs` of the inputs `expr` reads, in input order;
/// `None` for an expression that does not parse.
fn reads(expr: &str, inputs: &[ForgeField]) -> Option<Vec<usize>> {
    let names = read_identifiers(expr).ok()?;
    Some(
        inputs
            .iter()
            .enumerate()
            .filter(|(_, f)| names.contains(&f.id))
            .map(|(i, _)| i)
            .collect(),
    )
}

fn input_blocks(c: &Canvas, inputs: &[ForgeField]) -> Result<Vec<Block>, Refusal> {
    inputs.iter().map(|f| field_block(c, f)).collect()
}

/// The flow of a document of one of the four kinds, or `None` when an
/// expression of it has no reading.
fn flow_of(
    c: &Canvas,
    doc: &ForgeDocument,
    lexicon: &Lexicon,
) -> Result<Option<(String, Flow)>, Refusal> {
    Ok(match doc {
        ForgeDocument::Transform(m) => {
            let mut middles = Vec::new();
            for (k, out) in m.outputs.iter().enumerate() {
                let Some(expr) = &out.expr else { continue };
                let Some(r) = reads(expr, &m.inputs) else {
                    return Ok(None);
                };
                middles.push(Middle {
                    block: Block::new(c, wrapped(c, expr, EXPRESSION_W)?)?,
                    reads: r,
                    gives: Some(k),
                });
            }
            Some((
                m.name.clone(),
                Flow {
                    inputs: input_blocks(c, &m.inputs)?,
                    middles,
                    outputs: m
                        .outputs
                        .iter()
                        .map(|f| field_block(c, f))
                        .collect::<Result<_, _>>()?,
                },
            ))
        }
        ForgeDocument::Condition(m) => {
            let Some(r) = reads(&m.expr, &m.inputs) else {
                return Ok(None);
            };
            Some((
                m.name.clone(),
                Flow {
                    inputs: input_blocks(c, &m.inputs)?,
                    middles: vec![Middle {
                        block: Block::new(c, wrapped(c, &m.expr, EXPRESSION_W)?)?,
                        reads: r,
                        gives: Some(0),
                    }],
                    outputs: vec![Block::new(c, vec![(m.name.clone(), Face::Mono)])?],
                },
            ))
        }
        ForgeDocument::Filter(m) => {
            let settings: Vec<(String, Face)> = facts(m, &["name"])?
                .into_iter()
                .map(|(k, v)| (format!("{k}  {v}"), Face::Mono))
                .collect();
            Some((
                m.name.clone(),
                Flow {
                    inputs: vec![field_block(c, &m.input)?],
                    middles: vec![Middle {
                        block: Block::new(c, settings)?,
                        reads: vec![0],
                        gives: Some(0),
                    }],
                    outputs: vec![field_block(c, &m.output)?],
                },
            ))
        }
        ForgeDocument::Validator(m) => {
            let mut middles = Vec::new();
            let index = |id: &str| m.inputs.iter().position(|f| f.id == id);
            for rule in &m.rules.ranges {
                let low = rule.min.clone().unwrap_or_default();
                let high = rule.max.clone().unwrap_or_default();
                middles.push(Middle {
                    block: Block::new(
                        c,
                        vec![
                            (say(lexicon, Phrase::Range)?.to_string(), Face::Proportional),
                            (format!("{low} .. {high}"), Face::Mono),
                        ],
                    )?,
                    reads: index(&rule.id).into_iter().collect(),
                    gives: None,
                });
            }
            for rule in &m.rules.rate_of_changes {
                middles.push(Middle {
                    block: Block::new(
                        c,
                        vec![
                            (
                                say(lexicon, Phrase::MaxChange)?.to_string(),
                                Face::Proportional,
                            ),
                            (
                                format!(
                                    "{} {} {} ms",
                                    rule.max_delta,
                                    say(lexicon, Phrase::Every)?,
                                    rule.sample_interval_ms
                                ),
                                Face::Mono,
                            ),
                        ],
                    )?,
                    reads: index(&rule.id).into_iter().collect(),
                    gives: None,
                });
            }
            if let Some(expr) = &m.rules.plausibility {
                let Some(r) = reads(expr, &m.inputs) else {
                    return Ok(None);
                };
                let mut lines = vec![(
                    say(lexicon, Phrase::Plausibility)?.to_string(),
                    Face::Proportional,
                )];
                lines.extend(wrapped(c, expr, EXPRESSION_W)?);
                middles.push(Middle {
                    block: Block::new(c, lines)?,
                    reads: r,
                    gives: None,
                });
            }
            Some((
                m.name.clone(),
                Flow {
                    inputs: input_blocks(c, &m.inputs)?,
                    middles,
                    outputs: Vec::new(),
                },
            ))
        }
        _ => None,
    })
}

/// A column's boxes stacked top to bottom, `spacing` apart, centred on
/// `centre`: each box's top.
fn stack(blocks: &[f64], spacing: f64, centre: f64) -> (Vec<f64>, f64) {
    let total = blocks.iter().sum::<f64>() + spacing * blocks.len().saturating_sub(1) as f64;
    let mut y = centre - total / 2.0;
    let mut tops = Vec::with_capacity(blocks.len());
    for h in blocks {
        tops.push(y);
        y += h + spacing;
    }
    (tops, total)
}

/// A point on the side of a box: its `slot`th of `of` evenly spread places,
/// so several arrows meeting one box arrive apart.
fn port(top: f64, height: f64, slot: usize, of: usize) -> f64 {
    top + height * (slot + 1) as f64 / (of + 1) as f64
}

/// The dataflow picture of a transform, condition, filter or validator.
pub fn dataflow(
    doc: &ForgeDocument,
    lexicon: &Lexicon,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    let mut c = Canvas::new(page);
    let Some((name, flow)) = flow_of(&c, doc, lexicon)? else {
        return Ok(Vec::new());
    };
    if flow.inputs.is_empty() && flow.middles.is_empty() && flow.outputs.is_empty() {
        return Ok(Vec::new());
    }
    let body = c.style().body_pt;
    let title = format!("{name}: {}", say(lexicon, Phrase::Dataflow)?);
    c.title(0.0, 0.0, &title)?;
    let top = c.style().title_pt * c.style().leading + c.line_height();

    let widest = |blocks: &[Block]| blocks.iter().map(|b| b.width).fold(0.0, f64::max);
    let middle_blocks: Vec<&Block> = flow.middles.iter().map(|m| &m.block).collect();
    let in_w = widest(&flow.inputs);
    let mid_w = middle_blocks.iter().map(|b| b.width).fold(0.0, f64::max);
    let gap = COLUMN_GAP_EM * body;
    let (in_x, mid_x) = (0.0, in_w + gap);
    let out_x = mid_x + mid_w + gap;

    let spacing = body * 1.2;
    let heights = |blocks: Vec<&Block>| blocks.iter().map(|b| b.height).collect::<Vec<_>>();
    let in_h = heights(flow.inputs.iter().collect());
    let mid_h = heights(middle_blocks);
    let out_h = heights(flow.outputs.iter().collect());
    let tallest = [&in_h, &mid_h, &out_h]
        .iter()
        .map(|h| h.iter().sum::<f64>() + spacing * h.len().saturating_sub(1) as f64)
        .fold(0.0, f64::max);
    let centre = top + tallest / 2.0;
    let (in_tops, _) = stack(&in_h, spacing, centre);
    let (mid_tops, _) = stack(&mid_h, spacing, centre);
    let (out_tops, _) = stack(&out_h, spacing, centre);

    // How many arrows leave each input and arrive at each computation, so
    // the ones that share a side are spread along it.
    let mut leaving = vec![0usize; flow.inputs.len()];
    for m in &flow.middles {
        for &i in &m.reads {
            leaving[i] += 1;
        }
    }
    let mut left_of_input = vec![0usize; flow.inputs.len()];
    for (k, m) in flow.middles.iter().enumerate() {
        for (slot, &i) in m.reads.iter().enumerate() {
            let from = (
                in_x + flow.inputs[i].width,
                port(in_tops[i], in_h[i], left_of_input[i], leaving[i]),
            );
            left_of_input[i] += 1;
            let to = (mid_x, port(mid_tops[k], mid_h[k], slot, m.reads.len()));
            c.arrow(from, to, Ink::Black);
        }
    }
    for (k, m) in flow.middles.iter().enumerate() {
        if let Some(o) = m.gives {
            if o < flow.outputs.len() {
                let from = (
                    mid_x + flow.middles[k].block.width,
                    mid_tops[k] + mid_h[k] / 2.0,
                );
                let to = (out_x, out_tops[o] + out_h[o] / 2.0);
                c.arrow(from, to, Ink::Black);
            }
        }
    }
    for (i, b) in flow.inputs.iter().enumerate() {
        b.draw(&mut c, in_x, in_tops[i], None)?;
    }
    for (k, m) in flow.middles.iter().enumerate() {
        m.block.draw(&mut c, mid_x, mid_tops[k], Some(Ink::Shade))?;
    }
    for (o, b) in flow.outputs.iter().enumerate() {
        b.draw(&mut c, out_x, out_tops[o], None)?;
    }
    Ok(vec![Picture {
        stem: "dataflow".into(),
        sheet: c.finish(page, &title)?,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::sheet::Mark;
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn kind(name: &str) -> ForgeDocument {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join(format!("{name}.scxml"));
        let text = std::fs::read_to_string(&path).expect("reads");
        parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
            .expect("parses")
            .expect("not a statechart")
            .document
    }

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    fn picture(doc: &ForgeDocument) -> crate::diagram::sheet::Sheet {
        dataflow(doc, &EN, page()).expect("draws").remove(0).sheet
    }

    fn arrows(sheet: &crate::diagram::sheet::Sheet) -> usize {
        sheet
            .marks
            .iter()
            .filter(|m| matches!(m, Mark::Polygon { .. }))
            .count()
    }

    /// The transform example: one input, its expression, its output, one
    /// arrow into the expression and one out of it.
    #[test]
    fn a_transform_is_inputs_then_expression_then_outputs() {
        let sheet = picture(&kind("transform"));
        let words = sheet.words();
        for expected in ["transform: dataflow", "raw", "uint16", "celsius", "float64"] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert!(words.iter().any(|w| w.contains("raw * 0.1")), "{words:?}");
        assert_eq!(arrows(&sheet), 2);
        let korean = dataflow(&kind("transform"), &KO, page()).expect("draws");
        assert_ne!(korean[0].sheet.words(), words, "in the page's language");
    }

    /// An arrow joins an input to an expression only when the expression
    /// reads it: a second input the expression never names has none.
    #[test]
    fn an_input_nothing_reads_has_no_arrow() {
        let ForgeDocument::Transform(mut m) = kind("transform") else {
            panic!("a transform")
        };
        let mut extra = m.inputs[0].clone();
        extra.id = "unused".into();
        m.inputs.push(extra);
        let sheet = picture(&ForgeDocument::Transform(m));
        assert!(sheet.words().contains(&"unused"));
        assert_eq!(arrows(&sheet), 2, "the unused input draws no arrow");
    }

    /// What is read is what the parsed tree reads: a member after a dot is
    /// not an input, and a string's contents are not names.
    #[test]
    fn reads_are_the_trees_identifiers_not_the_texts_words() {
        let ForgeDocument::Transform(m) = kind("transform") else {
            panic!("a transform")
        };
        let mut inputs = m.inputs.clone();
        for id in ["a", "b", "len"] {
            let mut f = inputs[0].clone();
            f.id = id.into();
            inputs.push(f);
        }
        assert_eq!(reads("a.len + 1", &inputs), Some(vec![1]));
        assert_eq!(reads("'b' + a", &inputs), Some(vec![1]));
        assert_eq!(reads("len(a)", &inputs), Some(vec![1, 3]));
        assert_eq!(
            reads("a +", &inputs),
            None,
            "no reading for text that does not parse"
        );
    }

    /// A condition ends in a box that names it; a filter's middle box is
    /// the document's own settings.
    #[test]
    fn a_condition_and_a_filter_are_drawn_from_their_own_fields() {
        let sheet = picture(&kind("condition"));
        let words = sheet.words();
        for expected in ["doorClosed", "temperatureOk", "condition"] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert_eq!(arrows(&sheet), 3, "two inputs read, one output given");

        let sheet = picture(&kind("filter"));
        let words = sheet.words();
        for expected in ["rawButton", "stable", "filter_type  debounce", "window  3"] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert_eq!(arrows(&sheet), 2);
    }

    /// A validator draws each of its rules as a box, joined to the input
    /// it checks; the plausibility rule to every input it reads.
    #[test]
    fn a_validator_joins_each_rule_to_what_it_checks() {
        let sheet = picture(&kind("validator"));
        let words = sheet.words();
        for expected in [
            "speed",
            "motorState",
            "range",
            "0 .. 8000",
            "max change",
            "500 every 100 ms",
            "plausibility",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        // range -> speed, rate -> speed, plausibility -> speed and motorState.
        assert_eq!(arrows(&sheet), 4);
    }

    /// Every flow kind among the tree's fixtures is drawn or left to its
    /// table; none is refused, and every input is on the sheet.
    #[test]
    fn every_fixture_of_these_kinds_is_drawn_or_left_to_its_table() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/forge/resources");
        let mut drawn = 0;
        for entry in std::fs::read_dir(&dir).expect("fixtures") {
            let path = entry.expect("entry").path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !name.ends_with(".scxml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("reads");
            let Ok(Some(parsed)) = parse_forge_with_imports(
                &text,
                DocumentLabel::for_input_path(path.to_str().unwrap()),
            ) else {
                continue;
            };
            let inputs: Vec<String> = match &parsed.document {
                ForgeDocument::Transform(m) => m.inputs.iter().map(|f| f.id.clone()).collect(),
                ForgeDocument::Condition(m) => m.inputs.iter().map(|f| f.id.clone()).collect(),
                ForgeDocument::Validator(m) => m.inputs.iter().map(|f| f.id.clone()).collect(),
                ForgeDocument::Filter(m) => vec![m.input.id.clone()],
                _ => continue,
            };
            let pictures =
                dataflow(&parsed.document, &EN, page()).unwrap_or_else(|e| panic!("{name}: {e}"));
            let Some(p) = pictures.first() else { continue };
            drawn += 1;
            let words = p.sheet.words();
            for id in inputs {
                assert!(words.contains(&id.as_str()), "{name}: {id} in {words:?}");
            }
        }
        assert!(drawn >= 10, "the flow fixtures are drawn: {drawn}");
    }
}
