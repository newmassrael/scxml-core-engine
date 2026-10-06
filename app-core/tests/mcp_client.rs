// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The client of the authoring server: what it asks, what it does with what comes back, and how
//! it ends when the server is slow, silent or gone.
//!
//! The server here is a shell script that answers each request by its method, so these hold the
//! protocol and the process handling against a stand-in, with no Python in the way.
//!
//! Unix only: the stand-ins are shell scripts.

#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use sce_app_core::claude_code::AuthorServer;
use sce_app_core::mcp_client::{Called, McpClient, McpError};
use sce_app_core::runner::Cancel;
use serde_json::{json, Value};

use common::{scratch, write_program};

/// A stand-in for the server that answers `initialize` and `tools/list` and, for `tools/call`,
/// writes the request to `calls` and then does `behaviour`.
fn server(folder: &Path, behaviour: &str) -> AuthorServer {
    let program = folder.join("server.sh");
    let script = format!(
        "#!/bin/sh\n\
         while IFS= read -r line; do\n\
           id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\\([0-9][0-9]*\\).*/\\1/p')\n\
           case \"$line\" in\n\
             *'\"method\":\"initialize\"'*)\n\
               printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{\"tools\":{{}}}},\"serverInfo\":{{\"name\":\"fake\",\"version\":\"1\"}},\"instructions\":\"Use the tools.\"}}}}\\n' \"$id\" ;;\n\
             *'\"method\":\"tools/list\"'*)\n\
               printf '{{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{{\"tools\":[{{\"name\":\"works_read\",\"description\":\"Read a work.\",\"inputSchema\":{{\"type\":\"object\",\"properties\":{{\"work\":{{\"type\":\"string\"}}}}}}}},{{\"name\":\"bare\"}}]}}}}\\n' \"$id\" ;;\n\
             *'\"method\":\"tools/call\"'*)\n\
               printf '%s\\n' \"$line\" >> '{calls}'\n\
               {behaviour}\n\
               ;;\n\
           esac\n\
         done\n",
        calls = folder.join("calls").display(),
    );
    write_program(&program, &script);
    AuthorServer {
        command: program,
        args: vec![],
        env: vec![],
    }
}

const ANSWER: &str = "printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"the work\"},{\"type\":\"text\",\"text\":\"more\"}],\"isError\":false}}\\n' \"$id\"";

fn within(seconds: u64) -> Instant {
    Instant::now() + Duration::from_secs(seconds)
}

fn started(label: &str, behaviour: &str) -> (McpClient, PathBuf) {
    let folder = scratch(label);
    let client = McpClient::start(
        &server(&folder, behaviour),
        &folder,
        within(20),
        &Cancel::new(),
    )
    .expect("the server starts");
    (client, folder)
}

#[test]
fn it_says_what_the_server_says_of_how_to_use_it() {
    let (client, _) = started("mcp-instructions", ANSWER);

    assert_eq!(client.instructions(), Some("Use the tools."));
}

#[test]
fn it_lists_the_tools_with_what_they_say_of_themselves_and_a_schema_when_they_say_none() {
    let (mut client, _) = started("mcp-tools", ANSWER);

    let tools = client.tools(within(10), &Cancel::new()).unwrap();

    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "works_read");
    assert_eq!(tools[0].description, "Read a work.");
    assert_eq!(tools[0].schema["properties"]["work"]["type"], "string");
    // A tool that says nothing of its arguments takes none.
    assert_eq!(tools[1].name, "bare");
    assert_eq!(tools[1].schema, json!({"type": "object", "properties": {}}));
}

#[test]
fn a_call_names_the_tool_and_gives_the_arguments_and_the_words_it_answers_are_put_together() {
    let (mut client, folder) = started("mcp-call", ANSWER);

    let called = client
        .call(
            "works_read",
            &json!({"work": "door-lock"}),
            within(10),
            &Cancel::new(),
        )
        .unwrap();

    assert_eq!(
        called,
        Called {
            text: "the work\nmore".to_string(),
            is_error: false
        }
    );
    let sent = fs::read_to_string(folder.join("calls")).unwrap();
    assert!(sent.contains("\"name\":\"works_read\""), "{sent}");
    assert!(
        sent.contains("\"arguments\":{\"work\":\"door-lock\"}"),
        "{sent}"
    );
}

#[test]
fn a_tool_that_says_it_failed_is_an_answer_and_not_an_error_of_the_server() {
    let (mut client, _) = started(
        "mcp-is-error",
        "printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"no such work\"}],\"isError\":true}}\\n' \"$id\"",
    );

    let called = client
        .call("works_read", &json!({}), within(10), &Cancel::new())
        .unwrap();

    assert_eq!(
        called,
        Called {
            text: "no such work".to_string(),
            is_error: true
        }
    );
}

#[test]
fn what_the_server_says_that_is_not_the_answer_is_passed_over() {
    let (mut client, _) = started(
        "mcp-noise",
        // A line that is not JSON, a notification, and an answer to another request, then the answer.
        "printf 'starting up\\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\"}\\n{\"jsonrpc\":\"2.0\",\"id\":999,\"result\":{}}\\n'; printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"ok\"}]}}\\n' \"$id\"",
    );

    let called = client
        .call("works_read", &json!({}), within(10), &Cancel::new())
        .unwrap();

    assert_eq!(called.text, "ok");
}

#[test]
fn an_error_the_server_gives_is_its_refusal_in_its_words() {
    let (mut client, _) = started(
        "mcp-refused",
        "printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"error\":{\"code\":-32602,\"message\":\"arguments has to be an object\"}}\\n' \"$id\"",
    );

    let refused = client
        .call("works_read", &json!({}), within(10), &Cancel::new())
        .unwrap_err();

    assert_eq!(
        refused,
        McpError::Refused("arguments has to be an object".to_string())
    );
}

#[test]
fn a_server_that_does_not_answer_is_stopped_waiting_for_when_told_to() {
    let (mut client, _) = started("mcp-silent", "sleep 30");
    let cancel = Cancel::new();
    let stopper = {
        let cancel = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            cancel.cancel();
        })
    };

    let began = Instant::now();
    let stopped = client.call("works_read", &json!({}), within(60), &cancel);
    stopper.join().unwrap();

    assert_eq!(stopped, Err(McpError::Cancelled));
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "{:?}",
        began.elapsed()
    );
}

#[test]
fn a_server_that_takes_longer_than_the_time_given_is_stopped_waiting_for_at_that_time() {
    let (mut client, _) = started("mcp-slow", "sleep 30");

    let began = Instant::now();
    let late = client.call(
        "works_read",
        &json!({}),
        Instant::now() + Duration::from_millis(400),
        &Cancel::new(),
    );

    assert_eq!(late, Err(McpError::TimedOut));
    assert!(
        began.elapsed() < Duration::from_secs(3),
        "{:?}",
        began.elapsed()
    );
}

#[test]
fn a_server_that_goes_away_is_said_to_be_gone_with_what_it_last_said() {
    let (mut client, _) = started(
        "mcp-gone",
        "echo 'a module it needs is missing' >&2; exit 3",
    );

    let gone = client
        .call("works_read", &json!({}), within(10), &Cancel::new())
        .unwrap_err();

    let McpError::Gone(why) = gone else {
        panic!("expected Gone, got {gone:?}");
    };
    assert!(why.contains("a module it needs is missing"), "{why}");
}

#[test]
fn a_program_that_cannot_be_started_is_said_so_and_one_that_is_not_a_server_is_said_so() {
    let folder = scratch("mcp-not-a-server");
    let missing = AuthorServer {
        command: PathBuf::from("/nowhere/sce-author-mcp"),
        args: vec![],
        env: vec![],
    };
    // Reads nothing and says nothing: the handshake is never answered, and the time runs out.
    let deaf = folder.join("deaf.sh");
    write_program(&deaf, "#!/bin/sh\nsleep 30\n");

    let not_started = McpClient::start(&missing, &folder, within(5), &Cancel::new())
        .err()
        .unwrap();
    let deaf = McpClient::start(
        &AuthorServer {
            command: deaf,
            args: vec![],
            env: vec![],
        },
        &folder,
        Instant::now() + Duration::from_millis(500),
        &Cancel::new(),
    )
    .err()
    .unwrap();

    assert!(matches!(not_started, McpError::Gone(_)), "{not_started:?}");
    assert_eq!(deaf, McpError::TimedOut);
}

#[test]
fn the_server_is_started_with_the_environment_it_was_given_and_is_gone_when_the_client_is() {
    let folder = scratch("mcp-env");
    let program = folder.join("env.sh");
    write_program(
        &program,
        "#!/bin/sh\n\
         [ \"$1\" = \"--version\" ] && exit 0\n\
         printf '%s' \"$SCE_WORKS_DIR\" > \"$(dirname \"$0\")/env\"\n\
         echo $$ > \"$(dirname \"$0\")/pid\"\n\
         while IFS= read -r line; do\n\
           id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\\([0-9][0-9]*\\).*/\\1/p')\n\
           case \"$line\" in *'\"method\":\"initialize\"'*) printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{}}\\n' \"$id\";; esac\n\
         done\n",
    );
    let author = AuthorServer {
        command: program,
        args: vec![],
        env: vec![("SCE_WORKS_DIR".to_string(), "/works".to_string())],
    };

    let client = McpClient::start(&author, &folder, within(10), &Cancel::new()).unwrap();
    let pid = fs::read_to_string(folder.join("pid"))
        .unwrap()
        .trim()
        .to_string();
    let env = fs::read_to_string(folder.join("env")).unwrap();
    drop(client);

    assert_eq!(env, "/works");
    // The process was ended with the client.
    let gone = (0..50).any(|_| {
        let there = Path::new(&format!("/proc/{pid}")).exists();
        if there {
            thread::sleep(Duration::from_millis(50));
        }
        !there
    });
    assert!(gone, "the server {pid} is still running");
}

#[test]
fn a_server_with_no_instructions_has_none() {
    let folder = scratch("mcp-no-instructions");
    let program = folder.join("quiet.sh");
    write_program(
        &program,
        "#!/bin/sh\n\
         [ \"$1\" = \"--version\" ] && exit 0\n\
         while IFS= read -r line; do\n\
           id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\\([0-9][0-9]*\\).*/\\1/p')\n\
           case \"$line\" in *'\"method\":\"initialize\"'*) printf '{\"jsonrpc\":\"2.0\",\"id\":%s,\"result\":{\"instructions\":\"   \"}}\\n' \"$id\";; esac\n\
         done\n",
    );

    let client = McpClient::start(
        &AuthorServer {
            command: program,
            args: vec![],
            env: vec![],
        },
        &folder,
        within(10),
        &Cancel::new(),
    )
    .unwrap();

    assert_eq!(client.instructions(), None);
    let _: Value = json!(null);
}

/// A server that answers the handshake and the list of its tools, and then reads nothing more:
/// what a call that sends more than a pipe holds waits behind.
fn deaf_server(folder: &Path) -> AuthorServer {
    let program = folder.join("deaf.sh");
    write_program(
        &program,
        "#!/bin/sh\n\
         IFS= read -r line\n\
         printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{\"tools\":{}},\"serverInfo\":{\"name\":\"deaf\",\"version\":\"1\"}}}\\n'\n\
         IFS= read -r line\n\
         IFS= read -r line\n\
         printf '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"works_read\"}]}}\\n'\n\
         exec sleep 60\n",
    );
    AuthorServer {
        command: program,
        args: vec![],
        env: vec![],
    }
}

/// A client that has the tools of a server that then stops reading.
fn deaf_client(label: &str) -> McpClient {
    let folder = scratch(label);
    let mut client = McpClient::start(&deaf_server(&folder), &folder, within(20), &Cancel::new())
        .expect("the server starts");
    client.tools(within(10), &Cancel::new()).unwrap();
    client
}

/// How long a call that is told to stop, or whose time is up, may go on.
const PROMPTLY: Duration = Duration::from_secs(3);

#[test]
fn a_server_that_closes_its_input_and_lives_on_is_said_to_be_gone_and_not_waited_for() {
    let folder = scratch("mcp-closed-input");
    let program = folder.join("closed.sh");
    write_program(
        &program,
        "#!/bin/sh\n\
         IFS= read -r line\n\
         printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{\"tools\":{}},\"serverInfo\":{\"name\":\"closed\",\"version\":\"1\"}}}\\n'\n\
         IFS= read -r line\n\
         IFS= read -r line\n\
         exec 0<&-\n\
         printf '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"works_read\"}]}}\\n'\n\
         exec sleep 60\n",
    );
    let mut client = McpClient::start(
        &AuthorServer {
            command: program,
            args: vec![],
            env: vec![],
        },
        &folder,
        within(20),
        &Cancel::new(),
    )
    .expect("the server starts");
    client.tools(within(10), &Cancel::new()).unwrap();

    let started = Instant::now();
    let called = client.call(
        "works_read",
        &json!({"work": "w"}),
        within(30),
        &Cancel::new(),
    );

    // Not waited for until its time was up: the write that could not be made says so.
    assert!(matches!(called, Err(McpError::Gone(_))), "{called:?}");
    assert!(started.elapsed() < PROMPTLY, "{:?}", started.elapsed());
}

#[test]
fn a_call_that_is_being_sent_is_stopped_at_its_time_and_when_told_to_stop() {
    // Longer than a pipe holds, so that sending it waits on a server that reads none of it.
    let arguments = json!({ "work": "x".repeat(16 * 1024 * 1024) });
    let mut late = deaf_client("mcp-deaf-late");
    let mut told = deaf_client("mcp-deaf-told");

    let started = Instant::now();
    let timed_out = late.call(
        "works_read",
        &arguments,
        Instant::now() + Duration::from_millis(400),
        &Cancel::new(),
    );
    let timed_out_after = started.elapsed();
    let cancel = Cancel::new();
    let stopper = {
        let cancel = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            cancel.cancel();
        })
    };
    let started = Instant::now();
    let cancelled = told.call("works_read", &arguments, within(60), &cancel);
    let cancelled_after = started.elapsed();
    stopper.join().unwrap();

    assert!(
        matches!(timed_out, Err(McpError::TimedOut)),
        "{timed_out:?}"
    );
    assert!(timed_out_after < PROMPTLY, "{timed_out_after:?}");
    assert!(
        matches!(cancelled, Err(McpError::Cancelled)),
        "{cancelled:?}"
    );
    assert!(cancelled_after < PROMPTLY, "{cancelled_after:?}");
}
