// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the store tests share.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sce_app_core::{FigureRenderer, FigureRequest, FigureSet, RenderError, Sheet};

static COUNTER: AtomicU64 = AtomicU64::new(0);

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
