// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the store tests share.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sce_app_core::{
    Acceptor, Check, CheckOutcome, FigureRenderer, FigureRequest, FigureSet, ModelReviewer,
    PageRefusal, Record, RenderError, RequirementOutcome, RequirementsReport, Review,
    ReviewRequest, Revision, Sheet, Snapshot, Taken, Unresolved, Verdict,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Every file of a work as the product reads it, by the path the layout gives it and a
/// digest of its bytes: what a real record pins, in the form a stand-in can compare.
fn pinned(snapshot: &Snapshot) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = snapshot
        .model
        .documents()
        .into_iter()
        .map(|d| {
            (
                format!("design/{}", d.name),
                Revision::of(d.text.as_bytes()).to_string(),
            )
        })
        .collect();
    files.push((
        "spec/requirements.manifest.json".to_string(),
        Revision::of(snapshot.requirements.manifest.as_bytes()).to_string(),
    ));
    files.push((
        "spec/source.txt".to_string(),
        Revision::of(snapshot.source.as_bytes()).to_string(),
    ));
    if let Some(answers) = &snapshot.answers {
        files.push((
            "spec/answers.json".to_string(),
            Revision::of(answers.as_bytes()).to_string(),
        ));
    }
    files
}

/// A stand-in for what the product says of requirements and acceptance, built to
/// behave as the product does where the application depends on it: the requirement
/// list names ids and each is `implemented` unless the entry document says `MISSING`;
/// a record pins every file by path and digest; a check names the paths that moved.
impl Acceptor for FakeRenderer {
    fn report_requirements(&self, snapshot: &Snapshot) -> Result<RequirementsReport, RenderError> {
        let manifest: serde_json::Value =
            serde_json::from_str(&snapshot.requirements.manifest).expect("a manifest is JSON");
        let ids: Vec<String> = manifest["requirements"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|r| r["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let missing = snapshot.model.entry_text().contains("MISSING");
        let outcomes = ids
            .iter()
            .enumerate()
            .map(|(i, id)| RequirementOutcome {
                id: id.clone(),
                outcome: if missing && i == 0 {
                    "missing"
                } else {
                    "implemented"
                }
                .to_string(),
                section: Some(format!("S{}", i + 1)),
                node_paths: if missing && i == 0 {
                    Vec::new()
                } else {
                    vec![format!("states.s{i}")]
                },
            })
            .collect();
        Ok(RequirementsReport {
            generator: Some("fake-sce 0".to_string()),
            denominator: Some("synthesized".to_string()),
            outcomes,
            page: Some(format!(
                "ACCEPTANCE REPORT of {} requirement(s)\n",
                ids.len()
            )),
            page_refusal: None,
        })
    }

    fn take_acceptance(&self, snapshot: &Snapshot) -> Result<Taken, RenderError> {
        if snapshot.model.entry_text().contains("UNACCEPTABLE") {
            return Err(RenderError::Refused {
                code: "xml/parse-error".to_string(),
                message: "the design cannot be read".to_string(),
            });
        }
        let record = serde_json::json!({
            "record": "fake-acceptance",
            "channel": "direct",
            "pins": pinned(snapshot),
        })
        .to_string();
        let open = if snapshot.model.entry_text().contains("OPEN") {
            vec!["1 question(s) the specification leaves open (open-guard)".to_string()]
        } else {
            Vec::new()
        };
        Ok(Taken {
            generator: Some("fake-sce 0".to_string()),
            record,
            open,
        })
    }

    fn check_acceptance(
        &self,
        snapshot: &Snapshot,
        record: &str,
    ) -> Result<CheckOutcome, RenderError> {
        let wire: serde_json::Value = serde_json::from_str(record).expect("a fake record");
        let then: Vec<(String, String)> = serde_json::from_value(wire["pins"].clone()).unwrap();
        let now = pinned(snapshot);
        let mut lapses = Vec::new();
        for (path, digest) in &then {
            match now.iter().find(|(p, _)| p == path) {
                Some((_, d)) if d == digest => {}
                Some(_) => lapses.push(format!("{path} moved")),
                None => lapses.push(format!("{path} is gone")),
            }
        }
        for (path, _) in &now {
            if !then.iter().any(|(p, _)| p == path) {
                lapses.push(format!("{path} was not there when it was accepted"));
            }
        }
        // One sentence, as the product writes it: its lapses joined by `; `.
        Ok(if lapses.is_empty() {
            CheckOutcome::Holds
        } else {
            CheckOutcome::Lapsed {
                says: lapses.join("; "),
            }
        })
    }
}

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

/// The product not answering at all, for each of the three questions.
impl Acceptor for RefusingRenderer {
    fn report_requirements(&self, _: &Snapshot) -> Result<RequirementsReport, RenderError> {
        Err(RenderError::TimedOut { seconds: 30 })
    }

    fn take_acceptance(&self, _: &Snapshot) -> Result<Taken, RenderError> {
        Err(RenderError::TimedOut { seconds: 30 })
    }

    fn check_acceptance(&self, _: &Snapshot, _: &str) -> Result<CheckOutcome, RenderError> {
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
            "{} | name={} entry_file={} siblings={} page={} lexicon={} min_pt={}",
            request.model.chars().take(40).collect::<String>(),
            request.name.unwrap_or("-"),
            request.entry_file.unwrap_or("-"),
            request
                .siblings
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>()
                .join(","),
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

/// Run `save` while nobody can add files to the work's folder, so a save writes the
/// revision's file and its log line and cannot create the pointer's temporary file:
/// the failure itself, not a log written to look like it. `None` when this user is not
/// bound by the permission (root), which cannot show it -- said, not passed.
#[cfg(unix)]
pub fn while_the_pointer_cannot_move<T>(
    work_dir: &std::path::Path,
    save: impl FnOnce() -> T,
) -> Option<T> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(work_dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let probe = work_dir.join(".probe");
    if std::fs::File::create(&probe).is_ok() {
        let _ = std::fs::remove_file(&probe);
        std::fs::set_permissions(work_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        eprintln!("not run: this user can write into a read-only folder");
        return None;
    }
    let outcome = save();
    std::fs::set_permissions(work_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    Some(outcome)
}

/// Make a work's folder read as an OLDER BUILD left it: each pointer is its digest alone,
/// one line, and no log line names the place of the save it followed. That is the whole
/// of the difference (`store.rs` documents the format), so a build that reads this reads
/// what a build from before places were kept wrote -- without keeping a second binary.
pub fn as_an_older_build_wrote_it(work_dir: &std::path::Path) {
    for stem in ["source", "model", "answers", "requirements", "acceptances"] {
        let head = work_dir.join(format!("{stem}.head"));
        if let Ok(text) = std::fs::read_to_string(&head) {
            let digest = text.lines().next().expect("a digest").trim();
            std::fs::write(&head, format!("{digest}\n")).unwrap();
        }
        let log = work_dir.join(format!("{stem}.log"));
        if let Ok(text) = std::fs::read_to_string(&log) {
            let mut older = String::new();
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let mut record: serde_json::Value = serde_json::from_str(line).unwrap();
                record.as_object_mut().unwrap().remove("parent_at");
                older.push_str(&record.to_string());
                older.push('\n');
            }
            std::fs::write(&log, older).unwrap();
        }
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
