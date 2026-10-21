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
    /// The factory is being asked for the routers of sessions a restore
    /// resumed ([`Children::resuming`]): a path that is running is the same
    /// session, given a router again, and not a new start of it.
    resuming: bool,
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

    /// Run `register`, the registration of this factory on a machine a restore
    /// handed the replay, as the asks for the routers of the sessions that
    /// restore resumed (docs/adr/0045): each is the session its path was
    /// running when the machine was saved, so its sends keep the start they
    /// were told by, and what a step delivers to the path reaches it.
    pub fn resuming<R>(&self, register: impl FnOnce() -> R) -> R {
        self.book.lock().expect(HELD).resuming = true;
        let registered = register();
        self.book.lock().expect(HELD).resuming = false;
        registered
    }

    /// A router for `path`: a new start of it, or, while
    /// [`resuming`](Self::resuming), the session it is running given a router
    /// again. Either way it is now the session the path is running.
    fn start(&self, path: String) -> Box<dyn ChildMeshRouter> {
        let inbox = Inbox::default();
        let start = {
            let mut book = self.book.lock().expect(HELD);
            let resumed = book.resuming && book.running.contains_key(&path);
            let starts = book.starts.entry(path.clone()).or_insert(0);
            if !resumed {
                *starts += 1;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    type Sent = Arc<Mutex<Vec<Value>>>;

    fn greet(router: &mut dyn ChildMeshRouter) {
        router.send(HostSendRequest {
            target: "#server".to_string(),
            event_name: "list.hello".to_string(),
            ..HostSendRequest::default()
        });
    }

    fn starts(sent: &Sent) -> Vec<u64> {
        sent.lock()
            .expect(HELD)
            .iter()
            .map(|send| send["start"].as_u64().expect("a start"))
            .collect()
    }

    #[test]
    fn a_resumed_child_keeps_its_start_and_a_later_start_counts_on() {
        let sent: Sent = Arc::default();
        let children = Children::new(Arc::clone(&sent));
        let mut factory = children.factory();
        let mut first = factory("list", "").expect("a router");
        greet(first.as_mut());

        // A restore registers the factory again, and it is asked for the child
        // that was running: the same session, told by the same start.
        let mut registered = children.factory();
        let mut resumed = children
            .resuming(|| registered("list", ""))
            .expect("a router");
        greet(resumed.as_mut());
        assert!(
            children.deliver("list", "list.ping", ""),
            "the resumed child is the one the path runs"
        );
        assert_eq!(
            resumed.take_calls().len(),
            1,
            "a step's event reaches the resumed child's router"
        );

        // A start after the restore is the path's second.
        let mut again = registered("list", "").expect("a router");
        greet(again.as_mut());
        assert_eq!(starts(&sent), [1, 1, 2]);
    }
}
