// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A [`Generator`] that asks a model server the person runs to write the model of a work.
//!
//! Claude Code and Codex are programs that bring their own tool loop: the application starts one,
//! tells it which tools it may use, and takes its last message. A model server is only a service
//! that answers messages, so here the application is the other half of the conversation. It starts
//! the authoring server (the same one the programs are given), offers the model the tools the
//! programs are allowed and no others, carries each tool call the model makes to the authoring
//! server and its result back, and takes the model's last message as the draft.
//!
//! **What it speaks.** Chat Completions with tools, in the form OpenAI made common and that the
//! servers people run on their own computers (Ollama, llama.cpp's server, vLLM, LM Studio) offer:
//! `POST {address}/chat/completions` with `messages` and `tools`, and a `tool_calls` list in the
//! answer. Nothing here is particular to one server: what the person types is an address, and a
//! model the server lists (`GET {address}/models`).
//!
//! **What it does not trust.** A model is not a program that keeps to a form. Its last message may
//! be the draft inside a code fence, with a sentence before it, or with the marker of a tool call
//! left behind at its end (a model that was measured on a real server did all three). The draft is
//! taken out of whatever surrounds it, and what is not a draft is not guessed at: the model is told
//! what was wrong and writes again, a few times, and then the request is told it was unusable, as
//! for any client. A tool call that is not valid, or names a tool that was not offered, is answered
//! to the model as the error it is and is not an end.
//!
//! **What it leaves to the model server.** Nothing of the person's is kept by it: the specification
//! goes to the address they named and nowhere else, which is why a connection has a name a person
//! gave it. Whether the model can call tools at all is the model's; one that cannot is said to have
//! given no draft, in so many turns.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::claude_code::AuthorServer;
use crate::client_run::{
    blank_job, draft_from, prompt, schema, span_words, tail, Scratch, AUTHOR_TOOLS, SYSTEM_PROMPT,
};
use crate::http_client::{self, Endpoint, HttpError, Request};
use crate::mcp_client::{McpClient, McpError, Tool};
use crate::revision::Revision;
use crate::runner::{Cancel, Draft, GenerateError, Generator, Job};

/// The most of one answer of the server that is read.
const ANSWER_MAX: usize = 64 * 1024 * 1024;

/// How many times a message that is not the draft is answered with what was wrong, before the
/// request is told the draft was unusable.
const REPAIRS: u32 = 2;

/// The most of what a server said that is put in a sentence.
const SAID_MAX: usize = 400;

/// How long a server is given to list its models: a person is waiting at a screen for it.
const LIST_WITHIN: Duration = Duration::from_secs(10);

/// Said to the model beside what the authoring server says of how to use it.
const ANSWER_FORM: &str = "Your last message is the draft: one JSON object of exactly this form, \
and nothing else (no code fence, no words before or after it):";

/// What a model that answered with something that is not the draft is told.
fn repair_words(wrong: &str) -> String {
    format!(
        "Your last message is not the draft: {wrong}\n\nAnswer again with only the JSON object in \
         the form asked for."
    )
}

/// How a run is bounded.
#[derive(Debug, Clone)]
pub struct LocalConfig {
    /// The model the server is asked for, as it names it.
    pub model: String,
    /// How many times the model is asked, tool calls included.
    pub max_turns: u32,
    /// How long a run may take before it is stopped.
    pub timeout: Duration,
}

impl LocalConfig {
    /// The limits a run has when nothing else says: the ones of the other clients.
    pub fn for_model(model: impl Into<String>) -> Self {
        LocalConfig {
            model: model.into(),
            max_turns: 60,
            timeout: Duration::from_secs(30 * 60),
        }
    }
}

/// A model server, as a generator.
#[derive(Debug, Clone)]
pub struct Local {
    endpoint: Endpoint,
    author: AuthorServer,
    config: LocalConfig,
    /// The key the server wants, when it wants one.
    bearer: Option<String>,
}

impl Local {
    /// A generator that asks the server at `endpoint`, reaching the authoring server through
    /// `author`.
    pub fn new(endpoint: Endpoint, author: AuthorServer, config: LocalConfig) -> Self {
        Local {
            endpoint,
            author,
            config,
            bearer: None,
        }
    }

    /// The same, giving `key` to the server as the credential it asked for.
    pub fn with_bearer(mut self, key: String) -> Self {
        self.bearer = Some(key);
        self
    }

    /// How an error of the authoring server ends a run.
    fn mcp_failure(&self, error: McpError) -> GenerateError {
        match error {
            McpError::Cancelled => GenerateError::Cancelled,
            McpError::TimedOut => self.too_long(),
            other => GenerateError::Failed(other.to_string()),
        }
    }

    /// How an error of the model server ends a run.
    fn http_failure(&self, error: HttpError) -> GenerateError {
        match error {
            HttpError::Cancelled => GenerateError::Cancelled,
            HttpError::TimedOut => self.too_long(),
            other => GenerateError::Failed(other.to_string()),
        }
    }

    fn too_long(&self) -> GenerateError {
        GenerateError::Failed(format!(
            "the run took longer than {} and was stopped",
            span_words(self.config.timeout)
        ))
    }

    /// The tools the model is offered: the authoring server's, of the ones a client may use. A
    /// server that lacks one is not one a run can be made with, and says which.
    fn offered(
        &self,
        mcp: &mut McpClient,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<Vec<Tool>, GenerateError> {
        let have = mcp
            .tools(deadline, cancel)
            .map_err(|e| self.mcp_failure(e))?;
        let mut offered = Vec::new();
        let mut lacking = Vec::new();
        for name in AUTHOR_TOOLS {
            match have.iter().find(|tool| tool.name == name) {
                Some(tool) => offered.push(tool.clone()),
                None => lacking.push(name),
            }
        }
        if lacking.is_empty() {
            Ok(offered)
        } else {
            Err(GenerateError::Failed(format!(
                "the authoring server does not offer {}, which the task needs",
                lacking.join(", ")
            )))
        }
    }

    /// One turn: the messages so far to the server, and the model's message back.
    fn ask(
        &self,
        messages: &[Value],
        tools: &Value,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<Value, GenerateError> {
        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tools,
            "stream": false,
        })
        .to_string();
        let response = http_client::send(
            &self.endpoint,
            Request {
                method: "POST",
                path: "/chat/completions",
                bearer: self.bearer.as_deref(),
                json: Some(body.as_bytes()),
            },
            deadline,
            cancel,
            ANSWER_MAX,
        )
        .map_err(|e| self.http_failure(e))?;
        if !(200..300).contains(&response.status) {
            return Err(GenerateError::Failed(server_said(
                response.status,
                &response.body,
                self.bearer.is_some(),
            )));
        }
        let reply: Value = serde_json::from_slice(&response.body).map_err(|_| {
            GenerateError::Failed(format!(
                "the server's answer is not JSON: {}",
                short(&String::from_utf8_lossy(&response.body), 200)
            ))
        })?;
        let message = reply["choices"][0]["message"].clone();
        if message.is_object() {
            Ok(message)
        } else {
            Err(GenerateError::Failed(
                "the server's answer has no message: it is not a chat completion".to_string(),
            ))
        }
    }

    /// What a tool call of the model comes to, as the words handed back to it. A call that is not
    /// one that can be made is an answer to the model and not the end of the run.
    fn run_tool(
        &self,
        mcp: &mut McpClient,
        tools: &[Tool],
        call: &Call,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<String, GenerateError> {
        let Some(tool) = tools.iter().find(|tool| tool.name == call.name) else {
            let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
            return Ok(format!(
                "Error: there is no tool named `{}`. The tools are: {}.",
                call.name,
                names.join(", ")
            ));
        };
        let arguments = match &call.arguments {
            Ok(Value::Object(object)) => Value::Object(object.clone()),
            // A tool that takes nothing is called with nothing.
            Ok(Value::Null) => json!({}),
            Ok(_) => return Ok("Error: the arguments of a tool are one JSON object.".to_string()),
            Err(why) => {
                return Ok(format!("Error: the arguments are not valid JSON: {why}"));
            }
        };
        let called = mcp
            .call(&tool.name, &arguments, deadline, cancel)
            .map_err(|e| self.mcp_failure(e))?;
        Ok(match (called.is_error, called.text.is_empty()) {
            (true, _) => format!("Error: {}", called.text),
            (false, true) => "(the tool gave no output)".to_string(),
            (false, false) => called.text,
        })
    }
}

impl Generator for Local {
    fn kind(&self) -> &str {
        "local"
    }

    /// Named by what the model is told, what it must answer in, which tools it is offered, and how
    /// many times a wrong answer is put right. A change to any of them is another version of the
    /// instructions, and a bundle records which.
    fn instructions(&self) -> Option<String> {
        let material = format!(
            "{SYSTEM_PROMPT}\n{ANSWER_FORM}\n{}\n{}\n{}\n{REPAIRS}\n{}",
            schema(),
            prompt(&blank_job()),
            AUTHOR_TOOLS.join(","),
            repair_words("<what was wrong>"),
        );
        let digest = Revision::of(material.as_bytes()).to_string();
        Some(format!("local/{}", &digest[..12]))
    }

    fn generate(&self, job: &Job, cancel: &Cancel) -> Result<Draft, GenerateError> {
        let deadline = Instant::now() + self.config.timeout;
        let scratch = Scratch::new("sce-local")
            .map_err(|e| GenerateError::Failed(format!("a folder for the run: {e}")))?;
        let mut mcp = McpClient::start(&self.author, scratch.path(), deadline, cancel)
            .map_err(|e| self.mcp_failure(e))?;
        let tools = self.offered(&mut mcp, deadline, cancel)?;
        let offered = Value::Array(tools.iter().map(as_openai_tool).collect());
        let mut system = SYSTEM_PROMPT.to_string();
        if let Some(instructions) = mcp.instructions() {
            system.push_str("\n\n");
            system.push_str(instructions);
        }
        system.push_str(&format!("\n\n{ANSWER_FORM}\n{}", schema()));
        let mut messages = vec![
            json!({"role": "system", "content": system}),
            json!({"role": "user", "content": prompt(job)}),
        ];
        let mut repairs = 0;
        for turn in 0..self.config.max_turns {
            let message = self.ask(&messages, &offered, deadline, cancel)?;
            let calls = calls_of(&message, turn);
            if !calls.is_empty() {
                messages.push(echo_of(&message, &calls));
                for call in &calls {
                    let words = self.run_tool(&mut mcp, &tools, call, deadline, cancel)?;
                    messages
                        .push(json!({"role": "tool", "tool_call_id": call.id, "content": words}));
                }
                continue;
            }
            let said = content_of(&message);
            match draft_in(&said) {
                Ok(draft) => return Ok(draft),
                Err(wrong) if repairs < REPAIRS => {
                    repairs += 1;
                    messages.push(json!({"role": "assistant", "content": said}));
                    messages.push(json!({"role": "user", "content": repair_words(&wrong)}));
                }
                Err(wrong) => return Err(GenerateError::Unusable(wrong)),
            }
        }
        Err(GenerateError::Failed(format!(
            "the model did not give its draft in {} turns",
            self.config.max_turns
        )))
    }
}

/// A tool as the model is offered it.
fn as_openai_tool(tool: &Tool) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.schema,
        },
    })
}

/// A call of a tool the model made.
#[derive(Debug, Clone)]
struct Call {
    /// What the result is answered under: the model's own, or one made when it gave none.
    id: String,
    name: String,
    /// What the arguments were as the model wrote them, and the JSON they are when they are.
    raw: String,
    arguments: Result<Value, String>,
}

/// The tool calls in a message of the model, in order.
fn calls_of(message: &Value, turn: u32) -> Vec<Call> {
    let Some(calls) = message["tool_calls"].as_array() else {
        return Vec::new();
    };
    calls
        .iter()
        .enumerate()
        .map(|(at, call)| {
            let function = &call["function"];
            // A server gives the arguments as a string of JSON; one that gives an object is read too.
            let (raw, arguments) = match &function["arguments"] {
                Value::String(text) if text.trim().is_empty() => (text.clone(), Ok(Value::Null)),
                Value::String(text) => (
                    text.clone(),
                    serde_json::from_str::<Value>(text).map_err(|e| e.to_string()),
                ),
                other => (other.to_string(), Ok(other.clone())),
            };
            Call {
                id: call["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .map_or_else(|| format!("call_{turn}_{at}"), str::to_string),
                name: function["name"].as_str().unwrap_or("").to_string(),
                raw,
                arguments,
            }
        })
        .collect()
}

/// The model's message as it is put back into the conversation beside the results of its calls: what
/// it said and the calls it made, and nothing else it may have sent (its reasoning is its own).
fn echo_of(message: &Value, calls: &[Call]) -> Value {
    json!({
        "role": "assistant",
        "content": content_of(message),
        "tool_calls": calls.iter().map(|call| json!({
            "id": call.id,
            "type": "function",
            "function": {"name": call.name, "arguments": call.raw},
        })).collect::<Vec<_>>(),
    })
}

/// What the model said, as text: a string, or the text parts of a list of them.
fn content_of(message: &Value) -> String {
    match &message["content"] {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

/// The draft in the model's last message, or what is wrong with it.
fn draft_in(said: &str) -> Result<Draft, String> {
    draft_from(&answer_in(said)?)
}

/// The JSON object of the answer in `said`, out of what surrounds it.
///
/// The first `{` that begins a JSON object with a `model` in it is where the answer starts, and
/// what follows the object is not read: a code fence, a sentence, or the marker of a tool call that
/// a model left at its end. An object that is not the answer (a sentence with braces in it, an
/// object of the model's own) is passed over, and the next `{` is tried. What is not there is said.
fn answer_in(said: &str) -> Result<Value, String> {
    if said.trim().is_empty() {
        return Err("the message is empty".to_string());
    }
    let mut seen_object = false;
    for (at, _) in said.match_indices('{') {
        let mut values = serde_json::Deserializer::from_str(&said[at..]).into_iter::<Value>();
        if let Some(Ok(value)) = values.next() {
            if value.is_object() {
                seen_object = true;
                if value.get("model").is_some() {
                    return Ok(value);
                }
            }
        }
    }
    Err(if seen_object {
        "there is a JSON object in it, but it has no `model`: the draft has `model` and \
         `requirements`"
            .to_string()
    } else {
        format!("there is no JSON object in it: {}", short(said, 200))
    })
}

/// The start of `text`, on one line, for a sentence that quotes it.
fn short(text: &str, limit: usize) -> String {
    let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= limit {
        one_line
    } else {
        let head: String = one_line.chars().take(limit).collect();
        format!("{head}...")
    }
}

/// What a server answered that is not a success, in a sentence: its status, its own words when it
/// gave any (the form of an error differs by server, and an error that is plain text is read too),
/// and what the person can do about the two statuses that are usually the address or a key.
fn server_said(status: u16, body: &[u8], has_key: bool) -> String {
    let text = String::from_utf8_lossy(body);
    let why = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| {
            value["error"]["message"]
                .as_str()
                .or_else(|| value["error"].as_str())
                .or_else(|| value["message"].as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| text.to_string());
    let mut said = format!("the server answered {status}");
    let why = short(&why, SAID_MAX);
    if !why.is_empty() {
        said.push_str(&format!(": {why}"));
    }
    match status {
        404 => said.push_str(
            " (check the address: it is the server's OpenAI-compatible root, such as \
             http://127.0.0.1:11434/v1)",
        ),
        401 | 403 if !has_key => {
            said.push_str(" (the server wants a key, and this build keeps none for a server yet)")
        }
        _ => {}
    }
    said
}

/// Why a server's models could not be listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelsError {
    /// The server could not be reached or broke; says why.
    Unreachable(String),
    /// The server answered, and not with a list of models.
    Refused(String),
}

impl std::fmt::Display for ModelsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelsError::Unreachable(why) | ModelsError::Refused(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for ModelsError {}

/// The models the server at `endpoint` lists, as it names them: the screen offers them to choose
/// among. Nothing is run: this is the one question of the protocol that is not a model working.
pub fn list_models(endpoint: &Endpoint, bearer: Option<&str>) -> Result<Vec<String>, ModelsError> {
    let response = http_client::send(
        endpoint,
        Request {
            method: "GET",
            path: "/models",
            bearer,
            json: None,
        },
        Instant::now() + LIST_WITHIN,
        &Cancel::new(),
        ANSWER_MAX,
    )
    .map_err(|e| ModelsError::Unreachable(e.to_string()))?;
    if !(200..300).contains(&response.status) {
        return Err(ModelsError::Refused(server_said(
            response.status,
            &response.body,
            bearer.is_some(),
        )));
    }
    let list: Value = serde_json::from_slice(&response.body).map_err(|_| {
        ModelsError::Refused(
            "the server's answer is not a list of models: is the address its OpenAI-compatible \
             root, such as http://127.0.0.1:11434/v1?"
                .to_string(),
        )
    })?;
    let data = list["data"].as_array().ok_or_else(|| {
        ModelsError::Refused(format!(
            "the server's answer is not a list of models: {}",
            tail(&list.to_string(), 120)
        ))
    })?;
    Ok(data
        .iter()
        .filter_map(|model| model["id"].as_str())
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(model: &str) -> String {
        json!({
            "model": {"documents": [{"name": "m.scxml", "text": model}]},
            "requirements": {"manifest_text": "{}\n"},
        })
        .to_string()
    }

    #[test]
    fn a_message_that_is_the_draft_is_the_answer() {
        let answer = answer_in(&form("<scxml/>")).unwrap();

        assert_eq!(answer["model"]["documents"][0]["text"], "<scxml/>");
    }

    #[test]
    fn the_draft_is_taken_out_of_what_a_model_puts_around_it() {
        let draft = form("<scxml/>");
        for (label, said) in [
            ("a code fence", format!("```json\n{draft}\n```")),
            (
                "words before",
                format!("Here is the draft you asked for:\n{draft}"),
            ),
            (
                "words after",
                format!("{draft}\n\nLet me know if you want changes."),
            ),
            // What a model that was run on a real server left at the end of its last message.
            ("a tool call's marker", format!("{draft}<tool_call>")),
            (
                "all of them",
                format!("Done.\n```json\n{draft}\n```\n<tool_call>"),
            ),
        ] {
            assert!(answer_in(&said).is_ok(), "{label}: {said}");
        }
    }

    #[test]
    fn an_object_that_is_not_the_draft_is_passed_over_and_the_next_one_is_tried() {
        let said = format!(
            "The check said {{\"ok\": true}} and so:\n{}",
            form("<scxml/>")
        );

        let answer = answer_in(&said).unwrap();

        assert_eq!(answer["model"]["documents"][0]["text"], "<scxml/>");
    }

    #[test]
    fn braces_that_are_not_json_are_passed_over_too() {
        let said = format!(
            "Use {{braces}} in the {{state}} names. {}",
            form("<scxml/>")
        );

        assert!(answer_in(&said).is_ok());
    }

    #[test]
    fn what_is_not_a_draft_is_said_and_not_guessed_at() {
        let empty = answer_in("   ").unwrap_err();
        let prose = answer_in("I could not write a model for this.").unwrap_err();
        let other = answer_in("{\"ok\": true}").unwrap_err();
        let cut = answer_in("{\"model\": {\"documents\": [").unwrap_err();

        assert!(empty.contains("empty"), "{empty}");
        assert!(prose.contains("no JSON object"), "{prose}");
        assert!(prose.contains("I could not write a model"), "{prose}");
        assert!(other.contains("no `model`"), "{other}");
        // An answer that stopped in the middle (the model ran out of room) is not an object at all.
        assert!(cut.contains("no JSON object"), "{cut}");
    }

    #[test]
    fn a_draft_with_a_model_and_no_documents_is_what_the_draft_says_is_wrong_with_it() {
        let said = json!({"model": {"documents": []}, "requirements": {"manifest_text": "{}"}});

        let wrong = draft_in(&said.to_string()).unwrap_err();

        assert!(wrong.contains("empty"), "{wrong}");
    }

    #[test]
    fn what_a_server_says_of_a_refusal_is_in_its_words_whatever_its_form() {
        for (body, said) in [
            (
                r#"{"error":{"message":"model not found","type":"x"}}"#,
                "model not found",
            ),
            (r#"{"error":"model 'x' not found"}"#, "model 'x' not found"),
            (r#"{"message":"bad request"}"#, "bad request"),
            ("plain words", "plain words"),
        ] {
            let sentence = server_said(400, body.as_bytes(), false);

            assert_eq!(
                sentence,
                format!("the server answered 400: {said}"),
                "{body}"
            );
        }
    }

    #[test]
    fn the_two_statuses_that_are_usually_the_address_or_a_key_say_which_to_check() {
        let missing = server_said(404, b"404 page not found", false);
        let wants_a_key = server_said(401, b"{\"error\":\"unauthorized\"}", false);
        let has_one = server_said(401, b"{\"error\":\"unauthorized\"}", true);

        assert!(missing.contains("http://127.0.0.1:11434/v1"), "{missing}");
        assert!(wants_a_key.contains("wants a key"), "{wants_a_key}");
        // A key that was given and refused is not one the person is told to give.
        assert!(!has_one.contains("wants a key"), "{has_one}");
    }

    #[test]
    fn what_a_server_says_is_kept_short_and_on_one_line() {
        let long = "word ".repeat(500);

        let sentence = server_said(500, long.as_bytes(), false);

        assert!(sentence.chars().count() < 460, "{}", sentence.len());
        assert!(!sentence.contains('\n'));
        assert!(sentence.ends_with("..."), "{sentence}");
    }

    #[test]
    fn calls_are_read_with_their_arguments_as_the_model_wrote_them_and_an_id_when_it_gave_none() {
        let message = json!({"tool_calls": [
            {"id": "call_a", "function": {"name": "works_read", "arguments": "{\"work\":\"w\"}"}},
            {"function": {"name": "decisions", "arguments": ""}},
            {"id": "call_c", "function": {"name": "x", "arguments": "{not json"}},
            {"id": "call_d", "function": {"name": "y", "arguments": {"already": "an object"}}},
        ]});

        let calls = calls_of(&message, 4);

        assert_eq!(calls[0].id, "call_a");
        assert_eq!(calls[0].arguments, Ok(json!({"work": "w"})));
        // No id: one is made, and it is the same one the result is answered under.
        assert_eq!(calls[1].id, "call_4_1");
        assert_eq!(calls[1].arguments, Ok(Value::Null));
        assert!(calls[2].arguments.is_err());
        assert_eq!(calls[2].raw, "{not json");
        assert_eq!(calls[3].arguments, Ok(json!({"already": "an object"})));
    }

    #[test]
    fn the_message_put_back_carries_what_was_said_and_the_calls_and_none_of_the_models_reasoning() {
        let message = json!({
            "role": "assistant",
            "content": null,
            "reasoning": "private",
            "tool_calls": [{"id": "c1", "function": {"name": "works_read", "arguments": "{}"}}],
        });
        let calls = calls_of(&message, 0);

        let echoed = echo_of(&message, &calls);

        assert_eq!(echoed["role"], "assistant");
        assert_eq!(echoed["content"], "");
        assert_eq!(echoed["tool_calls"][0]["id"], "c1");
        assert_eq!(echoed["tool_calls"][0]["type"], "function");
        assert_eq!(echoed["tool_calls"][0]["function"]["arguments"], "{}");
        assert!(echoed.get("reasoning").is_none());
    }

    #[test]
    fn what_the_model_said_is_text_whether_it_is_given_as_a_string_or_as_parts() {
        assert_eq!(content_of(&json!({"content": "words"})), "words");
        assert_eq!(content_of(&json!({"content": null})), "");
        assert_eq!(
            content_of(
                &json!({"content": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]})
            ),
            "ab"
        );
    }

    #[test]
    fn the_instructions_are_named_and_the_same_for_every_server() {
        let one = Local::new(
            Endpoint::parse("http://127.0.0.1:1/v1").unwrap(),
            AuthorServer {
                command: "/a".into(),
                args: vec![],
                env: vec![],
            },
            LocalConfig::for_model("m"),
        );
        let other = Local::new(
            Endpoint::parse("http://10.0.0.5:9/x").unwrap(),
            AuthorServer {
                command: "/b".into(),
                args: vec![],
                env: vec![],
            },
            LocalConfig::for_model("another"),
        );

        let name = one.instructions().unwrap();

        assert!(name.starts_with("local/"), "{name}");
        assert_eq!(name.len(), "local/".len() + 12);
        // What names them is the task and the form, not where the server is or which model it runs.
        assert_eq!(other.instructions().unwrap(), name);
    }
}
