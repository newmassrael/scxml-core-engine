// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The routers a replay gives the child sessions of the machine it drives
//! (SCE_MESH.md, Child Sessions in Mesh; docs/adr/0043).
//!
//! A child that sends to a Mesh peer is given a router of its own by its
//! parent, which asks the factory its host registered; a replay is the host, so
//! it registers one ([`Children::factory`]) whose routers write down what each
//! child sent, told by the path of invoke ids that started it and how many
//! children that path had started, and hold what a step's `invoke` delivers to
//! the child until its parent applies it. A router answers for the child's own
//! children in turn, so a grandchild is told by `<invoke>/<invoke>`.

use sce_rust_runtime::{
    ChildMeshRouter, EventMetadata, EventType, HostInvokeEvent, HostInvokeResponse,
    HostSendRequest, HostSendResponse, MeshCall,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// What a child's router holds for it until its parent applies it.
type Inbox = Arc<Mutex<Vec<MeshCall>>>;

const HELD: &str = "a replay's record of its children is never left mid-update";

/// The starts of each path, and the router of the session each path is running.
#[derive(Default)]
struct Book {
    starts: BTreeMap<String, u64>,
    running: BTreeMap<String, Inbox>,
}

/// The children a replay gave routers to, and the list every send is written
/// into, the machine's own and its children's, oldest first.
#[derive(Clone)]
pub struct Children {
    sent: Arc<Mutex<Vec<Value>>>,
    book: Arc<Mutex<Book>>,
}

impl Children {
    /// Children whose sends are written into `sent`, beside the machine's own.
    pub fn new(sent: Arc<Mutex<Vec<Value>>>) -> Self {
        Self {
            sent,
            book: Arc::default(),
        }
    }

    /// The factory a replay registers on the machine: every child is given a
    /// router, whichever invoke starts it and from whatever document.
    pub fn factory(
        &self,
    ) -> impl FnMut(&str, &str) -> Option<Box<dyn ChildMeshRouter>> + Send + 'static {
        let children = self.clone();
        move |invoke_id: &str, _document: &str| Some(children.start(invoke_id.to_string()))
    }

    /// A router for a new start of `path`, which is now the session the path
    /// is running.
    fn start(&self, path: String) -> Box<dyn ChildMeshRouter> {
        let inbox = Inbox::default();
        let start = {
            let mut book = self.book.lock().expect(HELD);
            let starts = book.starts.entry(path.clone()).or_insert(0);
            *starts += 1;
            let start = *starts;
            book.running.insert(path.clone(), Arc::clone(&inbox));
            start
        };
        Box::new(Recording {
            path,
            start,
            inbox,
            children: self.clone(),
        })
    }

    /// Hand `event`, with `data` as its `_event.data`, to the running child of
    /// `path`, as its parent would be handed it by a step: the parent applies it
    /// the next time it drives the child. `false` when no child of `path` is
    /// running, which a step that names one fails on.
    pub fn deliver(&self, path: &str, event: &str, data: &str) -> bool {
        let book = self.book.lock().expect(HELD);
        let Some(inbox) = book.running.get(path) else {
            return false;
        };
        inbox.lock().expect(HELD).push(MeshCall::Raise {
            name: event.to_string(),
            metadata: EventMetadata {
                data: data.to_string(),
                event_type: EventType::External,
                ..EventMetadata::default()
            },
        });
        true
    }
}

/// The router of one start of one child.
struct Recording {
    path: String,
    start: u64,
    inbox: Inbox,
    children: Children,
}

impl ChildMeshRouter for Recording {
    /// Written down as the machine's own sends are, with the child's path and
    /// start; the peer is not there, so nothing answers.
    fn send(&mut self, request: HostSendRequest) -> Vec<HostSendResponse> {
        let mut send = serde_json::Map::new();
        send.insert("target".into(), Value::String(request.target));
        send.insert("event".into(), Value::String(request.event_name));
        if !request.event_data.is_empty() {
            send.insert(
                "data".into(),
                serde_json::from_str(&request.event_data).expect("the data of a send is JSON"),
            );
        }
        send.insert("invoke".into(), Value::String(self.path.clone()));
        send.insert("start".into(), Value::from(self.start));
        self.children
            .sent
            .lock()
            .expect(HELD)
            .push(Value::Object(send));
        Vec::new()
    }

    /// A request the child makes is started and never answered: no peer is
    /// there to answer it.
    fn invoke(&mut self, _event: HostInvokeEvent) -> Option<HostInvokeResponse> {
        None
    }

    fn take_calls(&mut self) -> Vec<MeshCall> {
        std::mem::take(&mut *self.inbox.lock().expect(HELD))
    }

    /// The path is no longer running this session: a step that names it fails
    /// until the path starts a child again.
    fn detach(&mut self) {
        let mut book = self.children.book.lock().expect(HELD);
        if book
            .running
            .get(&self.path)
            .is_some_and(|running| Arc::ptr_eq(running, &self.inbox))
        {
            book.running.remove(&self.path);
        }
    }

    fn child_router(
        &mut self,
        invoke_id: &str,
        _document: &str,
    ) -> Option<Box<dyn ChildMeshRouter>> {
        Some(self.children.start(format!("{}/{invoke_id}", self.path)))
    }
}
