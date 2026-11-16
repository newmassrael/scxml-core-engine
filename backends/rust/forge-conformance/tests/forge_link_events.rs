// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A link that carries the events it declares (docs/adr/0106, #142), compiled
//! and run.
//!
//! `sce-build`'s `link_event_paths` reads the text the generator writes for a
//! link's `pump` and `send`; nothing there shows that the framer's `decode`, a
//! codec's `encode` and the transmit pool's slot functions it calls are the ones
//! those documents generate. Here the link, its codecs and its pool are generated
//! by `build.rs` Step 9 from `tests/forge/conformance/link_events/`, included as
//! sibling modules, and driven over a driver whose wire loops back: what `send`
//! hands the driver is what the next `pump` reads.

mod link_modules {
    // The generated modules open with `#![allow(...)]`, which `build.rs` strips
    // because an inner attribute cannot sit at an `include!` point; each module
    // carries them as outer attributes. `dead_code` is added: a generated
    // module's items are its public contract, unused only because this test
    // includes it into a private module.
    #[allow(clippy::all, unused_imports, dead_code)]
    pub mod link_events_frame {
        include!(concat!(env!("OUT_DIR"), "/link_events_frame.rs"));
    }
    #[allow(clippy::all, unused_imports, dead_code)]
    pub mod link_events_query {
        include!(concat!(env!("OUT_DIR"), "/link_events_query.rs"));
    }
    #[allow(clippy::all, unused_imports, dead_code)]
    pub mod link_events_tx_pool {
        include!(concat!(env!("OUT_DIR"), "/link_events_tx_pool.rs"));
    }
    #[allow(clippy::all, unused_imports, unused_variables, dead_code)]
    pub mod link_events_pooled {
        include!(concat!(env!("OUT_DIR"), "/link_events_pooled.rs"));
    }
    #[allow(clippy::all, unused_imports, unused_variables, dead_code)]
    pub mod link_events_direct {
        include!(concat!(env!("OUT_DIR"), "/link_events_direct.rs"));
    }
}

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use link_modules::link_events_direct::{
    LinkEventsDirect, LinkEventsDirectInbound, LinkEventsDirectOutbound,
};
use link_modules::link_events_frame::LinkEventsFrame;
use link_modules::link_events_pooled::{
    LinkEventsPooled, LinkEventsPooledInbound, LinkEventsPooledOutbound,
};
use link_modules::link_events_query::LinkEventsQuery;
use link_modules::link_events_tx_pool::{LinkEventsTxPool, SLOT_COUNT};
use sce_link_runtime::{Link, LinkError, RxFrame, TxFrame};

/// The wire a [`Loopback`] driver stands on: the frames waiting to be read,
/// every frame sent, and the deadline of the last tick.
#[derive(Debug, Default)]
struct Wire {
    pending: VecDeque<Vec<u8>>,
    sent: Vec<Vec<u8>>,
    last_deadline: Option<u32>,
}

/// A driver whose wire loops back: a frame `tx` takes is read by a later `rx`.
struct Loopback {
    wire: Rc<RefCell<Wire>>,
    current: Vec<u8>,
}

impl Loopback {
    fn on(wire: &Rc<RefCell<Wire>>) -> Self {
        Self {
            wire: Rc::clone(wire),
            current: Vec::new(),
        }
    }
}

impl Link for Loopback {
    fn rx(&mut self) -> Option<RxFrame<'_>> {
        self.current = self.wire.borrow_mut().pending.pop_front()?;
        Some(RxFrame::new(&self.current))
    }

    fn tx(&mut self, frame: TxFrame<'_>) -> Result<(), LinkError> {
        let mut wire = self.wire.borrow_mut();
        wire.sent.push(frame.data.to_vec());
        wire.pending.push_back(frame.data.to_vec());
        Ok(())
    }

    fn poll(&mut self, deadline_us: u32) {
        self.wire.borrow_mut().last_deadline = Some(deadline_us);
    }
}

/// Each event a pump injected, with the frame's `msg_id` and `payload_len`.
type Injected = Vec<(LinkEventsPooledInbound, u8, u16)>;

/// What a pump of the pooled link answered, what it injected, and how many
/// frames the framer refused.
fn pump_pooled(link: &mut LinkEventsPooled<Loopback>) -> (usize, Injected, usize) {
    let mut injected = Vec::new();
    let mut refused = 0;
    let count = link.pump(
        250,
        |event, frame: &LinkEventsFrame| injected.push((event, frame.msg_id, frame.payload_len)),
        |_| refused += 1,
    );
    (count, injected, refused)
}

#[test]
fn a_pump_injects_every_row_that_holds_and_hands_on_what_the_framer_refuses() {
    let wire = Rc::new(RefCell::new(Wire::default()));
    {
        let mut wire = wire.borrow_mut();
        // A hello (msg_id 2, length 5), another message (msg_id 7), and two
        // bytes, which the three-byte framer refuses.
        wire.pending.push_back(vec![0x02, 0x05, 0x00]);
        wire.pending.push_back(vec![0x07, 0x00, 0x01]);
        wire.pending.push_back(vec![0x02, 0x05]);
    }
    let mut link = LinkEventsPooled::new(Loopback::on(&wire));

    let (count, injected, refused) = pump_pooled(&mut link);

    assert_eq!(
        wire.borrow().last_deadline,
        Some(250),
        "the pump gives the driver its tick"
    );
    assert_eq!(
        injected,
        [
            (LinkEventsPooledInbound::ScoutHelloReceived, 2, 5),
            (LinkEventsPooledInbound::ScoutAnyReceived, 2, 5),
            (LinkEventsPooledInbound::ScoutAnyReceived, 7, 0x0100),
        ],
        "each row is a rule of its own, in document order: the guarded row \
         raises its event for the hello only, the unguarded one for every frame"
    );
    assert_eq!(count, 3);
    assert_eq!(refused, 1, "a frame the framer refuses raises no event");
    assert_eq!(
        LinkEventsPooledInbound::ScoutHelloReceived.name(),
        "scout.hello.received"
    );
}

#[test]
fn a_send_through_the_pool_is_the_frame_the_next_pump_reads() {
    let wire = Rc::new(RefCell::new(Wire::default()));
    let mut link = LinkEventsPooled::new(Loopback::on(&wire));
    let mut pool = LinkEventsTxPool::new();

    let hello = LinkEventsPooledOutbound::ScoutFrameSend(LinkEventsFrame {
        msg_id: 2,
        payload_len: 0x0102,
    });
    assert_eq!(hello.name(), "scout.frame.send");
    link.send(&mut pool, &hello)
        .expect("the framer encodes the hello");
    let query = LinkEventsPooledOutbound::ScoutQuerySend(LinkEventsQuery {
        query_id: 9,
        window: 4,
    });
    link.send(&mut pool, &query)
        .expect("the query codec encodes the query");

    assert_eq!(
        wire.borrow().sent,
        [vec![0x02, 0x02, 0x01], vec![0x09, 0x04]],
        "each event is encoded by the codec its row names, little-endian"
    );
    assert_eq!(
        pool.free_count(),
        SLOT_COUNT,
        "a slot is returned once the driver has copied its bytes"
    );

    let (count, injected, refused) = pump_pooled(&mut link);
    assert_eq!(
        injected,
        [
            (LinkEventsPooledInbound::ScoutHelloReceived, 2, 0x0102),
            (LinkEventsPooledInbound::ScoutAnyReceived, 2, 0x0102),
        ],
        "the hello sent is the hello read back"
    );
    assert_eq!(count, 2);
    assert_eq!(refused, 1, "the two-byte query is no frame of the framer");
}

#[test]
fn a_send_without_a_pool_encodes_into_a_buffer_of_the_largest_encoding() {
    let wire = Rc::new(RefCell::new(Wire::default()));
    let mut link = LinkEventsDirect::new(Loopback::on(&wire));

    link.send(&LinkEventsDirectOutbound::ScoutFrameSend(LinkEventsFrame {
        msg_id: 2,
        payload_len: 7,
    }))
    .expect("the framer's three bytes fit the buffer of the largest encoding");
    link.send(&LinkEventsDirectOutbound::ScoutQuerySend(LinkEventsQuery {
        query_id: 1,
        window: 2,
    }))
    .expect("the query's two bytes fit too");
    assert_eq!(
        wire.borrow().sent,
        [vec![0x02, 0x07, 0x00], vec![0x01, 0x02]]
    );

    let mut injected = Vec::new();
    let count = link.pump(
        0,
        |event, frame: &LinkEventsFrame| injected.push((event, frame.payload_len)),
        |_| {},
    );
    assert_eq!(count, 1);
    assert_eq!(injected, [(LinkEventsDirectInbound::ScoutHelloReceived, 7)]);
}
