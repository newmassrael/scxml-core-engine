// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a test that puts a generated Kotlin copy into the world of the to-do list has in
// common: finding the JVM and the step server, starting one process per Kotlin actor, and
// building worlds in which some of the three actors are those processes.
//
// The step server is the module `:sce-kotlin-world` (backends/kotlin/world), built by
// `./gradlew :sce-kotlin-world:installDist`. A test that needs it follows the
// convention every test that needs a toolchain follows: with the tool or the build
// missing it says so and passes, unless `SCE_REQUIRE_TOOLS` is set, which is a lane's claim
// that the check ran, and then it fails.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sce_build::toolchain;
use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_world::external::{ExternalCommand, ExternalMachine};
use sce_rust_world::{Fate, World};

use super::{todo_world_of, TodoPeer, TodoPeerPersist};

/// Where the step server's distribution is built, from the root of the repository.
const DISTRIBUTION: &str = "backends/kotlin/world/build/install/sce-kotlin-world";

/// The machine the world hosts, by the name the step server knows it under.
const MACHINE: &str = "todo_peer";

/// The command that starts one step server, or `None` after saying why this run cannot
/// have one.
///
/// # Panics
///
/// When `SCE_REQUIRE_TOOLS` is set and the JVM or the build is missing.
pub fn step_server() -> Option<ExternalCommand> {
    let java = toolchain::require_or_skip("java", "host a generated Kotlin copy in a JVM")?;
    let lib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(DISTRIBUTION)
        .join("lib");
    let built = std::fs::read_dir(&lib).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("sce-kotlin-world-")
        })
    });
    if !built {
        toolchain::skipped(&format!(
            "the Kotlin step server is not built at {} (run `./gradlew :sce-kotlin-world:installDist`)",
            lib.display()
        ));
        return None;
    }
    Some(
        ExternalCommand::new(java)
            .args([
                // A process that lives for one test does not earn back the optimising
                // compiler's warm-up, and a machine this small needs one collector thread.
                "-XX:TieredStopAtLevel=1",
                "-XX:+UseSerialGC",
                "-Xmx256m",
                "-Dfile.encoding=UTF-8",
            ])
            .arg("-cp")
            .arg(lib.join("*"))
            .arg("com.sce.world.MainKt")
            .args(["--machine", MACHINE]),
    )
}

/// The actors of the to-do list.
pub const ACTORS: [&str; 3] = ["alice", "bob", "server"];

/// Every way of making some of the three actors Kotlin machines and the rest Rust, but
/// none and not all of them being Rust: seven, the permutations a test that is about
/// copies written in different languages agreeing runs.
pub const MIXES: [&[&str]; 7] = [
    &["alice"],
    &["bob"],
    &["server"],
    &["alice", "bob"],
    &["alice", "server"],
    &["bob", "server"],
    &["alice", "bob", "server"],
];

/// The processes that run Kotlin machines, one per actor that a test makes Kotlin in some
/// world. They live as long as the cast, so a test that builds many worlds from one cast
/// starts each JVM once, and the machine in each is started afresh by each world.
pub struct Cast {
    processes: BTreeMap<&'static str, ExternalMachine>,
}

impl Cast {
    /// A process for each of `actors`, or `None` when this run cannot host Kotlin (see
    /// [`step_server`]).
    ///
    /// # Panics
    ///
    /// When a process cannot be started or does not host the to-do list.
    pub fn of(actors: &[&'static str]) -> Option<Self> {
        let command = step_server()?;
        let mut processes = BTreeMap::new();
        for who in actors {
            assert!(
                ACTORS.contains(who),
                "{who} is not an actor of the to-do list"
            );
            let machine = ExternalMachine::spawn(&command)
                .unwrap_or_else(|failure| panic!("the Kotlin step server for {who}: {failure}"));
            assert_eq!(
                machine.machine(),
                MACHINE,
                "the step server hosts another machine"
            );
            processes.insert(*who, machine);
        }
        Some(Self { processes })
    }

    /// The worlds in which `kotlin` are Kotlin machines and the other actors Rust ones.
    ///
    /// # Panics
    ///
    /// When the cast holds no process for one of `kotlin`.
    pub fn mix(&self, kotlin: &[&'static str]) -> Mix<'_> {
        for who in kotlin {
            assert!(
                self.processes.contains_key(who),
                "the cast holds no process for {who}"
            );
        }
        Mix {
            cast: self,
            kotlin: kotlin.to_vec(),
        }
    }
}

/// One way of dividing the actors between Kotlin and Rust.
pub struct Mix<'a> {
    cast: &'a Cast,
    kotlin: Vec<&'static str>,
}

impl Mix<'_> {
    /// The world of the to-do list in this mix: `alice`, `bob` and the hub `server`, each
    /// `latency_ms` from the hub. Two worlds of one cast must not run at once, as they
    /// would share the processes.
    pub fn world(&self, seed: u64, latency_ms: i64) -> World {
        todo_world_of(seed, latency_ms, |world, who, router| {
            if self.is_kotlin(who) {
                world.add_external(who, &self.cast.processes[who], router);
            } else {
                world.add(who, TodoPeer::new(), router);
            }
        })
    }

    pub fn is_kotlin(&self, who: &str) -> bool {
        self.kotlin.contains(&who)
    }

    /// A copy killed and started again from what it saved of itself, whatever it is made
    /// of: a Kotlin copy is restored by its own `restore`, a Rust copy by its.
    pub fn restart(&self, world: &mut World, who: &str) {
        if self.is_kotlin(who) {
            world.restart_external(who);
        } else {
            let saved = self.save(world, who);
            self.restore_from(world, who, &saved);
        }
    }

    /// A copy as it stands, as the text it saves of itself, to be restored later: every
    /// backend saves the same text, so a Kotlin copy's is read as a Rust copy's is.
    pub fn save(&self, world: &mut World, who: &str) -> String {
        match world.external_saved(who) {
            Some(saved) => saved,
            None => world
                .actor::<TodoPeer>(who)
                .engine()
                .save_at(0)
                .expect("a running machine saves")
                .to_json(),
        }
    }

    /// The copy killed and started again from `saved`, which is older than what it held.
    pub fn restore_from(&self, world: &mut World, who: &str, saved: &str) {
        if self.is_kotlin(who) {
            world.restore_external(who, saved);
        } else {
            let saved = SavedState::from_json(saved).expect("a saved state parses");
            world.restart::<TodoPeer>(who, move |_, clock| {
                sce_rust_runtime::Engine::<TodoPeer>::restore_with(
                    TodoPeer::new(),
                    &saved,
                    clock,
                    0,
                )
                .expect("what a machine saved restores")
            });
        }
    }

    /// The mix as words for a failure message.
    pub fn describe(&self) -> String {
        let language = |who: &str| {
            if self.is_kotlin(who) {
                "kotlin"
            } else {
                "rust"
            }
        };
        format!(
            "alice {}, bob {}, hub {}",
            language("alice"),
            language("bob"),
            language("server")
        )
    }
}

/// One line of what the network carried, as the language of the machines that put it there
/// does not show: the envelope's id and the send id inside it are each machine's own, and
/// what is the same for any machine is who sent what to whom, when, and what became of it.
pub type OnTheWire = (i64, String, String, Fate, String, String);

/// What the network carried, in order, reduced to what two implementations of one document
/// must agree on: when, from whom, to whom, what became of it, and the event it carried
/// with its data.
pub fn on_the_wire(world: &World) -> Vec<OnTheWire> {
    world
        .trace()
        .into_iter()
        .map(|line| {
            let envelope = Envelope::decode(&mut SceCursor::new(&line.bytes)).expect("an envelope");
            (
                line.at_ms,
                line.from,
                line.to,
                line.fate,
                envelope.event_type.to_string(),
                String::from_utf8_lossy(envelope.data).into_owned(),
            )
        })
        .collect()
}
