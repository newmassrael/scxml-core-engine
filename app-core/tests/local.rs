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

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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

use common::{scratch, write_program};

// ---- the model server ------------------------------------------------------------------------

/// What was sent to the model server: the head of the request as text, and its body.
#[derive(Debug, Clone)]
struct Sent {
    head: String,
    body: Value,
}

/// What the model server does with the next request.
enum Script {
    Reply(u16, String),
    /// Reads the request and then says nothing, noting whether the other end went away.
    Hang,
}

struct ChatServer {
    address: String,
    seen: Arc<Mutex<Vec<Sent>>>,
    hung_up: Arc<AtomicBool>,
}

impl ChatServer {
    fn requests(&self) -> Vec<Sent> {
        self.seen.lock().unwrap().clone()
    }

    /// The messages of the request that was the `n`th.
    fn messages(&self, n: usize) -> Vec<Value> {
        self.requests()[n].body["messages"]
            .as_array()
            .unwrap()
            .clone()
    }
}

fn read_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut request = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let count = stream.read(&mut chunk).unwrap_or(0);
        request.extend_from_slice(&chunk[..count]);
        if let Some(at) = request.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&request[..at]).into_owned();
            let length = head
                .to_ascii_lowercase()
                .lines()
                .find_map(|line| line.strip_prefix("content-length:").map(str::to_string))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= at + 4 + length {
                return (head, request[at + 4..at + 4 + length].to_vec());
            }
        }
        if count == 0 {
            return (String::new(), Vec::new());
        }
    }
}

fn chat_server(script: Vec<Script>) -> ChatServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/v1", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let hung_up = Arc::new(AtomicBool::new(false));
    let script = Arc::new(Mutex::new(VecDeque::from(script)));
    let (record, noted, queue) = (Arc::clone(&seen), Arc::clone(&hung_up), script);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (head, body) = read_request(&mut stream);
            record.lock().unwrap().push(Sent {
                head,
                body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            });
            let next = queue.lock().unwrap().pop_front().unwrap_or_else(|| {
                Script::Reply(500, r#"{"error":"the script is exhausted"}"#.to_string())
            });
            match next {
                Script::Reply(status, body) => {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
                Script::Hang => {
                    let mut byte = [0u8; 1];
                    let closed = match stream.read(&mut byte) {
                        Ok(count) => count == 0,
                        Err(e) => matches!(
                            e.kind(),
                            std::io::ErrorKind::ConnectionReset
                                | std::io::ErrorKind::ConnectionAborted
                                | std::io::ErrorKind::BrokenPipe
                        ),
                    };
                    noted.store(closed, Ordering::SeqCst);
                }
            }
        }
    });
    ChatServer {
        address,
        seen,
        hung_up,
    }
}

/// A chat completion that calls tools: each is (id, name, arguments as the model wrote them).
fn calls(calls: &[(Option<&str>, &str, &str)]) -> Script {
    let list: Vec<Value> = calls
        .iter()
        .map(|(id, name, arguments)| {
            let mut call = json!({
                "type": "function",
                "function": {"name": name, "arguments": arguments},
            });
            if let Some(id) = id {
                call["id"] = json!(id);
            }
            call
        })
        .collect();
    Script::Reply(
        200,
        json!({"object": "chat.completion", "choices": [{
            "index": 0,
            "finish_reason": "tool_calls",
            "message": {"role": "assistant", "content": null, "reasoning": "private", "tool_calls": list},
        }]})
        .to_string(),
    )
}

/// A chat completion that says `content` and calls nothing.
fn says(content: &str) -> Script {
    Script::Reply(
        200,
        json!({"object": "chat.completion", "choices": [{
            "index": 0,
            "finish_reason": "stop",
            "message": {"role": "assistant", "content": content},
        }]})
        .to_string(),
    )
}

/// The draft, as a model writes it.
fn draft(model: &str) -> String {
    json!({
        "model": {"documents": [{"name": "m.scxml", "text": model}]},
        "requirements": {"manifest_text": "{\"doc_id\":\"door\",\"rev\":\"1\"}\n"},
    })
    .to_string()
}

// ---- the authoring server --------------------------------------------------------------------

/// A shell stand-in for the authoring server with `tools`. It answers `tools/call` by the tool's
/// name: `validate_scxml` says it failed, `scxml_unresolved` says nothing, any other says `ok:` and
/// its name. Every call is written to `calls` in the folder.
fn authoring_server(folder: &Path, tools: &[&str]) -> AuthorServer {
    let list = tools
        .iter()
        .map(|name| {
            format!(
                "{{\"name\":\"{name}\",\"description\":\"Does {name}.\",\"inputSchema\":{{\"type\":\"object\",\"properties\":{{\"work\":{{\"type\":\"string\"}}}}}}}}"
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let program = folder.join("author.sh");
    let script = format!(
        "#!/bin/sh\n\
         [ \"$1\" = \"--version\" ] && exit 0\n\
         tool_text() {{\n\
           if [ -f '{folder}'/answer.$1.json ]; then cat '{folder}'/answer.$1.json; else printf '\"ok:%s\"' \"$1\"; fi\n\
         }}\n\
         requirement_set() {{\n\
           n=$(grep -c 'scxml_requirement_set' '{calls}')\n\
           file='{folder}'/requirement_set.$n.json\n\
           [ -f \"$file\" ] || file='{set}'\n\
           if [ -f \"$file\" ]; then text=$(cat \"$file\"); else text='\"ok:scxml_requirement_set\"'; fi\n\
           failing=false\n\
           [ -f '{folder}'/requirement_set.$n.fails ] && failing=true\n\
           printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"content\":[{{\"type\":\"text\",\"text\":%s}}],\"isError\":%s}}}}\\n' \"$id\" \"$text\" \"$failing\"\n\
         }}\n\
         while IFS= read -r line; do\n\
           id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\\([0-9][0-9]*\\).*/\\1/p')\n\
           case \"$line\" in\n\
             *'\"method\":\"initialize\"'*)\n\
               printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{\"tools\":{{}}}},\"serverInfo\":{{\"name\":\"fake\",\"version\":\"1\"}},\"instructions\":\"Use the tools.\"}}}}\\n' \"$id\" ;;\n\
             *'\"method\":\"tools/list\"'*)\n\
               printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"tools\":[{list}]}}}}\\n' \"$id\" ;;\n\
             *'\"method\":\"tools/call\"'*)\n\
               printf '%s\\n' \"$line\" >> '{calls}'\n\
               name=$(printf '%s' \"$line\" | sed -n 's/.*\"name\":\"\\([a-z_]*\\)\".*/\\1/p')\n\
               case \"$name\" in\n\
                 validate_scxml) printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"content\":[{{\"type\":\"text\",\"text\":\"line 3: bad\"}}],\"isError\":true}}}}\\n' \"$id\" ;;\n\
                 scxml_unresolved) printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"content\":[]}}}}\\n' \"$id\" ;;\n\
                 scxml_requirement_set) requirement_set ;;\n\
                 *) printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"content\":[{{\"type\":\"text\",\"text\":%s}}]}}}}\\n' \"$id\" \"$(tool_text \"$name\")\" ;;\n\
               esac ;;\n\
           esac\n\
         done\n",
        calls = folder.join("calls").display(),
        folder = folder.display(),
        set = folder.join("requirement_set.json").display(),
    );
    write_program(&program, &script);
    AuthorServer {
        command: program,
        args: vec![],
        env: vec![],
    }
}

/// From now on the stand-in's `scxml_requirement_set` answers with `answer`, as the tool does:
/// the JSON that holds the list the application keeps (`manifest_text` and `sidecar_text`).
fn requirement_set_gives(folder: &Path, answer: &Value) {
    put_answer(folder, "requirement_set.json", answer);
}

/// The same for the `n`th call (counting from 1) alone, which says it failed when `fails`: a tool
/// that failed can still have written JSON in its text.
fn requirement_set_gives_at(folder: &Path, n: u32, answer: &Value, fails: bool) {
    put_answer(folder, &format!("requirement_set.{n}.json"), answer);
    if fails {
        std::fs::write(folder.join(format!("requirement_set.{n}.fails")), "").unwrap();
    }
}

/// From now on the stand-in's tool `name` (one that has no answer of its own) answers with
/// `answer`, in place of `ok:` and its name.
fn tool_gives(folder: &Path, name: &str, answer: &Value) {
    put_answer(folder, &format!("answer.{name}.json"), answer);
}

/// The file holds the answer as a JSON string literal, which is how a tool's text is put in the
/// reply.
fn put_answer(folder: &Path, file: &str, answer: &Value) {
    let literal = serde_json::to_string(&answer.to_string()).unwrap();
    std::fs::write(folder.join(file), literal).unwrap();
}

/// What the authoring server was called with, one request to a line.
fn called(folder: &Path) -> String {
    std::fs::read_to_string(folder.join("calls")).unwrap_or_default()
}

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
