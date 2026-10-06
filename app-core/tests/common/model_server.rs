// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The two stand-ins a model server run is held against: a model server that answers each request
//! from a script and records what it was sent, and an authoring server that is a shell script
//! answering by method. Neither is a model: what is held is the conversation the application has
//! with them, which is the same whatever server a person runs.
//!
//! Unix only: the authoring server stand-in is a shell script.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sce_app_core::claude_code::AuthorServer;
use serde_json::{json, Value};

use super::write_program;

// ---- the model server ------------------------------------------------------------------------

/// What was sent to the model server: the head of the request as text, and its body.
#[derive(Debug, Clone)]
pub struct Sent {
    pub head: String,
    pub body: Value,
}

/// What the model server does with the next request.
pub enum Script {
    Reply(u16, String),
    /// Reads the request and then says nothing, noting whether the other end went away.
    Hang,
}

pub struct ChatServer {
    pub address: String,
    pub seen: Arc<Mutex<Vec<Sent>>>,
    pub hung_up: Arc<AtomicBool>,
}

impl ChatServer {
    pub fn requests(&self) -> Vec<Sent> {
        self.seen.lock().unwrap().clone()
    }

    /// The messages of the request that was the `n`th.
    pub fn messages(&self, n: usize) -> Vec<Value> {
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

pub fn chat_server(script: Vec<Script>) -> ChatServer {
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
pub fn calls(calls: &[(Option<&str>, &str, &str)]) -> Script {
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
pub fn says(content: &str) -> Script {
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
pub fn draft(model: &str) -> String {
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
pub fn authoring_server(folder: &Path, tools: &[&str]) -> AuthorServer {
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
         [ \"$1\" = \"--check\" ] && exit 0\n\
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
pub fn requirement_set_gives(folder: &Path, answer: &Value) {
    put_answer(folder, "requirement_set.json", answer);
}

/// The same for the `n`th call (counting from 1) alone, which says it failed when `fails`: a tool
/// that failed can still have written JSON in its text.
pub fn requirement_set_gives_at(folder: &Path, n: u32, answer: &Value, fails: bool) {
    put_answer(folder, &format!("requirement_set.{n}.json"), answer);
    if fails {
        std::fs::write(folder.join(format!("requirement_set.{n}.fails")), "").unwrap();
    }
}

/// From now on the stand-in's tool `name` (one that has no answer of its own) answers with
/// `answer`, in place of `ok:` and its name.
pub fn tool_gives(folder: &Path, name: &str, answer: &Value) {
    put_answer(folder, &format!("answer.{name}.json"), answer);
}

/// The file holds the answer as a JSON string literal, which is how a tool's text is put in the
/// reply.
fn put_answer(folder: &Path, file: &str, answer: &Value) {
    let literal = serde_json::to_string(&answer.to_string()).unwrap();
    std::fs::write(folder.join(file), literal).unwrap();
}

/// What the authoring server was called with, one request to a line.
pub fn called(folder: &Path) -> String {
    std::fs::read_to_string(folder.join("calls")).unwrap_or_default()
}
