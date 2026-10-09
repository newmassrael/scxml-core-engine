// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A model server as a generator: what it is asked, what it is offered, what is done with what it
//! answers, and how a run ends when the server is slow, silent, wrong or gone.
//!
//! The model server here is a thread that answers each request from a script and records what it
//! was sent, and the authoring server is a shell script that answers by method. Neither is a model:
//! what is held is the conversation the application has with them, which is the same whatever
//! server a person runs. That the conversation works with a real model is what the live test
//! (`local_live`, ignored unless asked for) holds.
//!
//! Unix only: the authoring server stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sce_app_core::claude_code::AuthorServer;
use sce_app_core::codex::AUTHOR_TOOLS;
use sce_app_core::http_client::Endpoint;
use sce_app_core::local::{list_models, Local, LocalConfig, ModelsError, Step};
use sce_app_core::runner::{Cancel, GenerateError, Generator, Job};
use sce_app_core::{Revision, WorkId};
use serde_json::{json, Value};

use common::model_server::{
    authoring_server, called, calls, chat_server, draft, requirement_set_gives,
    requirement_set_gives_at, says, scope_of, tool_gives, ChatServer, Script,
};
use common::scratch;

// ---- the rig ---------------------------------------------------------------------------------

/// An address nothing listens on. Not a port that was bound and let go of: tests run side by side,
/// and another test's server may be given that very port between the letting go and the asking
/// (measured: a test that was refused here was answered by another's server and reset). Port 1 is
/// below the range the system hands out, so no test can hold it, and nothing of this machine's
/// listens there.
const NOBODY: &str = "http://127.0.0.1:1/v1";

fn job() -> Job {
    Job {
        work: WorkId::parse("door-lock").unwrap(),
        title: "Door lock".to_string(),
        request: "req-7".to_string(),
        attempt: 1,
        source: "The lock opens when the code matches.".to_string(),
        source_revision: Revision::of(b"The lock opens when the code matches."),
        answers: Default::default(),
        previous: None,
        refusal: None,
        fresh_ids: false,
    }
}

struct Rig {
    server: ChatServer,
    folder: PathBuf,
}

impl Rig {
    fn new(label: &str, script: Vec<Script>) -> Rig {
        Rig {
            server: chat_server(script),
            folder: scratch(label),
        }
    }

    fn local(&self, config: LocalConfig) -> Local {
        self.local_with(config, &AUTHOR_TOOLS)
    }

    fn local_with(&self, config: LocalConfig, tools: &[&str]) -> Local {
        Local::new(
            Endpoint::parse(&self.server.address).unwrap(),
            authoring_server(&self.folder, tools),
            config,
        )
    }

    fn run(&self) -> Result<sce_app_core::runner::Draft, GenerateError> {
        self.local(LocalConfig::for_model("qwen-test"))
            .generate(&job(), &Cancel::new())
    }
}

/// The tools the stand-in has: the ones a client may use, and one that saves, which none may.
fn with_a_saving_tool() -> Vec<&'static str> {
    let mut tools = AUTHOR_TOOLS.to_vec();
    tools.push("works_save_model");
    tools
}

fn tool_message(messages: &[Value], at: usize) -> String {
    messages[at]["content"].as_str().unwrap().to_string()
}

// ---- the conversation ------------------------------------------------------------------------

#[test]
fn a_model_that_reads_the_work_and_then_answers_is_asked_twice_and_its_draft_is_taken() {
    let rig = Rig::new(
        "local-ok",
        vec![
            calls(&[(Some("call_1"), "works_read", "{\"work\":\"door-lock\"}")]),
            says(&draft("<scxml><!-- door --></scxml>")),
        ],
    );

    let made = rig.run().unwrap();

    assert_eq!(made.model.entry_text(), "<scxml><!-- door --></scxml>");
    assert!(made.requirements.manifest.contains("door"));
    let requests = rig.server.requests();
    assert_eq!(requests.len(), 2);
    // The path the address carries, and what is asked of the model.
    assert!(
        requests[0]
            .head
            .starts_with("POST /v1/chat/completions HTTP/1.1"),
        "{}",
        requests[0].head
    );
    assert_eq!(requests[0].body["model"], "qwen-test");
    assert_eq!(requests[0].body["stream"], false);
    let first = rig.server.messages(0);
    assert_eq!(first[0]["role"], "system");
    let system = first[0]["content"].as_str().unwrap();
    assert!(
        system.contains("Use the tools."),
        "the authoring server's own words: {system}"
    );
    assert!(system.contains("one JSON object"), "{system}");
    // The form of the answer is an example to fill in, and not a schema to say back.
    assert!(
        system.contains("<the whole SCXML document"),
        "the form of the answer: {system}"
    );
    assert!(!system.contains("\"properties\""), "{system}");
    // The tools it has, said, because the instructions describe tools it has not.
    assert!(system.contains("You have exactly these tools"), "{system}");
    assert!(system.contains(&AUTHOR_TOOLS.join(", ")), "{system}");
    assert!(system.contains("You have none of them"), "{system}");
    assert_eq!(first[1]["role"], "user");
    assert!(first[1]["content"]
        .as_str()
        .unwrap()
        .contains("`door-lock`"));
    // The call the model made reached the authoring server, with its arguments.
    assert!(
        called(&rig.folder).contains("\"arguments\":{\"work\":\"door-lock\"}"),
        "{}",
        called(&rig.folder)
    );
    // The second turn has the model's call and its result, answered under the id the model gave.
    let second = rig.server.messages(1);
    assert_eq!(second.len(), 4);
    assert_eq!(second[2]["role"], "assistant");
    assert_eq!(second[2]["tool_calls"][0]["id"], "call_1");
    assert_eq!(second[2]["tool_calls"][0]["function"]["name"], "works_read");
    assert!(
        second[2].get("reasoning").is_none(),
        "the model's reasoning is its own"
    );
    assert_eq!(second[3]["role"], "tool");
    assert_eq!(second[3]["tool_call_id"], "call_1");
    assert_eq!(second[3]["content"], "ok:works_read");
}

#[test]
fn the_model_is_offered_the_tools_a_client_may_use_with_what_the_server_says_of_each() {
    let rig = Rig::new("local-offered", vec![says(&draft("<scxml/>"))]);

    rig.local_with(LocalConfig::for_model("m"), &with_a_saving_tool())
        .generate(&job(), &Cancel::new())
        .unwrap();

    let tools = rig.server.requests()[0].body["tools"]
        .as_array()
        .unwrap()
        .clone();
    let names: Vec<&str> = tools
        .iter()
        .map(|t| t["function"]["name"].as_str().unwrap())
        .collect();
    // These and no other, in the order they are named: a tool that saves is not among them.
    assert_eq!(names, AUTHOR_TOOLS);
    assert_eq!(tools[0]["type"], "function");
    assert_eq!(tools[0]["function"]["description"], "Does works_read.");
    assert_eq!(
        tools[0]["function"]["parameters"]["properties"]["work"]["type"],
        "string"
    );
}

#[test]
fn a_tool_that_was_not_offered_is_not_called_and_the_model_is_told_which_there_are() {
    let rig = Rig::new(
        "local-not-offered",
        vec![
            calls(&[(Some("c1"), "works_save_model", "{}")]),
            says(&draft("<scxml/>")),
        ],
    );

    rig.local_with(LocalConfig::for_model("m"), &with_a_saving_tool())
        .generate(&job(), &Cancel::new())
        .unwrap();

    // It exists on the authoring server, and the model has no way to reach it.
    assert!(
        !called(&rig.folder).contains("works_save_model"),
        "{}",
        called(&rig.folder)
    );
    let told = tool_message(&rig.server.messages(1), 3);
    assert!(
        told.starts_with("Error: there is no tool named `works_save_model`"),
        "{told}"
    );
    assert!(told.contains("works_read"), "{told}");
}

#[test]
fn what_a_tool_gives_is_handed_back_in_order_and_a_failure_and_nothing_are_said_as_such() {
    let rig = Rig::new(
        "local-results",
        vec![
            calls(&[
                (Some("a"), "works_read", "{}"),
                (Some("b"), "validate_scxml", "{}"),
                (Some("c"), "scxml_unresolved", "{}"),
            ]),
            says(&draft("<scxml/>")),
        ],
    );

    rig.run().unwrap();

    let second = rig.server.messages(1);
    // One message of the model with three calls, and then each result under its own id, in order.
    assert_eq!(second[2]["tool_calls"].as_array().unwrap().len(), 3);
    assert_eq!(
        (
            second[3]["tool_call_id"].as_str(),
            second[3]["content"].as_str()
        ),
        (Some("a"), Some("ok:works_read"))
    );
    assert_eq!(
        (
            second[4]["tool_call_id"].as_str(),
            second[4]["content"].as_str()
        ),
        (Some("b"), Some("Error: line 3: bad"))
    );
    assert_eq!(
        (
            second[5]["tool_call_id"].as_str(),
            second[5]["content"].as_str()
        ),
        (Some("c"), Some("(the tool gave no output)"))
    );
}

#[test]
fn calls_that_have_no_id_are_each_answered_under_one_made_for_them() {
    let rig = Rig::new(
        "local-no-id",
        vec![
            calls(&[(None, "works_read", "{}"), (None, "scxml_kinds", "{}")]),
            says(&draft("<scxml/>")),
        ],
    );

    rig.run().unwrap();

    let second = rig.server.messages(1);
    // Two calls with no id of their own get ids that differ, so that each result is answered
    // under its own call and not under both.
    assert_eq!(second[2]["tool_calls"][0]["id"], "call_0_0");
    assert_eq!(second[2]["tool_calls"][1]["id"], "call_0_1");
    assert_eq!(
        (
            second[3]["tool_call_id"].as_str(),
            second[3]["content"].as_str()
        ),
        (Some("call_0_0"), Some("ok:works_read"))
    );
    assert_eq!(
        (
            second[4]["tool_call_id"].as_str(),
            second[4]["content"].as_str()
        ),
        (Some("call_0_1"), Some("ok:scxml_kinds"))
    );
}

#[test]
fn arguments_that_are_not_json_are_answered_to_the_model_as_the_error_they_are_and_the_run_goes_on()
{
    let rig = Rig::new(
        "local-bad-arguments",
        vec![
            calls(&[(Some("c1"), "works_read", "{not json")]),
            says(&draft("<scxml/>")),
        ],
    );

    rig.run().unwrap();

    assert!(tool_message(&rig.server.messages(1), 3).contains("not valid JSON"));
    // Nothing was sent to the authoring server for it.
    assert_eq!(called(&rig.folder), "");
}

#[test]
fn the_draft_a_model_wraps_in_a_fence_and_ends_with_a_tool_marker_is_taken() {
    let wrapped = format!(
        "Done.\n```json\n{}\n```\n<tool_call>",
        draft("<scxml><!-- fenced --></scxml>")
    );
    let rig = Rig::new("local-fenced", vec![says(&wrapped)]);

    let made = rig.run().unwrap();

    assert_eq!(made.model.entry_text(), "<scxml><!-- fenced --></scxml>");
    assert_eq!(rig.server.requests().len(), 1);
}

#[test]
fn a_message_that_is_not_the_draft_is_answered_with_what_was_wrong_and_the_model_writes_again() {
    let rig = Rig::new(
        "local-repair",
        vec![
            says("I could not write a model for this."),
            says(&draft("<scxml/>")),
        ],
    );

    rig.run().unwrap();

    let second = rig.server.messages(1);
    let last = second.len() - 1;
    assert_eq!(second[last - 1]["role"], "assistant");
    assert_eq!(
        second[last - 1]["content"],
        "I could not write a model for this."
    );
    assert_eq!(second[last]["role"], "user");
    let told = second[last]["content"].as_str().unwrap();
    assert!(told.contains("not the draft"), "{told}");
    assert!(told.contains("no JSON object"), "{told}");
}

#[test]
fn a_model_that_never_gives_the_draft_is_unusable_after_the_repairs_and_the_request_is_told_why() {
    let rig = Rig::new(
        "local-unusable",
        vec![says("nope"), says("still nope"), says("no, really")],
    );

    let refused = rig.run().unwrap_err();

    let GenerateError::Unusable(wrong) = refused else {
        panic!("expected Unusable, got {refused:?}");
    };
    assert!(wrong.contains("no JSON object"), "{wrong}");
    // Asked once, and again after each of the two repairs.
    assert_eq!(rig.server.requests().len(), 3);
}

/// The draft with a model and no requirements: what the model is asked to write.
fn model_only(model: &str) -> String {
    json!({"model": {"documents": [{"name": "m.scxml", "text": model}]}}).to_string()
}

#[test]
fn the_requirement_list_is_the_one_the_tool_gave_and_the_model_does_not_write_it_out() {
    let rig = Rig::new(
        "local-list",
        vec![
            calls(&[(Some("c1"), "scxml_requirement_set", "{}")]),
            says(&model_only("<scxml><!-- no list in the draft --></scxml>")),
        ],
    );
    requirement_set_gives(
        &rig.folder,
        &json!({
            "manifest_text": "{\"doc_id\":\"from-tool\",\"rev\":\"9\"}\n",
            "sidecar_text": "{\"R1\":\"the door closes\"}",
            "unclaimed_sentences": [],
        }),
    );

    let made = rig.run().unwrap();

    assert_eq!(
        made.model.entry_text(),
        "<scxml><!-- no list in the draft --></scxml>"
    );
    assert_eq!(
        made.requirements.manifest,
        "{\"doc_id\":\"from-tool\",\"rev\":\"9\"}\n"
    );
    assert_eq!(
        made.requirements.sidecar.as_deref(),
        Some("{\"R1\":\"the door closes\"}")
    );
}

#[test]
fn a_list_the_model_retyped_wrongly_is_not_the_list() {
    let wrong = json!({
        "model": {"documents": [{"name": "m.scxml", "text": "<scxml/>"}]},
        "requirements": {"manifest_text": "a copy with a mistake in it"},
    })
    .to_string();
    let rig = Rig::new(
        "local-list-retyped",
        vec![
            calls(&[(Some("c1"), "scxml_requirement_set", "{}")]),
            says(&wrong),
        ],
    );
    requirement_set_gives(
        &rig.folder,
        &json!({"manifest_text": "{\"doc_id\":\"from-tool\"}\n"}),
    );

    let made = rig.run().unwrap();

    assert_eq!(made.requirements.manifest, "{\"doc_id\":\"from-tool\"}\n");
}

#[test]
fn the_last_list_the_tool_gave_is_the_one_kept() {
    let rig = Rig::new(
        "local-list-last",
        vec![
            calls(&[(Some("c1"), "scxml_requirement_set", "{}")]),
            calls(&[(Some("c2"), "scxml_requirement_set", "{}")]),
            calls(&[(Some("c3"), "validate_scxml", "{}")]),
            says(&model_only("<scxml/>")),
        ],
    );
    requirement_set_gives_at(
        &rig.folder,
        1,
        &json!({"manifest_text": "{\"rev\":\"1\"}\n"}),
        false,
    );
    requirement_set_gives_at(
        &rig.folder,
        2,
        &json!({"manifest_text": "{\"rev\":\"2\"}\n"}),
        false,
    );

    let made = rig.run().unwrap();

    // The model built the list again after changing its mind; the call of another tool after that
    // does not replace it.
    assert_eq!(made.requirements.manifest, "{\"rev\":\"2\"}\n");
}

#[test]
fn what_another_tool_gives_is_not_the_list_even_when_it_is_shaped_like_one() {
    let rig = Rig::new(
        "local-list-other-tool",
        vec![
            calls(&[(Some("c1"), "scxml_requirement_set", "{}")]),
            // `works_read` gives back the list that was saved before, in the same two fields.
            calls(&[(Some("c2"), "works_read", "{}")]),
            says(&model_only("<scxml/>")),
        ],
    );
    requirement_set_gives(
        &rig.folder,
        &json!({"manifest_text": "{\"rev\":\"built\"}\n"}),
    );
    tool_gives(
        &rig.folder,
        "works_read",
        &json!({"manifest_text": "{\"rev\":\"saved-before\"}\n"}),
    );

    let made = rig.run().unwrap();

    assert_eq!(made.requirements.manifest, "{\"rev\":\"built\"}\n");
}

#[test]
fn a_call_of_the_tool_that_failed_does_not_replace_the_list() {
    let rig = Rig::new(
        "local-list-failed",
        vec![
            calls(&[(Some("c1"), "scxml_requirement_set", "{}")]),
            calls(&[(Some("c2"), "scxml_requirement_set", "{}")]),
            says(&model_only("<scxml/>")),
        ],
    );
    requirement_set_gives_at(
        &rig.folder,
        1,
        &json!({"manifest_text": "{\"rev\":\"1\"}\n"}),
        false,
    );
    // A failure whose text happens to be JSON with a list in it is still a failure.
    requirement_set_gives_at(
        &rig.folder,
        2,
        &json!({"manifest_text": "{\"rev\":\"from-the-failure\"}\n"}),
        true,
    );

    let made = rig.run().unwrap();

    assert_eq!(made.requirements.manifest, "{\"rev\":\"1\"}\n");
}

#[test]
fn a_model_that_never_built_the_list_and_gave_none_is_told_the_list_is_missing() {
    let rig = Rig::new(
        "local-list-missing",
        vec![says(&model_only("<scxml/>")), says(&draft("<scxml/>"))],
    );

    let made = rig.run().unwrap();

    // Told what was wrong, and then it wrote the draft with a list of its own.
    let told = rig.server.messages(1);
    assert!(
        told.last().unwrap()["content"]
            .as_str()
            .unwrap()
            .contains("requirement list is missing"),
        "{told:?}"
    );
    assert!(made.requirements.manifest.contains("door"));
}

#[test]
fn a_model_that_says_the_schema_back_is_told_that_it_did() {
    let echo = json!({
        "properties": {"model": {"type": "object"}, "requirements": {"type": "object"}},
        "required": ["model", "requirements"],
        "type": "object",
    })
    .to_string();
    let rig = Rig::new("local-echo", vec![says(&echo), says(&draft("<scxml/>"))]);

    rig.run().unwrap();

    let told = rig.server.messages(1);
    assert!(
        told.last().unwrap()["content"]
            .as_str()
            .unwrap()
            .contains("JSON Schema"),
        "{told:?}"
    );
}

#[test]
fn a_model_that_only_calls_tools_runs_out_of_turns() {
    let script = (0..10)
        .map(|_| calls(&[(Some("c"), "works_read", "{}")]))
        .collect();
    let rig = Rig::new("local-turns", script);
    let config = LocalConfig {
        max_turns: 3,
        ..LocalConfig::for_model("m")
    };

    let refused = rig
        .local(config)
        .generate(&job(), &Cancel::new())
        .unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("did not give its draft in 3 turns"), "{said}");
    assert_eq!(rig.server.requests().len(), 3);
}

// ---- when the servers are not what they should be --------------------------------------------

#[test]
fn a_model_server_that_refuses_is_said_in_its_words() {
    let rig = Rig::new(
        "local-refused",
        vec![Script::Reply(
            400,
            r#"{"error":{"message":"model 'qwen-test' not found"}}"#.to_string(),
        )],
    );

    let refused = rig.run().unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("answered 400"), "{said}");
    assert!(said.contains("model 'qwen-test' not found"), "{said}");
}

#[test]
fn an_answer_that_is_not_a_chat_completion_is_said_not_to_be() {
    let rig = Rig::new(
        "local-not-chat",
        vec![Script::Reply(200, "<html>hello</html>".to_string())],
    );

    let refused = rig.run().unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("not JSON"), "{said}");
}

#[test]
fn the_authoring_server_is_told_which_work_the_run_is_for() {
    let rig = Rig::new("local-scope", vec![says(&draft("<scxml/>"))]);

    rig.run().unwrap();

    // Without it the server answers for every work of the folder, and a specification that names
    // another work in a call is read by the model.
    assert_eq!(scope_of(&rig.folder), "door-lock");
}

#[test]
fn a_work_the_host_named_for_the_server_is_not_the_work_the_run_is_for() {
    let rig = Rig::new("local-scope-replaced", vec![says(&draft("<scxml/>"))]);
    let mut author = authoring_server(&rig.folder, &AUTHOR_TOOLS);
    author
        .env
        .push(("SCE_AUTHOR_WORK".to_string(), "another-work".to_string()));
    let local = Local::new(
        Endpoint::parse(&rig.server.address).unwrap(),
        author,
        LocalConfig::for_model("qwen-test"),
    );

    local.generate(&job(), &Cancel::new()).unwrap();

    assert_eq!(scope_of(&rig.folder), "door-lock");
}

#[test]
fn a_model_server_that_is_not_there_is_said_so() {
    let folder = scratch("local-nobody");
    let local = Local::new(
        Endpoint::parse(NOBODY).unwrap(),
        authoring_server(&folder, &AUTHOR_TOOLS),
        LocalConfig::for_model("m"),
    );

    let refused = local.generate(&job(), &Cancel::new()).unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("could not be reached"), "{said}");
}

/// What a server answers when the arguments a model wrote for a tool call are not JSON, so that
/// the call cannot be made and there is no message to give back: Ollama answers 500, and quotes
/// what the model wrote (measured with `gpt-oss:120b`, which wrote a document into a call).
fn unreadable_call(raw: &str, why: &str) -> Script {
    Script::Reply(
        500,
        json!({"error": {"message": format!("error parsing tool call: raw='{raw}', err={why}")}})
            .to_string(),
    )
}

#[test]
fn a_tool_call_the_server_could_not_read_is_told_to_the_model_which_makes_it_again() {
    let rig = Rig::new(
        "local-unreadable",
        vec![
            calls(&[(Some("c1"), "works_read", "{\"work\":\"door-lock\"}")]),
            unreadable_call(
                "{ \"documents\": [ { \"name\": \"door",
                "unexpected end of JSON input",
            ),
            says(&draft("<scxml/>")),
        ],
    );

    let made = rig.run().unwrap();

    assert_eq!(made.model.entry_text(), "<scxml/>");
    assert_eq!(rig.server.requests().len(), 3);
    // The model was asked again, and told that its call was not read and why.
    let last = rig.server.messages(2).pop().unwrap();
    assert_eq!(last["role"], "user");
    let words = last["content"].as_str().unwrap();
    assert!(
        words.contains("could not read your last tool call"),
        "{words}"
    );
    assert!(words.contains("unexpected end of JSON input"), "{words}");
    // What it wrote is not said back to it: it can be as long as a document, and it has it.
    assert!(!words.contains("\"documents\""), "{words}");
}

#[test]
fn a_call_the_server_could_not_read_is_a_step_that_says_why_it_could_not() {
    let rig = Rig::new(
        "local-unreadable-trace",
        vec![
            unreadable_call("{ \"a\": ", "unexpected end of JSON input"),
            says(&draft("<scxml/>")),
        ],
    );
    let steps: Arc<Mutex<Vec<Step>>> = Arc::default();
    let sink = Arc::clone(&steps);

    rig.local(LocalConfig::for_model("m"))
        .with_trace(move |step| sink.lock().unwrap().push(step.clone()))
        .generate(&job(), &Cancel::new())
        .unwrap();

    let steps = steps.lock().unwrap().clone();
    assert_eq!(
        steps[..3],
        [
            Step::Asked {
                turn: 0,
                messages: 2
            },
            Step::NotReadable {
                why: "unexpected end of JSON input".to_string()
            },
            // The model is asked again with what it was told added to the conversation.
            Step::Asked {
                turn: 1,
                messages: 3
            },
        ]
    );
}

#[test]
fn a_model_whose_tool_calls_are_never_read_is_given_a_few_tries_and_then_the_server_is_quoted() {
    let rig = Rig::new(
        "local-unreadable-always",
        vec![
            unreadable_call("{ \"a\": ", "unexpected end of JSON input"),
            unreadable_call("{ \"b\": ", "unexpected end of JSON input"),
            unreadable_call("{ \"c\": ", "unexpected end of JSON input"),
            says(&draft("<scxml/>")),
        ],
    );

    let refused = rig.run().unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("the server answered 500"), "{said}");
    assert!(said.contains("error parsing tool call"), "{said}");
    // The first, and a try after each of the two repairs: not an endless asking.
    assert_eq!(rig.server.requests().len(), 3);
}

#[test]
fn a_server_error_that_is_not_a_tool_call_it_could_not_read_is_not_put_to_the_model() {
    let rig = Rig::new(
        "local-500-other",
        vec![
            Script::Reply(
                500,
                r#"{"error":"the model runner has crashed"}"#.to_string(),
            ),
            says(&draft("<scxml/>")),
        ],
    );

    let refused = rig.run().unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("the model runner has crashed"), "{said}");
    assert_eq!(rig.server.requests().len(), 1);
}

#[test]
fn a_model_server_that_does_not_answer_ends_the_run_at_its_time() {
    let rig = Rig::new("local-slow", vec![Script::Hang]);
    let config = LocalConfig {
        timeout: Duration::from_millis(1500),
        ..LocalConfig::for_model("m")
    };

    let began = Instant::now();
    let refused = rig
        .local(config)
        .generate(&job(), &Cancel::new())
        .unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("longer than 1500 milliseconds"), "{said}");
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "{:?}",
        began.elapsed()
    );
}

#[test]
fn a_run_that_is_told_to_stop_stops_and_the_model_server_sees_the_connection_close() {
    let rig = Rig::new("local-cancel", vec![Script::Hang]);
    let cancel = Cancel::new();
    let stopper = {
        let cancel = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1500));
            cancel.cancel();
        })
    };

    let began = Instant::now();
    let stopped = rig
        .local(LocalConfig::for_model("m"))
        .generate(&job(), &cancel);
    stopper.join().unwrap();

    assert!(
        matches!(stopped, Err(GenerateError::Cancelled)),
        "{stopped:?}"
    );
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "{:?}",
        began.elapsed()
    );
    // The model is not left working for nobody.
    let limit = Instant::now() + Duration::from_secs(5);
    while !rig.server.hung_up.load(Ordering::SeqCst) && Instant::now() < limit {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        rig.server.hung_up.load(Ordering::SeqCst),
        "the server did not see the connection close"
    );
}

#[test]
fn an_authoring_server_that_lacks_a_tool_the_task_needs_is_said_to_lack_it_and_the_model_is_not_asked(
) {
    let rig = Rig::new("local-lacking", vec![says(&draft("<scxml/>"))]);
    let tools: Vec<&str> = AUTHOR_TOOLS
        .iter()
        .copied()
        .filter(|t| *t != "decisions")
        .collect();

    let refused = rig
        .local_with(LocalConfig::for_model("m"), &tools)
        .generate(&job(), &Cancel::new())
        .unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("does not offer decisions"), "{said}");
    assert!(rig.server.requests().is_empty());
}

#[test]
fn an_authoring_server_that_cannot_be_started_is_said_so_and_the_model_is_not_asked() {
    let server = chat_server(vec![says(&draft("<scxml/>"))]);
    let local = Local::new(
        Endpoint::parse(&server.address).unwrap(),
        AuthorServer {
            command: PathBuf::from("/nowhere/sce-author-mcp"),
            args: vec![],
            env: vec![],
        },
        LocalConfig::for_model("m"),
    );

    let refused = local.generate(&job(), &Cancel::new()).unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("could not be started"), "{said}");
    assert!(server.requests().is_empty());
}

#[test]
fn a_key_is_sent_when_there_is_one_and_not_when_there_is_none() {
    let with = Rig::new("local-key", vec![says(&draft("<scxml/>"))]);
    let without = Rig::new("local-no-key", vec![says(&draft("<scxml/>"))]);

    with.local(LocalConfig::for_model("m"))
        .with_bearer("sk-local-secret".to_string())
        .generate(&job(), &Cancel::new())
        .unwrap();
    without.run().unwrap();

    assert!(with.server.requests()[0]
        .head
        .contains("Authorization: Bearer sk-local-secret"));
    assert!(!without.server.requests()[0]
        .head
        .to_ascii_lowercase()
        .contains("authorization"));
}

#[test]
fn a_run_leaves_no_authoring_server_behind() {
    let rig = Rig::new("local-no-leftover", vec![says(&draft("<scxml/>"))]);
    rig.run().unwrap();

    // The stand-in is the process of this folder's script: none of it is left running.
    let script = rig.folder.join("author.sh").display().to_string();
    let limit = Instant::now() + Duration::from_secs(5);
    let running = || {
        std::fs::read_dir("/proc")
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| std::fs::read(entry.path().join("cmdline")).ok())
            .any(|cmdline| String::from_utf8_lossy(&cmdline).contains(&script))
    };
    while running() && Instant::now() < limit {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(!running(), "the authoring server is still running");
}

#[test]
fn a_run_tells_whoever_watches_each_step_of_it_in_order() {
    let rig = Rig::new(
        "local-trace",
        vec![
            calls(&[
                (Some("a"), "works_read", "{\"work\":\"w\"}"),
                (Some("b"), "validate_scxml", "{}"),
            ]),
            says("not the draft"),
            says(&draft("<scxml/>")),
        ],
    );
    let steps: Arc<Mutex<Vec<Step>>> = Arc::default();
    let sink = Arc::clone(&steps);

    rig.local(LocalConfig::for_model("m"))
        .with_trace(move |step| sink.lock().unwrap().push(step.clone()))
        .generate(&job(), &Cancel::new())
        .unwrap();

    let steps = steps.lock().unwrap().clone();
    let said = |turn: u32, words: &str, calls: &[(&str, &str)]| Step::Said {
        turn,
        words: words.to_string(),
        calls: calls
            .iter()
            .map(|(n, a)| (n.to_string(), a.to_string()))
            .collect(),
    };
    assert_eq!(
        steps,
        vec![
            Step::Asked {
                turn: 0,
                messages: 2
            },
            said(
                0,
                "",
                &[("works_read", "{\"work\":\"w\"}"), ("validate_scxml", "{}")]
            ),
            Step::Tool {
                name: "works_read".to_string(),
                failed: false,
                words: "ok:works_read".to_string()
            },
            // A tool that said it failed is a step that says so, with its words.
            Step::Tool {
                name: "validate_scxml".to_string(),
                failed: true,
                words: "Error: line 3: bad".to_string()
            },
            // The question and the model's call, and the two results.
            Step::Asked {
                turn: 1,
                messages: 5
            },
            said(1, "not the draft", &[]),
            Step::NotTheDraft {
                why: "there is no JSON object in it: not the draft".to_string()
            },
            Step::Asked {
                turn: 2,
                messages: 7
            },
            said(2, &draft("<scxml/>"), &[]),
        ]
    );
}

#[test]
fn the_message_that_ends_a_run_unusable_is_a_step_too_and_says_what_the_request_is_told() {
    let rig = Rig::new(
        "local-trace-unusable",
        vec![says("nope"), says("still nope"), says("no, really")],
    );
    let steps: Arc<Mutex<Vec<Step>>> = Arc::default();
    let sink = Arc::clone(&steps);

    let refused = rig
        .local(LocalConfig::for_model("m"))
        .with_trace(move |step| sink.lock().unwrap().push(step.clone()))
        .generate(&job(), &Cancel::new())
        .unwrap_err();

    let GenerateError::Unusable(told) = refused else {
        panic!("expected Unusable, got {refused:?}");
    };
    let whys: Vec<String> = steps
        .lock()
        .unwrap()
        .iter()
        .filter_map(|step| match step {
            Step::NotTheDraft { why } => Some(why.clone()),
            _ => None,
        })
        .collect();
    // One for each message that was not the draft, the last of them among them: three, and the last
    // is the words the request is told.
    assert_eq!(whys.len(), 3, "{whys:?}");
    assert_eq!(whys.last(), Some(&told));
}

#[test]
fn a_run_with_nobody_watching_is_the_same_run() {
    let rig = Rig::new("local-untraced", vec![says(&draft("<scxml/>"))]);

    assert!(rig.run().is_ok());
}

#[test]
fn the_instructions_are_named() {
    let rig = Rig::new("local-instructions", vec![]);

    let name = rig
        .local(LocalConfig::for_model("m"))
        .instructions()
        .unwrap();

    assert!(name.starts_with("local/"), "{name}");
    assert_eq!(rig.local(LocalConfig::for_model("another")).kind(), "local");
}

// ---- the models a server lists ---------------------------------------------------------------

#[test]
fn the_models_a_server_lists_are_the_ones_it_names_in_its_order() {
    let server = chat_server(vec![Script::Reply(
        200,
        json!({"object": "list", "data": [{"id": "qwen3-coder:30b", "object": "model"}, {"id": "devstral:24b"}]}).to_string(),
    )]);

    let models = list_models(&Endpoint::parse(&server.address).unwrap(), None).unwrap();

    assert_eq!(models, vec!["qwen3-coder:30b", "devstral:24b"]);
    let head = &server.requests()[0].head;
    assert!(head.starts_with("GET /v1/models HTTP/1.1"), "{head}");
}

#[test]
fn a_server_that_lists_none_lists_none() {
    let server = chat_server(vec![Script::Reply(
        200,
        r#"{"object":"list","data":[]}"#.to_string(),
    )]);

    assert_eq!(
        list_models(&Endpoint::parse(&server.address).unwrap(), None),
        Ok(vec![])
    );
}

#[test]
fn a_server_that_wants_a_key_says_so_and_a_page_that_is_not_a_list_is_said_not_to_be_one() {
    let wants = chat_server(vec![Script::Reply(
        401,
        r#"{"error":{"message":"missing api key"}}"#.to_string(),
    )]);
    let page = chat_server(vec![Script::Reply(
        200,
        "<html>a login page</html>".to_string(),
    )]);
    let object = chat_server(vec![Script::Reply(200, r#"{"models":[]}"#.to_string())]);

    let unauthorized = list_models(&Endpoint::parse(&wants.address).unwrap(), None).unwrap_err();
    let not_json = list_models(&Endpoint::parse(&page.address).unwrap(), None).unwrap_err();
    let not_a_list = list_models(&Endpoint::parse(&object.address).unwrap(), None).unwrap_err();

    assert!(
        unauthorized.to_string().contains("missing api key"),
        "{unauthorized}"
    );
    assert!(
        unauthorized.to_string().contains("wants a key"),
        "{unauthorized}"
    );
    assert!(matches!(unauthorized, ModelsError::NeedsKey(_)));
    assert!(matches!(not_json, ModelsError::Refused(_)));
    assert!(
        not_json.to_string().contains("OpenAI-compatible"),
        "{not_json}"
    );
    assert!(matches!(not_a_list, ModelsError::Refused(_)));
}

#[test]
fn a_server_that_is_not_there_is_unreachable() {
    let nobody = list_models(&Endpoint::parse(NOBODY).unwrap(), None).unwrap_err();

    assert!(matches!(nobody, ModelsError::Unreachable(_)), "{nobody:?}");
    assert!(
        nobody.to_string().contains("could not be reached"),
        "{nobody}"
    );
}

#[test]
fn the_key_given_for_listing_is_sent() {
    let server = chat_server(vec![Script::Reply(
        200,
        r#"{"object":"list","data":[]}"#.to_string(),
    )]);

    list_models(&Endpoint::parse(&server.address).unwrap(), Some("sk-list")).unwrap();

    assert!(server.requests()[0]
        .head
        .contains("Authorization: Bearer sk-list"));
}
