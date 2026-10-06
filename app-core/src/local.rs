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

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::claude_code::AuthorServer;
use crate::client_run::{
    blank_job, draft_from, prompt, span_words, tail, Scratch, AUTHOR_TOOLS, SYSTEM_PROMPT,
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

/// Said to the model of what its last message is: the draft, written out as an example to be filled
/// in. A description of the form (a JSON Schema) is what the other clients are given, through a
/// channel of their own; a model that is given one in its conversation says it back (measured on a
/// real server: its last message was the schema), and an example is what it copies the shape of.
const ANSWER_FORM: &str = "Your last message is the draft: one JSON object and nothing else (no \
code fence, no words before or after it), written like this, with the real document in place of \
what is in angle brackets:\n\
{\"model\": {\"documents\": [{\"name\": \"<file name, such as door.scxml>\", \"text\": \"<the \
whole SCXML document, exactly as validate_scxml accepted it>\"}]}}\n\
When the model is several documents that import each other, list each of them in `documents` \
and add `\"entry\": \"<the name of the one to start from>\"` inside `model`. The requirement list \
is not part of what you write: the application takes it from your last scxml_requirement_set \
call, so call that tool and leave `requirements` out of the draft.";

/// What a model is told of its tools: the authoring instructions it is also given describe tools
/// that save and that take a request, which are for the clients that do those things themselves,
/// and a model that reads of a tool tries it (one was seen calling `works_begin_generation`).
fn tools_words(names: &[&str]) -> String {
    format!(
        "You have exactly these tools and no others: {}. The authoring instructions below also \
         describe tools that save, take a request or accept a design. You have none of them, and \
         the application does those parts.",
        names.join(", ")
    )
}

/// Everything a run says to the model that does not depend on the work: what [`Local::instructions`]
/// is the name of.
fn told() -> String {
    format!(
        "{SYSTEM_PROMPT}\n{}\n{ANSWER_FORM}\n{}\n{}\n{REPAIRS}\n{}\n{SCHEMA_ECHO}",
        tools_words(&AUTHOR_TOOLS),
        prompt(&blank_job()),
        AUTHOR_TOOLS.join(","),
        repair_words("<what was wrong>"),
    )
}

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

/// One step of a run, for whoever watches it (a test, a log, a progress line): what was asked of
/// the model, what it said, what its tools did, and why a message was not the draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The model is asked, for the `turn`th time, with `messages` in the conversation so far.
    Asked { turn: u32, messages: usize },
    /// What the model said, and the tools it called (each as its name and its arguments as the
    /// model wrote them).
    Said {
        turn: u32,
        words: String,
        calls: Vec<(String, String)>,
    },
    /// A tool was run for the model: which, whether it said it failed, and what it gave back (the
    /// words the model was handed).
    Tool {
        name: String,
        failed: bool,
        words: String,
    },
    /// The model's message was not the draft, and why.
    NotTheDraft { why: String },
}

/// Somebody who is told each step of a run.
type Sink = Arc<dyn Fn(&Step) + Send + Sync>;

/// Where the steps of a run go, when anybody wants them.
#[derive(Clone, Default)]
struct Trace(Option<Sink>);

impl std::fmt::Debug for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "Trace(on)"
        } else {
            "Trace(off)"
        })
    }
}

impl Trace {
    /// Say a step, which is only worked out when somebody is listening.
    fn say(&self, step: impl FnOnce() -> Step) {
        if let Some(sink) = &self.0 {
            sink(&step());
        }
    }
}

/// What a tool call came to: what the tool said, and whether it said it failed. Whether it failed
/// is known where the call is made and is carried along, not read back out of the words.
struct ToolWords {
    words: String,
    failed: bool,
}

impl ToolWords {
    fn gave(words: String) -> Self {
        Self {
            words,
            failed: false,
        }
    }

    fn failed(words: String) -> Self {
        Self {
            words,
            failed: true,
        }
    }

    /// The words the model is handed: a failure is said to be one.
    fn handed_back(&self) -> String {
        if self.failed {
            format!("Error: {}", self.words)
        } else {
            self.words.clone()
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
    trace: Trace,
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
            trace: Trace::default(),
        }
    }

    /// The same, giving `key` to the server as the credential it asked for.
    pub fn with_bearer(mut self, key: String) -> Self {
        self.bearer = Some(key);
        self
    }

    /// The same, telling `sink` each [`Step`] of a run as it happens.
    pub fn with_trace(mut self, sink: impl Fn(&Step) + Send + Sync + 'static) -> Self {
        self.trace = Trace(Some(Arc::new(sink)));
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

    /// What a tool call of the model comes to, as the words handed back to it and whether they say
    /// it failed. A call that is not one that can be made is an answer to the model and not the
    /// end of the run.
    fn run_tool(
        &self,
        mcp: &mut McpClient,
        tools: &[Tool],
        call: &Call,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<ToolWords, GenerateError> {
        let Some(tool) = tools.iter().find(|tool| tool.name == call.name) else {
            let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
            return Ok(ToolWords::failed(format!(
                "there is no tool named `{}`. The tools are: {}.",
                call.name,
                names.join(", ")
            )));
        };
        let arguments = match &call.arguments {
            Ok(Value::Object(object)) => Value::Object(object.clone()),
            // A tool that takes nothing is called with nothing.
            Ok(Value::Null) => json!({}),
            Ok(_) => {
                return Ok(ToolWords::failed(
                    "the arguments of a tool are one JSON object.".to_string(),
                ))
            }
            Err(why) => {
                return Ok(ToolWords::failed(format!(
                    "the arguments are not valid JSON: {why}"
                )));
            }
        };
        let called = mcp
            .call(&tool.name, &arguments, deadline, cancel)
            .map_err(|e| self.mcp_failure(e))?;
        Ok(match (called.is_error, called.text.is_empty()) {
            (true, _) => ToolWords::failed(called.text),
            (false, true) => ToolWords::gave("(the tool gave no output)".to_string()),
            (false, false) => ToolWords::gave(called.text),
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
        let digest = Revision::of(told().as_bytes()).to_string();
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
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        let mut system = format!("{SYSTEM_PROMPT}\n\n{}", tools_words(&names));
        if let Some(instructions) = mcp.instructions() {
            system.push_str("\n\n");
            system.push_str(instructions);
        }
        system.push_str(&format!("\n\n{ANSWER_FORM}"));
        // What the tool that builds the requirement list gave last: it is the list, and the model
        // is not asked to write it out again.
        let mut listed: Option<Listed> = None;
        let mut messages = vec![
            json!({"role": "system", "content": system}),
            json!({"role": "user", "content": prompt(job)}),
        ];
        let mut repairs = 0;
        for turn in 0..self.config.max_turns {
            self.trace.say(|| Step::Asked {
                turn,
                messages: messages.len(),
            });
            let message = self.ask(&messages, &offered, deadline, cancel)?;
            let calls = calls_of(&message, turn);
            self.trace.say(|| Step::Said {
                turn,
                words: content_of(&message),
                calls: calls
                    .iter()
                    .map(|call| (call.name.clone(), call.raw.clone()))
                    .collect(),
            });
            if !calls.is_empty() {
                messages.push(echo_of(&message, &calls));
                for call in &calls {
                    let done = self.run_tool(&mut mcp, &tools, call, deadline, cancel)?;
                    self.trace.say(|| Step::Tool {
                        name: call.name.clone(),
                        failed: done.failed,
                        words: done.handed_back(),
                    });
                    if call.name == REQUIREMENT_SET && !done.failed {
                        listed = Listed::in_words(&done.words).or(listed);
                    }
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": done.handed_back(),
                    }));
                }
                continue;
            }
            let said = content_of(&message);
            match draft_in(&said, listed.as_ref()) {
                Ok(draft) => return Ok(draft),
                Err(wrong) if repairs < REPAIRS => {
                    self.trace.say(|| Step::NotTheDraft { why: wrong.clone() });
                    repairs += 1;
                    messages.push(json!({"role": "assistant", "content": said}));
                    messages.push(json!({"role": "user", "content": repair_words(&wrong)}));
                }
                Err(wrong) => {
                    self.trace.say(|| Step::NotTheDraft { why: wrong.clone() });
                    return Err(GenerateError::Unusable(wrong));
                }
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

/// The requirement list that the tool which builds it gave: what the application uses as the
/// draft's `requirements`, so that a list of some thousands of characters is not copied out by a
/// model whose copy can be wrong. Taken from what the tool's own answer says it returns
/// (`manifest_text` and `sidecar_text`, in the tool's description).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Listed {
    manifest: String,
    sidecar: Option<String>,
}

/// The authoring server's tool that builds the requirement list.
const REQUIREMENT_SET: &str = "scxml_requirement_set";

impl Listed {
    /// The list in the words of a call of the tool, when they are its answer.
    fn in_words(words: &str) -> Option<Listed> {
        let answer: Value = serde_json::from_str(words).ok()?;
        let manifest = answer["manifest_text"].as_str().filter(|m| !m.is_empty())?;
        Some(Listed {
            manifest: manifest.to_string(),
            sidecar: answer["sidecar_text"].as_str().map(str::to_string),
        })
    }

    /// `answer` with this list as its `requirements`, whatever it said of them.
    fn put_in(&self, answer: &mut Value) {
        let mut requirements = json!({"manifest_text": self.manifest});
        if let Some(sidecar) = &self.sidecar {
            requirements["sidecar_text"] = json!(sidecar);
        }
        answer["requirements"] = requirements;
    }
}

/// What a model that said the description of the form back is told.
const SCHEMA_ECHO: &str = "that is the description of the form (a JSON Schema), not a draft \
written in it: write the draft itself, an object with `model` in it";

/// The draft in the model's last message, or what is wrong with it. `listed` is the requirement
/// list the tool gave, which is the draft's whatever the model wrote of it.
fn draft_in(said: &str, listed: Option<&Listed>) -> Result<Draft, String> {
    let mut first_wrong = None;
    for mut answer in answers_in(said)? {
        if let Some(listed) = listed {
            listed.put_in(&mut answer);
        }
        match draft_from(&answer) {
            Ok(draft) => return Ok(draft),
            Err(wrong) => {
                first_wrong.get_or_insert(wrong);
            }
        }
    }
    Err(first_wrong.unwrap_or_else(|| {
        "there is a JSON object in it, but it has no `model`: the draft has `model` in it"
            .to_string()
    }))
}

/// The JSON objects in `said` that may be the draft, out of what surrounds them: those that have
/// a `model` in them, in the order they begin.
///
/// Each `{` begins a try: what is read from it is the first JSON value, and what follows is not
/// read (a code fence, a sentence, or the marker of a tool call that a model left at its end).
/// An object that is not the draft (a sentence with braces in it, a check's result the model
/// quoted) is not one, and a draft is not told from the things inside it by having a `model`
/// key, so the caller reads each as a draft and takes the first that is one. A message that is the
/// description of the form is said to be that, and what is not there is said.
fn answers_in(said: &str) -> Result<Vec<Value>, String> {
    if said.trim().is_empty() {
        return Err("the message is empty".to_string());
    }
    let mut objects = Vec::new();
    for (at, _) in said.match_indices('{') {
        let mut values = serde_json::Deserializer::from_str(&said[at..]).into_iter::<Value>();
        if let Some(Ok(value)) = values.next() {
            if value.is_object() {
                objects.push((at, value));
            }
        }
    }
    // A schema's own top level is `properties` and `type`; no draft has them there.
    if let Some((_, first)) = objects.first() {
        if first.get("properties").is_some() && first.get("type").is_some() {
            return Err(SCHEMA_ECHO.to_string());
        }
    }
    if objects.is_empty() {
        return Err(format!(
            "there is no JSON object in it: {}",
            short(said, 200)
        ));
    }
    let candidates: Vec<Value> = objects
        .into_iter()
        .map(|(_, value)| value)
        .filter(|value| value.get("model").is_some())
        .collect();
    if candidates.is_empty() {
        Err(
            "there is a JSON object in it, but it has no `model`: the draft has `model` in it"
                .to_string(),
        )
    } else {
        Ok(candidates)
    }
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
        let made = draft_in(&form("<scxml/>"), None).unwrap();

        assert_eq!(made.model.entry_text(), "<scxml/>");
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
            assert!(draft_in(&said, None).is_ok(), "{label}: {said}");
        }
    }

    #[test]
    fn an_object_that_is_not_the_draft_is_passed_over_and_the_next_one_is_tried() {
        let said = format!(
            "The check said {{\"ok\": true}} and so:\n{}",
            form("<scxml/>")
        );

        let made = draft_in(&said, None).unwrap();

        assert_eq!(made.model.entry_text(), "<scxml/>");
    }

    #[test]
    fn an_object_that_has_a_model_and_is_not_a_draft_is_not_taken_for_one() {
        // A `model` key is not what makes a draft: a check's result the model quoted can have one.
        let said = format!(
            "{{\"model\": \"a state machine\"}}\nand here is the draft:\n{}",
            form("<scxml/>")
        );

        let made = draft_in(&said, None).unwrap();

        assert_eq!(made.model.entry_text(), "<scxml/>");
    }

    #[test]
    fn braces_that_are_not_json_are_passed_over_too() {
        let said = format!(
            "Use {{braces}} in the {{state}} names. {}",
            form("<scxml/>")
        );

        assert!(draft_in(&said, None).is_ok());
    }

    #[test]
    fn what_is_not_a_draft_is_said_and_not_guessed_at() {
        let empty = draft_in("   ", None).err().unwrap();
        let prose = draft_in("I could not write a model for this.", None)
            .err()
            .unwrap();
        let other = draft_in("{\"ok\": true}", None).err().unwrap();
        let cut = draft_in("{\"model\": {\"documents\": [", None)
            .err()
            .unwrap();

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

        let wrong = draft_in(&said.to_string(), None).unwrap_err();

        assert!(wrong.contains("empty"), "{wrong}");
    }

    #[test]
    fn a_message_that_is_the_description_of_the_form_is_said_to_be_that() {
        // What a model said on a real server: the schema it was given, back as its last message.
        let schema = json!({
            "properties": {
                "model": {"properties": {"documents": {"type": "array"}}, "type": "object"},
                "requirements": {"type": "object"},
            },
            "required": ["model", "requirements"],
            "type": "object",
        });

        let wrong = draft_in(&schema.to_string(), None).unwrap_err();

        assert!(wrong.contains("JSON Schema"), "{wrong}");
        assert!(wrong.contains("write the draft itself"), "{wrong}");
    }

    #[test]
    fn the_requirement_list_is_the_one_the_tool_gave_whatever_the_model_wrote_of_it() {
        let listed = Listed {
            manifest: "{\"doc_id\":\"from-tool\"}\n".to_string(),
            sidecar: Some("{\"R1\":\"the door closes\"}".to_string()),
        };
        let without = json!({"model": {"documents": [{"name": "m.scxml", "text": "<scxml/>"}]}});
        let retyped = json!({
            "model": {"documents": [{"name": "m.scxml", "text": "<scxml/>"}]},
            "requirements": {"manifest_text": "a copy with a mistake in it"},
        });

        for said in [without, retyped] {
            let made = draft_in(&said.to_string(), Some(&listed)).unwrap();

            assert_eq!(made.requirements.manifest, "{\"doc_id\":\"from-tool\"}\n");
            assert_eq!(
                made.requirements.sidecar.as_deref(),
                Some("{\"R1\":\"the door closes\"}")
            );
        }
    }

    #[test]
    fn with_no_list_from_the_tool_the_requirements_are_the_models_and_their_absence_is_said() {
        let without = json!({"model": {"documents": [{"name": "m.scxml", "text": "<scxml/>"}]}});

        let wrong = draft_in(&without.to_string(), None).unwrap_err();

        assert!(wrong.contains("requirement list is missing"), "{wrong}");
    }

    #[test]
    fn the_list_is_read_out_of_the_words_of_the_tool_and_not_out_of_other_words() {
        let gave = json!({
            "manifest_text": "{\"doc_id\":\"d\"}\n",
            "sidecar_text": "s",
            "unclaimed_sentences": [],
        })
        .to_string();

        assert_eq!(
            Listed::in_words(&gave),
            Some(Listed {
                manifest: "{\"doc_id\":\"d\"}\n".to_string(),
                sidecar: Some("s".to_string()),
            })
        );
        assert_eq!(Listed::in_words("ok:scxml_requirement_set"), None);
        assert_eq!(Listed::in_words("{\"manifest_text\": \"\"}"), None);
        assert_eq!(Listed::in_words("{\"other\": 1}"), None);
    }

    #[test]
    fn the_model_is_told_the_tools_it_has_and_that_the_instructions_name_more() {
        let words = tools_words(&["works_read", "validate_scxml"]);

        assert!(words.contains("works_read, validate_scxml"), "{words}");
        assert!(words.contains("You have none of them"), "{words}");
    }

    #[test]
    fn what_the_instructions_are_named_by_is_everything_the_model_is_told() {
        let told = told();

        for (what, words) in [
            ("the system prompt", SYSTEM_PROMPT.to_string()),
            ("the form of the answer", ANSWER_FORM.to_string()),
            ("the tools it has", tools_words(&AUTHOR_TOOLS)),
            ("the first message", prompt(&blank_job())),
            ("the repair", repair_words("<what was wrong>")),
            ("what is said of the schema", SCHEMA_ECHO.to_string()),
            ("the number of repairs", format!("\n{REPAIRS}\n")),
        ] {
            assert!(told.contains(&words), "{what} is not in what is named");
        }
    }

    #[test]
    fn the_form_is_given_as_an_example_to_fill_in_and_not_as_a_schema() {
        assert!(
            ANSWER_FORM.contains("<the whole SCXML document"),
            "{ANSWER_FORM}"
        );
        assert!(!ANSWER_FORM.contains("\"properties\""), "{ANSWER_FORM}");
        assert!(!ANSWER_FORM.contains("\"required\""), "{ANSWER_FORM}");
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
