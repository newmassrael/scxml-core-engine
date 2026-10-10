// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A link and a worker as what they are joined to.
//!
//! Neither kind computes anything; each is defined by the other documents
//! it names — a link by the codec that frames its messages and the pools it
//! stages them in, a worker by the link it reads from. So the picture is the
//! document in the middle and the documents it names around it, each
//! joined by a line labelled with the role the name plays (`framer`,
//! `stage_pool`, `link_rx`), and each box saying which file and kind that
//! name resolves to through the document's own `<sce:import>`.
//!
//! A name no import resolves is drawn all the same, marked as not
//! imported: a reference the document makes and cannot back is the thing
//! worth seeing. An import no role names is drawn too, with no line.

use super::{facts, say, Block, Picture};
use crate::diagram::canvas::Canvas;
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::metrics::Face;
use crate::diagram::sheet::Ink;
use crate::diagram::words::Phrase;
use crate::forge::model::{ForgeDocument, ForgeImport, ParsedForge};
use crate::forge::page::Lexicon;

/// The room between the document and what it names, besides the widest
/// role label, in body-text sizes.
const SPOKE_GAP_EM: f64 = 4.0;

/// The roles a document names others in, as `(role, name)`.
fn roles(doc: &ForgeDocument) -> Option<(String, Vec<(&'static str, String)>)> {
    match doc {
        ForgeDocument::Link(m) => {
            let named = [
                ("framer", Some(m.framer.clone())),
                ("rx_pool", m.rx_pool.clone()),
                ("tx_pool", m.tx_pool.clone()),
                ("stage_pool", m.stage_pool.clone()),
            ];
            Some((
                m.name.clone(),
                named
                    .into_iter()
                    .filter_map(|(role, name)| name.filter(|n| !n.is_empty()).map(|n| (role, n)))
                    .collect(),
            ))
        }
        ForgeDocument::Worker(m) => {
            let named = [
                ("link_rx", Some(m.link_rx.clone())),
                ("outbox", m.outbox.clone()),
            ];
            Some((
                m.name.clone(),
                named
                    .into_iter()
                    .filter_map(|(role, name)| name.filter(|n| !n.is_empty()).map(|n| (role, n)))
                    .collect(),
            ))
        }
        _ => None,
    }
}

/// What the document says about itself, one `key  value` line each: its
/// scalars, then what the scalars do not hold.
fn hub_lines(doc: &ForgeDocument) -> Result<Vec<String>, Refusal> {
    let role_fields = [
        "name",
        "framer",
        "rx_pool",
        "tx_pool",
        "stage_pool",
        "link_rx",
        "outbox",
    ];
    let mut lines: Vec<(String, String)> = match doc {
        ForgeDocument::Link(m) => {
            let mut lines = facts(m, &role_fields)?;
            lines.extend(
                m.inbound
                    .iter()
                    .map(|e| ("inbound".to_string(), e.event.clone())),
            );
            lines.extend(
                m.outbound
                    .iter()
                    .map(|e| ("outbound".to_string(), e.event.clone())),
            );
            lines
        }
        ForgeDocument::Worker(m) => {
            let mut lines = facts(m, &role_fields)?;
            lines.push(("inbox".into(), m.inbox.queue_ref.clone()));
            lines
        }
        _ => Vec::new(),
    };
    let key_w = lines
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    Ok(lines
        .drain(..)
        .map(|(k, v)| format!("{k:<key_w$}  {v}"))
        .collect())
}

fn spoke_lines(
    name: &str,
    imports: &[ForgeImport],
    lexicon: &Lexicon,
) -> Result<Vec<(String, Face)>, Refusal> {
    Ok(match imports.iter().find(|i| i.alias == name) {
        Some(import) => vec![
            (import.alias.clone(), Face::Mono),
            (import.kind.as_attr().to_string(), Face::Mono),
            (import.src.clone(), Face::Mono),
        ],
        None => vec![
            (name.to_string(), Face::Mono),
            (
                say(lexicon, Phrase::NotImported)?.to_string(),
                Face::Proportional,
            ),
        ],
    })
}

/// The structure picture of a link or a worker.
pub fn structure(
    parsed: &ParsedForge,
    lexicon: &Lexicon,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    let Some((name, roles)) = roles(&parsed.document) else {
        return Ok(Vec::new());
    };
    let mut c = Canvas::new(page);
    let body = c.style().body_pt;
    let line = c.line_height();
    let title = format!("{name}: {}", say(lexicon, Phrase::Structure)?);
    c.title(0.0, 0.0, &title)?;
    let top = c.style().title_pt * c.style().leading + line;

    // What is joined by a role, then what is imported and named by none.
    let mut spokes: Vec<(Option<&'static str>, Block)> = Vec::new();
    for (role, target) in &roles {
        spokes.push((
            Some(*role),
            Block::new(&c, spoke_lines(target, &parsed.imports, lexicon)?)?,
        ));
    }
    for import in &parsed.imports {
        if !roles.iter().any(|(_, n)| *n == import.alias) {
            spokes.push((
                None,
                Block::new(&c, spoke_lines(&import.alias, &parsed.imports, lexicon)?)?,
            ));
        }
    }
    let hub_text: Vec<(String, Face)> = hub_lines(&parsed.document)?
        .into_iter()
        .map(|l| (l, Face::Mono))
        .collect();
    let hub = Block::new(&c, hub_text)?;

    let mut label_w = 0.0f64;
    for (role, _) in &spokes {
        if let Some(role) = role {
            label_w = label_w.max(c.width_of(Face::Mono, role, body)?);
        }
    }
    let spacing = body * 1.2;
    let column_h = spokes.iter().map(|(_, b)| b.height).sum::<f64>()
        + spacing * spokes.len().saturating_sub(1) as f64;
    let height = column_h.max(hub.height);
    let hub_top = top + (height - hub.height) / 2.0;
    let spoke_x = hub.width + label_w + SPOKE_GAP_EM * body;

    let linked = spokes.iter().filter(|(r, _)| r.is_some()).count();
    let mut y = top + (height - column_h) / 2.0;
    let mut slot = 0usize;
    for (role, block) in &spokes {
        if let Some(role) = role {
            let from = (
                hub.width,
                hub_top + hub.height * (slot + 1) as f64 / (linked + 1) as f64,
            );
            let to = (spoke_x, y + block.height / 2.0);
            c.stroke(from, to, Ink::Black, false);
            let mid = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
            c.label_centered(mid.0, mid.1 - line / 2.0, role, Face::Mono)?;
            slot += 1;
        }
        block.draw(&mut c, spoke_x, y, Some(Ink::Shade))?;
        y += block.height + spacing;
    }
    // The document itself drawn last, over the ends of the lines.
    hub.draw(&mut c, 0.0, hub_top, None)?;

    Ok(vec![Picture {
        stem: "structure".into(),
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

    fn kind(name: &str) -> ParsedForge {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join(format!("{name}.scxml"));
        let text = std::fs::read_to_string(&path).expect("reads");
        parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
            .expect("parses")
            .expect("not a statechart")
    }

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    fn lines(sheet: &crate::diagram::sheet::Sheet) -> usize {
        sheet
            .marks
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    Mark::Stroke {
                        ink: Ink::Black,
                        ..
                    }
                )
            })
            .count()
    }

    /// The UDP endpoint: the document in the middle, the codec that frames
    /// it and the pool it stages in around it, each joined by its role and
    /// each saying the file and kind its name resolves to.
    #[test]
    fn a_link_is_joined_to_its_framer_and_its_pool() {
        let sheet = structure(&kind("link"), &EN, page())
            .expect("draws")
            .remove(0)
            .sheet;
        let words = sheet.words();
        for expected in [
            "link: structure",
            "framer",
            "stage_pool",
            "frame_codec",
            "codec",
            "codec.scxml",
            "rx_pool",
            "buffer-pool",
            "buffer-pool.scxml",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert!(
            words
                .iter()
                .any(|w| w.starts_with("class") && w.ends_with("udp")),
            "{words:?}"
        );
        assert!(
            words
                .iter()
                .any(|w| w.starts_with("backpressure") && w.ends_with("drop")),
            "{words:?}"
        );
        assert_eq!(lines(&sheet), 2, "one line for each role");
        let korean = structure(&kind("link"), &KO, page()).expect("draws");
        assert_ne!(korean[0].sheet.words(), words, "in the page's language");
    }

    /// A worker is joined to the link it reads, and says its inbox.
    #[test]
    fn a_worker_is_joined_to_the_link_it_reads() {
        let sheet = structure(&kind("worker"), &EN, page())
            .expect("draws")
            .remove(0)
            .sheet;
        let words = sheet.words();
        for expected in [
            "worker: structure",
            "link_rx",
            "udp_endpoint",
            "link",
            "link.scxml",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert!(
            words
                .iter()
                .any(|w| w.starts_with("inbox") && w.ends_with("rx_events")),
            "{words:?}"
        );
        assert_eq!(lines(&sheet), 1);
    }

    /// A name no import resolves is drawn as not imported, and an import
    /// no role names is drawn with no line to it.
    #[test]
    fn a_name_nothing_imports_and_an_import_nothing_names_are_both_drawn() {
        let mut parsed = kind("link");
        let unused = parsed.imports[1].clone();
        parsed.imports.retain(|i| i.alias != "rx_pool");
        let sheet = structure(&parsed, &EN, page())
            .expect("draws")
            .remove(0)
            .sheet;
        let words = sheet.words();
        assert!(
            words.contains(&"rx_pool") && words.contains(&"not imported"),
            "{words:?}"
        );
        assert_eq!(lines(&sheet), 2, "the unresolved name is still joined");

        let mut parsed = kind("link");
        let mut extra = unused;
        extra.alias = "spare".into();
        parsed.imports.push(extra);
        let sheet = structure(&parsed, &EN, page())
            .expect("draws")
            .remove(0)
            .sheet;
        assert!(sheet.words().contains(&"spare"));
        assert_eq!(lines(&sheet), 2, "and no line goes to it");
    }

    /// Other kinds have no structure to draw.
    #[test]
    fn a_document_of_another_kind_has_no_structure() {
        assert!(structure(&kind("timer"), &EN, page())
            .expect("draws")
            .is_empty());
    }
}
