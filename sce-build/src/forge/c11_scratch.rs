// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The storage of a C11 machine's scratch roles, shared by liveness
//! (docs/adr/0086, section 8.5 (e)).
//!
//! Every buffer of the size of a machine's slot of `_event.data` that the
//! machine writes data into before it hands the data on is a role of its
//! scratch (section 8.5 (d)). Two roles may share storage when they are never
//! live at once, and the storage is allocated as a compiler allocates
//! registers: an interference graph over the roles, coloured, one colour per
//! storage location (Chaitin, "Register allocation and spilling via graph
//! coloring", 1982). Each colour is one anonymous `union` of the machine's
//! scratch, which holds every role of that colour under the role's own name,
//! so the generated code reads and writes a role by its name.
//!
//! **What counts as live.** A role is live in an *activity*: the stretch of
//! the generated code from the statement that writes the role to its last read
//! (a `<send>` from the evaluation of its payload to the send, a host-run
//! invocation's start from its first pair to the host's answer, a lift from
//! its first field to the error it raises). An activity holds every role it
//! writes, all of them live through the whole of it, which over-approximates
//! each role's own range.
//!
//! **How reentrancy is modelled.** An activity *contains* another when the
//! other can begin and end while the first is in progress: a call the first
//! makes, or a function a host may call from a handler the first calls.
//! [`NESTS`] lists each such pair; containment is its transitive closure. A
//! host handler is one activity, which contains every machine function a host
//! may call from inside a handler (a typed raise, a typed completion, a raise
//! by name, a completion by token); the machine's own drive (`_step`, `_tick`,
//! a restore) is not one of them, since a machine is driven by its owner at a
//! macrostep boundary and not from inside one of its own calls (docs/adr/0084).
//! The drain of the inbox that `_raise_external` makes before it enqueues is
//! an activity of its own, which writes no role: it applies posted inputs
//! from the machine's own storage (`inbox_taken`), and each raise it makes is
//! a view the queue copies.
//!
//! **Interference.** Two roles interfere when some activity of one is an
//! activity of the other, or contains it, or is contained by it: then some
//! program point has both live. Two activities neither of which contains the
//! other run one after the other, so roles held only by them do not interfere.
//!
//! **Why the allocation is sound.** The colouring gives two interfering roles
//! different colours ([`colours`] checks every earlier neighbour of a role
//! before it picks), and two roles of one colour therefore never have a
//! program point where both are live. That rests on [`NESTS`]
//! over-approximating what can run during an activity; an edge in doubt is
//! listed. The colouring is greedy in [`ROLES`]' order, so it is
//! deterministic, and it uses at most one colour more than the largest number
//! of neighbours any role has.

/// A stretch of a C11 machine's generated code in which the roles it writes
/// are live ([`Role::activities`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Activity {
    /// A `<send>`, from the evaluation of its payload to its delivery, a
    /// host-served send's call of its handler included.
    Send,
    /// The start of a host-run `<invoke>`, from its first pair to the host's
    /// answer.
    InvokeStart,
    /// The start again of the host-run invocations a restore re-armed.
    Restart,
    /// A host's handler, called by the machine, and every machine function a
    /// host may call from inside it.
    HostCallback,
    /// The drain of the inbox `_raise_external` makes before it enqueues.
    Drain,
    /// A typed raise seam (`_raise_<event>_typed`).
    Seam,
    /// A typed host-run result's completion (`_complete_<invoke>`).
    Result,
    /// A host-run invocation's failure a restore found
    /// (`raise_host_invoke_failure`).
    Failure,
    /// The delivery of a BasicHTTP reply.
    Reply,
    /// A host-served send's handler call and the replies it reported, until
    /// each is raised.
    HostReply,
    /// A host-run invocation's start call and the answer it gave, until the
    /// completion or the refusal is raised.
    HostAnswer,
    /// A child session's done data raised as `done.invoke`.
    ChildDone,
    /// A script engine's done data raised as `done.state`.
    DoneState,
    /// The lift of an event's typed payload at its dequeue.
    Lift,
    /// A reason the machine writes and raises at once as an internal error.
    Reason,
    /// A restore's reading of the queued events.
    Restore,
}

impl Activity {
    /// Every activity.
    pub const ALL: [Activity; 16] = [
        Activity::Send,
        Activity::InvokeStart,
        Activity::Restart,
        Activity::HostCallback,
        Activity::Drain,
        Activity::Seam,
        Activity::Result,
        Activity::Failure,
        Activity::Reply,
        Activity::HostReply,
        Activity::HostAnswer,
        Activity::ChildDone,
        Activity::DoneState,
        Activity::Lift,
        Activity::Reason,
        Activity::Restore,
    ];

    /// The name the generated C marks where the activity begins, in a comment
    /// `/* sce-scratch-activity: <name> */`: the activity runs from there to
    /// the end of the block that holds the comment. A host callback has no
    /// mark: it is a call through a function pointer of the machine.
    pub fn marker(self) -> &'static str {
        match self {
            Activity::Send => "send",
            Activity::InvokeStart => "invoke_start",
            Activity::Restart => "restart",
            Activity::HostCallback => "host_callback",
            Activity::Drain => "drain",
            Activity::Seam => "seam",
            Activity::Result => "result",
            Activity::Failure => "failure",
            Activity::Reply => "reply",
            Activity::HostReply => "host_reply",
            Activity::HostAnswer => "host_answer",
            Activity::ChildDone => "child_done",
            Activity::DoneState => "done_state",
            Activity::Lift => "lift",
            Activity::Reason => "reason",
            Activity::Restore => "restore",
        }
    }

    /// The activity the generated C marks by `marker` ([`Self::marker`]).
    pub fn marked(marker: &str) -> Option<Activity> {
        Self::ALL.iter().copied().find(|a| a.marker() == marker)
    }
}

/// `(outer, inner)`: `inner` can begin and end while `outer` is in progress.
/// An over-approximation: a pair in doubt is listed.
pub const NESTS: &[(Activity, Activity)] = &[
    // A send raises errors from literals, calls a host's handler, raises its
    // event (which drains the inbox), makes a BasicHTTP request whose reply it
    // delivers, and hands a child an event; the child's done data is listed
    // too, though the child only queues what it is handed.
    (Activity::Send, Activity::HostCallback),
    (Activity::Send, Activity::HostReply),
    (Activity::Send, Activity::Drain),
    // A host-served send's handler, and the raise of each reply it reported.
    (Activity::HostReply, Activity::HostCallback),
    (Activity::HostReply, Activity::Drain),
    (Activity::Send, Activity::Reply),
    (Activity::Send, Activity::ChildDone),
    // A start raises a reason it formats, calls a host's handler, and raises
    // a completion the host gave at once.
    (Activity::InvokeStart, Activity::Reason),
    (Activity::InvokeStart, Activity::HostCallback),
    (Activity::InvokeStart, Activity::HostAnswer),
    (Activity::InvokeStart, Activity::Drain),
    (Activity::Restart, Activity::Reason),
    (Activity::Restart, Activity::HostCallback),
    (Activity::Restart, Activity::HostAnswer),
    (Activity::Restart, Activity::Drain),
    (Activity::Restart, Activity::Failure),
    // A start's handler, and the completion or refusal it answered.
    (Activity::HostAnswer, Activity::HostCallback),
    (Activity::HostAnswer, Activity::Drain),
    // What a host may call from inside a handler.
    (Activity::HostCallback, Activity::Seam),
    (Activity::HostCallback, Activity::Result),
    (Activity::HostCallback, Activity::Drain),
    // Every raise onto the external queue drains the inbox first.
    (Activity::Seam, Activity::Drain),
    (Activity::Result, Activity::Drain),
    (Activity::Failure, Activity::Drain),
    (Activity::Reply, Activity::Drain),
    (Activity::ChildDone, Activity::Drain),
    // A lift raises the reason it wrote.
    (Activity::Lift, Activity::Reason),
];

/// How many slots a role holds: one, one per pair of the widest element that
/// hands a host its pairs, or one per reply a host-served send may report
/// (`SCE_MAX_HOST_RESPONSES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rows {
    One,
    Pairs,
    Responses,
}

impl Rows {
    /// The name the machine's header spells the rows by: none for one slot.
    pub fn spelling(self) -> &'static str {
        match self {
            Rows::One => "",
            Rows::Pairs => "pairs",
            Rows::Responses => "responses",
        }
    }
}

/// A role of a C11 machine's scratch: its member's name, how many slots it
/// holds, what it holds, and the activities it is live in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Role {
    pub name: &'static str,
    pub rows: Rows,
    pub about: &'static str,
    pub activities: &'static [Activity],
}

/// Every role, in the order the colouring takes them.
pub const ROLES: &[Role] = &[
    Role {
        name: "payload",
        rows: Rows::One,
        about: "A `<send>`'s payload, as JSON, from its evaluation until it is sent; and the data \
                a restore reads into a queued event.",
        activities: &[Activity::Send, Activity::Restore],
    },
    Role {
        name: "content",
        rows: Rows::One,
        about: "A `<send>`'s `<content expr>` as the text a host's request carries, or the body \
                of a BasicHTTP request.",
        activities: &[Activity::Send],
    },
    Role {
        name: "pairs",
        rows: Rows::Pairs,
        about: "The text of each pair a host-served `<send>` or a host-run `<invoke>` hands \
                over, one row per pair, the namelist's first.",
        activities: &[Activity::Send, Activity::InvokeStart],
    },
    Role {
        name: "request",
        rows: Rows::One,
        about: "A host-run invocation's request event data.",
        activities: &[Activity::InvokeStart],
    },
    Role {
        name: "invoke_content",
        rows: Rows::One,
        about: "A host-run invocation's `<content expr>` under a script engine.",
        activities: &[Activity::InvokeStart],
    },
    Role {
        name: "message",
        rows: Rows::One,
        about: "A reason the machine raises as the data of an internal error event.",
        activities: &[Activity::Lift, Activity::Reason],
    },
    Role {
        name: "failure",
        rows: Rows::One,
        about: "The reason a host-run invocation failed, raised as its `error.invoke`'s data.",
        activities: &[Activity::Failure],
    },
    Role {
        name: "seam",
        rows: Rows::One,
        about: "The data a typed raise seam writes a host's payload as.",
        activities: &[Activity::Seam],
    },
    Role {
        name: "result",
        rows: Rows::One,
        about: "The data a typed host-run result completes its invocation with.",
        activities: &[Activity::Result],
    },
    Role {
        name: "done",
        rows: Rows::One,
        about: "The done data a child session or the script engine hands a done event.",
        activities: &[Activity::ChildDone, Activity::DoneState],
    },
    Role {
        name: "reply",
        rows: Rows::One,
        about: "The data of a reply a BasicHTTP response carries.",
        activities: &[Activity::Reply],
    },
    Role {
        name: "answer",
        rows: Rows::One,
        about: "The completion or the refusal a host answered a host-run invocation's start \
                with.",
        activities: &[Activity::HostAnswer],
    },
    Role {
        name: "replies",
        rows: Rows::Responses,
        about: "The data of each reply a host-served send's handler reported, one row per \
                reply; held by a machine that serves such a send.",
        activities: &[Activity::HostReply],
    },
];

/// Whether a machine holds the role `role`: every role but `replies`, which a
/// machine holds when it serves a host-served send, `host_sends`, whose header
/// then names the bound of its rows.
pub fn held(role: &Role, host_sends: bool) -> bool {
    role.rows != Rows::Responses || host_sends
}

/// Whether `inner` can run while `outer` is in progress: `outer` contains it,
/// directly or through activities it contains ([`NESTS`]).
pub fn contains(outer: Activity, inner: Activity) -> bool {
    let mut seen = vec![outer];
    let mut pending = vec![outer];
    while let Some(at) = pending.pop() {
        for &(from, to) in NESTS {
            if from == at && !seen.contains(&to) {
                if to == inner {
                    return true;
                }
                seen.push(to);
                pending.push(to);
            }
        }
    }
    false
}

/// Whether `a` and `b` are live at one program point: some activity of one is
/// an activity of the other, contains it or is contained by it.
pub fn interfere(a: &Role, b: &Role) -> bool {
    a.activities.iter().any(|&x| {
        b.activities
            .iter()
            .any(|&y| x == y || contains(x, y) || contains(y, x))
    })
}

/// The colour of each role of `roles`, by index: each role, in order, takes
/// the smallest colour no earlier role it interferes with holds.
pub fn colours_of(roles: &[Role]) -> Vec<usize> {
    let mut colours: Vec<usize> = Vec::with_capacity(roles.len());
    for (at, role) in roles.iter().enumerate() {
        let taken: Vec<usize> = roles[..at]
            .iter()
            .zip(&colours)
            .filter(|(earlier, _)| interfere(role, earlier))
            .map(|(_, &colour)| colour)
            .collect();
        let colour = (0..).find(|c| !taken.contains(c)).unwrap_or(0);
        colours.push(colour);
    }
    colours
}

/// The colour of each role of [`ROLES`], by index.
pub fn colours() -> Vec<usize> {
    colours_of(ROLES)
}

/// A role as the machine's header declares it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScratchRole {
    pub name: &'static str,
    /// Its rows ([`Rows::spelling`]).
    pub rows: &'static str,
    pub about: &'static str,
}

/// One colour: the roles that share one storage location, in [`ROLES`]' order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScratchColour {
    pub roles: Vec<ScratchRole>,
}

/// The scratch of a machine that holds the roles [`held`] answers for
/// `host_sends`, as its header declares it: one union per colour of those
/// roles, in the order of the colours.
pub fn layout(host_sends: bool) -> Vec<ScratchColour> {
    let roles: Vec<Role> = ROLES
        .iter()
        .copied()
        .filter(|role| held(role, host_sends))
        .collect();
    let colours = colours_of(&roles);
    let count = colours.iter().copied().max().map_or(0, |top| top + 1);
    let mut layout: Vec<ScratchColour> = (0..count)
        .map(|_| ScratchColour { roles: Vec::new() })
        .collect();
    for (role, colour) in roles.iter().zip(colours) {
        layout[colour].roles.push(ScratchRole {
            name: role.name,
            rows: role.rows.spelling(),
            about: role.about,
        });
    }
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every two roles that interfere hold different storage: the soundness of
    /// the allocation, asked of every pair.
    #[test]
    fn every_two_interfering_roles_hold_different_storage() {
        let colours = colours();
        for (a, role_a) in ROLES.iter().enumerate() {
            for (b, role_b) in ROLES.iter().enumerate().skip(a + 1) {
                if interfere(role_a, role_b) {
                    assert_ne!(
                        colours[a], colours[b],
                        "`{}` and `{}` are live at once and share storage",
                        role_a.name, role_b.name
                    );
                }
            }
        }
    }

    /// Each activity's mark names it alone, so the generated code's marks read
    /// back as the activities they were written for.
    #[test]
    fn every_activity_reads_back_from_its_mark() {
        for activity in Activity::ALL {
            assert_eq!(Activity::marked(activity.marker()), Some(activity));
        }
        assert_eq!(Activity::marked("no_such_activity"), None);
    }

    /// The roles of one `<send>` are live at once, as are a start's; a typed
    /// raise a host makes from a handler is live while either is.
    #[test]
    fn the_roles_of_one_activity_and_of_a_callback_inside_it_interfere() {
        let role = |name: &str| ROLES.iter().find(|r| r.name == name).expect("a role");
        assert!(interfere(role("payload"), role("content")));
        assert!(interfere(role("pairs"), role("request")));
        assert!(interfere(role("payload"), role("seam")));
        assert!(interfere(role("request"), role("message")));
    }

    /// The drain of the inbox writes no role, so a failure's reason and an
    /// internal reason, a seam's data and a result's, are never live at once
    /// and share storage.
    #[test]
    fn roles_of_activities_run_one_after_the_other_share_storage() {
        let role = |name: &str| ROLES.iter().find(|r| r.name == name).expect("a role");
        assert!(!interfere(role("failure"), role("message")));
        assert!(!interfere(role("seam"), role("result")));
        let colours = colours();
        let at = |name: &str| ROLES.iter().position(|r| r.name == name).expect("a role");
        assert_eq!(colours[at("seam")], colours[at("result")]);
    }

    /// A colouring that merges two interfering roles is told as unsound by the
    /// check every allocation is held to.
    #[test]
    fn a_colouring_that_merges_two_interfering_roles_is_unsound() {
        let mut colours = colours();
        let at = |name: &str| ROLES.iter().position(|r| r.name == name).expect("a role");
        colours[at("content")] = colours[at("payload")];
        let unsound = ROLES.iter().enumerate().any(|(a, role_a)| {
            ROLES
                .iter()
                .enumerate()
                .skip(a + 1)
                .any(|(b, role_b)| interfere(role_a, role_b) && colours[a] == colours[b])
        });
        assert!(unsound);
    }

    /// The layout holds every role a machine holds once, in fewer colours than
    /// roles; one that serves no host-served send holds no replies.
    #[test]
    fn the_layout_holds_every_role_once_in_fewer_colours_than_roles() {
        for host_sends in [true, false] {
            let layout = layout(host_sends);
            let names: Vec<&str> = layout
                .iter()
                .flat_map(|colour| colour.roles.iter().map(|role| role.name))
                .collect();
            let expected = ROLES.iter().filter(|role| held(role, host_sends)).count();
            assert_eq!(names.len(), expected);
            assert!(layout.len() < expected);
            assert!(layout.iter().all(|colour| !colour.roles.is_empty()));
            assert_eq!(names.contains(&"replies"), host_sends);
        }
    }
}
