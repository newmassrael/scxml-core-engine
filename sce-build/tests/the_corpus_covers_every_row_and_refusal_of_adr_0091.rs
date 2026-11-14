// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! ADR 0091 §7: the corpus covers every row of its §2 and every refusal of its
//! §3 (docs/SCE_TTCN3_LANE.md §12, §13.1 S2). Each row of §2 is held here by
//! what it holds, each aspect by the test cases of the corpus that run it, and
//! each refusal of §3 by the fixtures that are refused for it.
//!
//! A run is a test case of a suite of the corpus with a committed trace and the
//! expected log of that trace (§10.2, §12), which every backend's runner is held
//! to. A refusal is covered by a fixture the parser's tests hold to its
//! diagnostic, or by the reason, recorded in the lane's design, why no document
//! reaches it. The rows and the refusals are read from the ADR itself, so a row
//! or a refusal the ADR gains is uncovered until this table names its cover.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::lane_corpus::{read, sce_build, test_cases, CORPUS};

/// A test case of a suite of the corpus, by the suite's unit name and the test
/// case's name.
struct Run {
    suite: &'static str,
    case: &'static str,
}

/// What a row holds that a run shows, and the runs that show it.
struct Aspect {
    what: &'static str,
    runs: &'static [Run],
}

/// A row of ADR 0091 §2, by its concept as the table's first column writes it.
struct Row {
    concept: &'static str,
    aspects: &'static [Aspect],
}

/// How a refusal of ADR 0091 §3 is covered.
enum Cover {
    /// Fixtures, relative to `tests/fixtures/test_profile`, each refused for it.
    Fixtures(&'static [&'static str]),
    /// No document reaches it, for the reason the lane's design records under the
    /// given text.
    Recorded(&'static str),
}

/// A refusal of ADR 0091 §3, by the words its bullet starts with.
struct Refusal {
    starts: &'static str,
    cover: Cover,
}

const fn run(suite: &'static str, case: &'static str) -> Run {
    Run { suite, case }
}

const RUNTIME: &str = "runtime_suite";
const BOUNDARY: &str = "boundary_suite";

/// The rows of ADR 0091 §2 and the runs of each aspect.
const ROWS: &[Row] = &[
    Row {
        concept: "Type definitions",
        aspects: &[
            Aspect {
                what: "a record of unsigned integers",
                runs: &[run(RUNTIME, "answers_the_job")],
            },
            Aspect {
                what: "an enumerated type",
                runs: &[],
            },
            Aspect {
                what: "a union",
                runs: &[],
            },
            Aspect {
                what: "an optional field",
                runs: &[],
            },
            Aspect {
                what: "a text",
                runs: &[],
            },
            Aspect {
                what: "a byte string",
                runs: &[],
            },
            Aspect {
                what: "a list",
                runs: &[],
            },
            Aspect {
                what: "a record within a record",
                runs: &[],
            },
            Aspect {
                what: "a signed integer",
                runs: &[run(BOUNDARY, "exchanges_signed_numbers")],
            },
            Aspect {
                what: "a 64-bit integer",
                runs: &[run(BOUNDARY, "exchanges_signed_numbers")],
            },
        ],
    },
    Row {
        concept: "Template",
        aspects: &[
            Aspect {
                what: "a specific value",
                runs: &[run(RUNTIME, "answers_the_job")],
            },
            Aspect {
                what: "`?`",
                runs: &[run(RUNTIME, "serves_a_ptc")],
            },
            Aspect {
                what: "`*`",
                runs: &[],
            },
            Aspect {
                what: "a range",
                runs: &[],
            },
            Aspect {
                what: "a length restriction",
                runs: &[],
            },
            Aspect {
                what: "`omit`",
                runs: &[],
            },
            Aspect {
                what: "`ifpresent`",
                runs: &[],
            },
            Aspect {
                what: "a value list",
                runs: &[],
            },
            Aspect {
                what: "a complement",
                runs: &[],
            },
            Aspect {
                what: "a parameterised template",
                runs: &[run(RUNTIME, "answers_the_job")],
            },
            Aspect {
                what: "a template reference",
                runs: &[],
            },
            Aspect {
                what: "union matching",
                runs: &[],
            },
        ],
    },
    Row {
        concept: "Port type",
        aspects: &[Aspect {
            what: "a port type of messages in and out",
            runs: &[run(RUNTIME, "answers_the_job")],
        }],
    },
    Row {
        concept: "Component type",
        aspects: &[Aspect {
            what: "a component with ports, timers and variables",
            runs: &[run(RUNTIME, "answers_the_job")],
        }],
    },
    Row {
        concept: "Test configuration",
        aspects: &[
            Aspect {
                what: "`map`",
                runs: &[run(RUNTIME, "answers_the_job")],
            },
            Aspect {
                what: "`connect`",
                runs: &[run(RUNTIME, "serves_a_ptc")],
            },
        ],
    },
    Row {
        concept: "Test case",
        aspects: &[Aspect {
            what: "a test case on its configuration",
            runs: &[run(RUNTIME, "answers_the_job")],
        }],
    },
    Row {
        concept: "Module parameters (PICS, PIXIT)",
        aspects: &[
            Aspect {
                what: "a feature",
                runs: &[run(RUNTIME, "serves_a_ptc")],
            },
            Aspect {
                what: "a module parameter",
                runs: &[run(RUNTIME, "answers_the_job")],
            },
        ],
    },
    Row {
        concept: "Control part",
        aspects: &[
            Aspect {
                what: "the order of the test cases",
                runs: &[
                    run(RUNTIME, "answers_the_job"),
                    run(RUNTIME, "serves_a_ptc"),
                ],
            },
            Aspect {
                what: "selection by `all`",
                runs: &[run(RUNTIME, "keeps_a_ptc")],
            },
            Aspect {
                what: "selection by `any`",
                runs: &[run(RUNTIME, "answers_before_the_guard")],
            },
            Aspect {
                what: "a guard",
                runs: &[run(RUNTIME, "answers_before_the_guard")],
            },
        ],
    },
    Row {
        concept: "`setverdict`",
        aspects: &[
            Aspect {
                what: "`pass`",
                runs: &[run(RUNTIME, "answers_the_job")],
            },
            Aspect {
                what: "`inconclusive`",
                runs: &[run(RUNTIME, "keeps_a_ptc")],
            },
            Aspect {
                what: "`fail`",
                runs: &[run(BOUNDARY, "answers_another_job")],
            },
            Aspect {
                what: "`none`",
                runs: &[run(BOUNDARY, "sets_no_verdict")],
            },
        ],
    },
    Row {
        concept: "`alt` with receive branches",
        aspects: &[Aspect {
            what: "a receive branch beside a timeout branch",
            runs: &[run(RUNTIME, "answers_the_job")],
        }],
    },
    Row {
        concept: "`timer`",
        aspects: &[
            Aspect {
                what: "start, stop and timeout",
                runs: &[run(RUNTIME, "answers_the_job"), run(RUNTIME, "keeps_a_ptc")],
            },
            Aspect {
                what: "a default duration",
                runs: &[run(RUNTIME, "answers_the_job")],
            },
            Aspect {
                what: "a restart",
                runs: &[],
            },
            Aspect {
                what: "a duration given at the start",
                runs: &[],
            },
        ],
    },
    Row {
        concept: "`altstep` and `activate`",
        aspects: &[Aspect {
            what: "a branch of an ancestor state",
            runs: &[],
        }],
    },
    Row {
        concept: "`send`",
        aspects: &[Aspect {
            what: "a send of a specific template",
            runs: &[run(RUNTIME, "answers_the_job")],
        }],
    },
    Row {
        concept: "PTC `create`, `start`, `done`, `kill`",
        aspects: &[
            Aspect {
                what: "`create` and `start`",
                runs: &[run(RUNTIME, "serves_a_ptc")],
            },
            Aspect {
                what: "`done`",
                runs: &[run(RUNTIME, "serves_a_ptc")],
            },
            Aspect {
                what: "`kill`",
                runs: &[run(RUNTIME, "keeps_a_ptc")],
            },
        ],
    },
    Row {
        concept: "`log`",
        aspects: &[Aspect {
            what: "a log line",
            runs: &[run(RUNTIME, "serves_a_ptc")],
        }],
    },
];

/// The ways the runtime sets `error` (docs/SCE_TEST_PROFILE.md §5.2, P13) that
/// a document can lead a run to, and the runs that reach each.
const ERRORS: &[Aspect] = &[
    Aspect {
        what: "the guard",
        runs: &[run(RUNTIME, "answers_before_the_guard")],
    },
    Aspect {
        what: "a test case that can never end",
        runs: &[run(BOUNDARY, "never_ends")],
    },
    Aspect {
        what: "a dynamic error",
        runs: &[run(BOUNDARY, "sends_to_a_removed_ptc")],
    },
];

/// The aspects no run of the corpus shows yet, `<row>: <aspect>` each: the
/// suites that run them are made runnable after this table (§12.4).
const NOT_YET_RUN: &[&str] = &[
    "Type definitions: an enumerated type",
    "Type definitions: a union",
    "Type definitions: an optional field",
    "Type definitions: a text",
    "Type definitions: a byte string",
    "Type definitions: a list",
    "Type definitions: a record within a record",
    "Template: `*`",
    "Template: a range",
    "Template: a length restriction",
    "Template: `omit`",
    "Template: `ifpresent`",
    "Template: a value list",
    "Template: a complement",
    "Template: a template reference",
    "Template: union matching",
    "`timer`: a restart",
    "`timer`: a duration given at the start",
    "`altstep` and `activate`: a branch of an ancestor state",
];

/// The refusals of ADR 0091 §3 and how each is covered.
const REFUSALS: &[Refusal] = &[
    Refusal {
        starts: "any native expression",
        cover: Cover::Fixtures(&["refused_component_native_condition.scxml"]),
    },
    Refusal {
        starts: "any construct that raises a script-engine cause",
        cover: Cover::Recorded("**none**: no test component reaches `test-profile/script-engine`"),
    },
    Refusal {
        starts: "a profile document whose `datamodel` is not `sce-static`",
        cover: Cover::Fixtures(&[
            "refused_component_datamodel.scxml",
            "refused_port_type_datamodel.scxml",
        ]),
    },
    Refusal {
        starts: "a `cond` that reads a received message",
        cover: Cover::Fixtures(&["refused_component_cond_reads_message.scxml"]),
    },
    Refusal {
        starts: "`<parallel>` inside a `test-component`",
        cover: Cover::Fixtures(&["refused_component_parallel.scxml"]),
    },
    Refusal {
        starts: "`<history>`",
        cover: Cover::Fixtures(&["refused_component_history.scxml"]),
    },
    Refusal {
        starts: "`<script>`",
        cover: Cover::Fixtures(&["refused_component_script.scxml"]),
    },
    Refusal {
        starts: "`<donedata>` used as a verdict carrier",
        cover: Cover::Fixtures(&["refused_component_donedata.scxml"]),
    },
    Refusal {
        starts: "a receive branch whose template is not one of the port's declared incoming types",
        cover: Cover::Fixtures(&["refused_component_receive_not_in.scxml"]),
    },
    Refusal {
        starts: "a test case selected by a feature its suite's `feature-model` does not declare",
        cover: Cover::Fixtures(&["assembly/refused_suite_feature_undeclared.scxml"]),
    },
    Refusal {
        starts: "`<raise>` and a `<send>` to `#_internal`",
        cover: Cover::Fixtures(&[
            "refused_component_raise.scxml",
            "refused_component_send_internal.scxml",
        ]),
    },
    Refusal {
        starts: "a `<send>` whose target is not one of the component's ports",
        cover: Cover::Fixtures(&["refused_component_send_unknown_port.scxml"]),
    },
    Refusal {
        starts: "a `<send>` with a delay, and `<cancel>`",
        cover: Cover::Fixtures(&[
            "refused_component_send_delay.scxml",
            "refused_component_cancel.scxml",
        ]),
    },
    Refusal {
        starts: "an `<invoke>` of anything but a test component",
        cover: Cover::Fixtures(&[
            "refused_component_invoke_not_component.scxml",
            "refused_component_invoke_type.scxml",
        ]),
    },
    Refusal {
        starts: "a transition that is not one branch kind",
        cover: Cover::Fixtures(&[
            "refused_component_branch_event.scxml",
            "refused_component_branch_two_kinds.scxml",
        ]),
    },
    Refusal {
        starts: "a done branch whose target keeps the invoking state active",
        cover: Cover::Fixtures(&["refused_component_done_branch_stays.scxml"]),
    },
];

/// The tests that hold each refused fixture to its diagnostic: a fixture one of
/// them names is read by it and held to its code.
const HOLDING_TESTS: &[&str] = &[
    "tests/a_test_profile_document_is_held_to_what_it_states.rs",
    "tests/a_test_is_held_to_the_documents_it_imports.rs",
];

fn adr() -> String {
    read(&sce_build().join(
        "../docs/adr/0091-a-test-is-a-document-of-the-test-profile-and-its-ttcn-3-export-runs-and-agrees-with-it.md",
    ))
}

/// The lines of `text` from the heading `from` to the next heading of the same
/// level or above.
fn section<'t>(text: &'t str, from: &str) -> Vec<&'t str> {
    let mut lines = text.lines().skip_while(|line| *line != from);
    assert!(lines.next().is_some(), "the ADR has the heading {from}");
    lines
        .take_while(|line| !line.starts_with("## ") && !line.starts_with("### "))
        .collect()
}

/// The concepts of the table of ADR 0091 §2, in its order.
fn adr_rows(adr: &str) -> Vec<String> {
    section(adr, "### 2. What the profile contains")
        .into_iter()
        .filter(|line| line.starts_with("| "))
        .skip(1)
        .map(|line| {
            line.split('|')
                .nth(1)
                .expect("a row has a first cell")
                .trim()
                .to_string()
        })
        .collect()
}

/// The refusals of ADR 0091 §3, each the first line of its bullet, nested ones
/// included, but a bullet that only introduces the ones nested in it.
fn adr_refusals(adr: &str) -> Vec<String> {
    section(adr, "### 3. What the profile refuses")
        .into_iter()
        .filter_map(|line| {
            line.strip_prefix("- ")
                .or_else(|| line.strip_prefix("  - "))
        })
        .filter(|bullet| !bullet.ends_with(':'))
        .map(str::to_string)
        .collect()
}

/// Why `run` is no run of the corpus; `None` when it is one.
fn not_a_run(run: &Run) -> Option<String> {
    let Some(suite) = CORPUS.iter().find(|suite| suite.name() == run.suite) else {
        return Some(format!("{} is no suite of the corpus", run.suite));
    };
    if !test_cases(&suite.path())
        .iter()
        .any(|(name, _)| name == run.case)
    {
        return Some(format!("{} executes no test case {}", run.suite, run.case));
    }
    let file = format!("{}.json", run.case);
    for (what, dir) in [
        ("trace", suite.traces()),
        ("expected log", suite.expected_logs()),
    ] {
        if !dir.join(&file).exists() {
            return Some(format!(
                "{}'s test case {} has no {what} {}",
                run.suite,
                run.case,
                dir.join(&file).display()
            ));
        }
    }
    None
}

/// Every aspect of the table, `<row>: <aspect>` each, with its runs.
fn aspects() -> Vec<(String, &'static [Run])> {
    ROWS.iter()
        .flat_map(|row| {
            row.aspects
                .iter()
                .map(move |aspect| (format!("{}: {}", row.concept, aspect.what), aspect.runs))
        })
        .chain(
            ERRORS
                .iter()
                .map(|aspect| (format!("error: {}", aspect.what), aspect.runs)),
        )
        .collect()
}

/// The rows of the table are the rows of ADR 0091 §2, in its order.
#[test]
fn the_table_holds_every_row_of_the_adr() {
    let ours: Vec<&str> = ROWS.iter().map(|row| row.concept).collect();
    assert_eq!(ours, adr_rows(&adr()));
}

/// Each aspect of a row is shown by runs of the corpus, each a test case of a
/// suite of the corpus with its trace and its expected log, but the aspects
/// NOT_YET_RUN names, which no run shows.
#[test]
fn every_row_is_run_by_the_corpus() {
    let mut wrong = Vec::new();
    let mut unrun = BTreeSet::new();
    for (aspect, runs) in aspects() {
        if runs.is_empty() {
            unrun.insert(aspect.clone());
        }
        for run in runs {
            if let Some(why) = not_a_run(run) {
                wrong.push(format!("{aspect}: {why}"));
            }
        }
    }
    let named: BTreeSet<String> = NOT_YET_RUN
        .iter()
        .map(|aspect| aspect.to_string())
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(
        unrun, named,
        "the aspects no run shows are the ones NOT_YET_RUN names"
    );
}

/// Each refusal of ADR 0091 §3 has its cover: the refusal's bullet starts with
/// the words of exactly one entry of the table, and every entry's words start a
/// bullet; each fixture is committed and held to its diagnostic by a test of the
/// parser, and a recorded reason stands in the lane's design.
#[test]
fn every_refusal_is_covered() {
    let refusals = adr_refusals(&adr());
    let mut wrong = Vec::new();
    for bullet in &refusals {
        let matched = REFUSALS
            .iter()
            .filter(|refusal| bullet.starts_with(refusal.starts))
            .count();
        if matched != 1 {
            wrong.push(format!(
                "the refusal \"{bullet}\" starts {matched} entries of the table"
            ));
        }
    }
    let holding: String = HOLDING_TESTS
        .iter()
        .map(|test| read(&sce_build().join(test)))
        .collect();
    let lane = read(&sce_build().join("../docs/SCE_TTCN3_LANE.md"));
    let fixtures = sce_build().join("tests/fixtures/test_profile");
    for refusal in REFUSALS {
        if !refusals
            .iter()
            .any(|bullet| bullet.starts_with(refusal.starts))
        {
            wrong.push(format!(
                "\"{}\" starts no refusal of the ADR",
                refusal.starts
            ));
        }
        match refusal.cover {
            Cover::Fixtures(names) => {
                assert!(!names.is_empty(), "{} names its fixtures", refusal.starts);
                for name in names {
                    if !fixtures.join(name).exists() {
                        wrong.push(format!("{}: no fixture {name}", refusal.starts));
                    }
                    let file = Path::new(name)
                        .file_name()
                        .expect("a fixture is a file")
                        .to_string_lossy()
                        .into_owned();
                    if !holding.contains(&format!("\"{file}\"")) {
                        wrong.push(format!(
                            "{}: no test of the parser holds {file} to its diagnostic",
                            refusal.starts
                        ));
                    }
                }
            }
            Cover::Recorded(reason) => {
                if !lane.contains(reason) {
                    wrong.push(format!(
                        "{}: the lane's design records no reason \"{reason}\"",
                        refusal.starts
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The table's reading of the ADR is the one its tests rely on: the first row
/// and the last, and the refusal that only introduces its nested ones is not
/// one of its own.
#[test]
fn the_adr_is_read_as_its_table_and_its_bullets() {
    let adr = adr();
    let rows = adr_rows(&adr);
    assert_eq!(rows.first().map(String::as_str), Some("Type definitions"));
    assert_eq!(rows.last().map(String::as_str), Some("`log`"));
    let refusals = adr_refusals(&adr);
    assert!(
        refusals
            .iter()
            .all(|bullet| !bullet.starts_with("the constructs the operational semantics")),
        "{refusals:?}"
    );
    assert!(
        refusals
            .iter()
            .any(|bullet| bullet.starts_with("a done branch whose target keeps")),
        "a nested refusal is read: {refusals:?}"
    );
}
