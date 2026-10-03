// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the store tests share.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sce_app_core::{
    Check, FigureRenderer, FigureRequest, FigureSet, ModelReviewer, PageRefusal, Record,
    RenderError, Review, ReviewRequest, Sheet, Unresolved, Verdict,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// What the stand-in product says of a model, by what the model says: one that
/// contains `REFUSE` is refused with a record, one that contains `NOPAGE` is
/// accepted and its page refused, and any other is accepted, leaves one question
/// open, and has a page that says what it was asked.
fn review_of(request: &ReviewRequest<'_>) -> Review {
    let generator = Some("fake-sce 0".to_string());
    if request.model.contains("REFUSE") {
        return Review {
            generator,
            check: Check {
                verdict: Verdict::Refused,
                kind: None,
                open: Vec::new(),
                unresolved: Vec::new(),
                records: vec![Record {
                    code: "validation/invalid-reference".to_string(),
                    message: "Transition in state 'closed' references non-existent target state 'nowhere'"
                        .to_string(),
                    stage: Some("validation".to_string()),
                    line: Some(3),
                    fix: None,
                }],
            },
            page: None,
            page_refusal: None,
        };
    }
    let page_refused = request.model.contains("NOPAGE");
    Review {
        generator,
        check: Check {
            verdict: Verdict::Accepted,
            kind: Some("statechart".to_string()),
            open: vec!["1 question(s) the specification leaves open (open-guard)".to_string()],
            unresolved: vec![Unresolved {
                id: "open-guard".to_string(),
                node_path: "states.closed.transitions[0]".to_string(),
                line: Some(3),
                reason: Some("Which card values open the door?".to_string()),
            }],
            records: Vec::new(),
        },
        page: (!page_refused).then(|| {
            format!(
                "machine {} (lexicon: {})\n  model: {}\n",
                request.name.unwrap_or("-"),
                request.lexicon.unwrap_or("-"),
                request.model.chars().take(40).collect::<String>(),
            )
        }),
        page_refusal: page_refused.then(|| PageRefusal {
            code: "cli/pseudo-unsupported".to_string(),
            message: "the page does not abbreviate this construct".to_string(),
        }),
    }
}

impl ModelReviewer for FakeRenderer {
    fn review(&self, request: &ReviewRequest<'_>) -> Result<Review, RenderError> {
        Ok(review_of(request))
    }
}

impl ModelReviewer for RefusingRenderer {
    fn review(&self, _: &ReviewRequest<'_>) -> Result<Review, RenderError> {
        Err(RenderError::TimedOut { seconds: 30 })
    }
}

/// A generator that draws nothing real: one picture and one table whose text
/// says what it was asked, so a test sees that the model and the options
/// reached the renderer and that the sheets come back in order.
pub struct FakeRenderer;

impl FigureRenderer for FakeRenderer {
    fn render(&self, request: &FigureRequest<'_>) -> Result<FigureSet, RenderError> {
        let asked = format!(
            "{} | name={} page={} lexicon={} min_pt={}",
            request.model.chars().take(40).collect::<String>(),
            request.name.unwrap_or("-"),
            request.page.unwrap_or("-"),
            request.lexicon.unwrap_or("-"),
            request.min_pt.map_or("-".to_string(), |p| p.to_string()),
        );
        Ok(FigureSet {
            generator: Some("fake-sce 0".to_string()),
            sheets: vec![
                Sheet {
                    name: "picture.svg".to_string(),
                    svg: format!(
                        "<svg xmlns=\"http://www.w3.org/2000/svg\"><text>{asked}</text></svg>\n"
                    ),
                },
                Sheet {
                    name: "fields-1.svg".to_string(),
                    svg: "<svg xmlns=\"http://www.w3.org/2000/svg\"><text>fields</text></svg>\n"
                        .to_string(),
                },
            ],
        })
    }
}

/// A generator that says no, as the product does for a model it will not draw.
pub struct RefusingRenderer;

impl FigureRenderer for RefusingRenderer {
    fn render(&self, _: &FigureRequest<'_>) -> Result<FigureSet, RenderError> {
        Err(RenderError::Refused {
            code: "cli/diagram-does-not-fit".to_string(),
            message: "the figure needs 925 x 125 pt and the page gives 510 x 757 pt".to_string(),
        })
    }
}

/// A fresh, empty folder under the build's temporary directory, named for the
/// test that asked and unique to this run.
pub fn scratch(label: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "app-core-{label}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the scratch folder");
    dir
}
