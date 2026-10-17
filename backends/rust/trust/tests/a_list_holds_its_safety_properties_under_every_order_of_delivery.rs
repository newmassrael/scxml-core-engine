// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The properties ADR 0034 states for the list, checked against the code and not against a drawing of
// it. A small world is run under a seeded scheduler: an administrator's host that holds the log, three
// devices that write when their own log says they may, and a network that gives each host the entries
// (in order) and the operations (in any order, and again) when it likes, including after a device was
// taken off at a number the administrator's host had seen and others had gone past. When everything
// has been given to everyone, what is asked is: that every host holds the same operations (two hosts
// that have been given the same objects hold the same list, whatever order they came in); that no host
// holds an operation of a device numbered past what the log accepted of it (nothing is admitted
// beyond a `final_seq`, in hindsight as well); and that no changed operation is ever taken in
// (a relay that holds no key cannot make a host hold what no member wrote). A failing seed is printed:
// the run is reproduced by it.

use std::collections::BTreeSet;

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, TakeError, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const DEVICES: [u64; 3] = [1, 2, 3];

/// splitmix64: a seeded source that is the same on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

/// A write with a stamp that goes forward along a device's chain.
fn write(author: u64, id: u64, wall: i64) -> String {
    json!({"id": id.to_string(), "field": "3", "value": id.to_string(), "text": "",
           "wallTime": wall.to_string(), "counter": "0", "nodeId": author.to_string()})
    .to_string()
}

struct World {
    rng: Rng,
    /// Host 0 holds the administrator's log and no person; hosts 1.. are the devices.
    hosts: Vec<Trust>,
    /// The entries, in order: what every host is given, each in turn.
    log: Vec<Entry>,
    /// How many entries each host has been given.
    given: Vec<usize>,
    /// Every operation made, as (event, data), and which hosts were given it.
    ops: Vec<(String, String)>,
    delivered: Vec<BTreeSet<usize>>,
    next_id: u64,
    removed: bool,
    /// Whether a device in this run signs two operations at one number somewhere.
    equivocating: bool,
}

impl World {
    fn new(seed: u64) -> Self {
        let genesis = Log::genesis_draft(
            LIST,
            Body::Genesis {
                administrators: vec![admin().public()],
                quorum: 1,
                profile: Profile::Untrusted,
            },
        )
        .sign(&admin())
        .finish();
        let mut hosts = vec![Trust::open(LIST)];
        for node in DEVICES {
            let trust = Trust::open(LIST);
            trust.set_identity(node, person(node));
            hosts.push(trust);
        }
        for host in &hosts {
            host.append(&genesis).unwrap();
        }
        let hosts_len = hosts.len();
        let mut world = Self {
            rng: Rng(seed),
            hosts,
            log: vec![genesis],
            given: vec![1; hosts_len],
            ops: Vec::new(),
            delivered: vec![BTreeSet::new(); hosts_len],
            next_id: 1,
            removed: false,
            equivocating: seed % 3 == 0,
        };
        for node in DEVICES {
            world.publish(Body::Grant {
                node,
                role: 2,
                key: person(node).public(),
                final_seq: None,
            });
        }
        world
    }

    /// The administrator's host appends an entry, which the others are given in time.
    fn publish(&mut self, body: Body) {
        let entry = self.hosts[0].draft(body).sign(&admin()).finish();
        self.hosts[0].append(&entry).unwrap();
        self.log.push(entry);
        self.given[0] = self.log.len();
    }

    fn step(&mut self) {
        match self.rng.below(12) {
            0..=3 => self.write(),
            4..=6 => self.give_entry(),
            7..=9 => self.give_op(),
            10 => self.remove(),
            _ => {
                if self.equivocating {
                    self.equivocate();
                } else {
                    self.give_op();
                }
            }
        }
    }

    /// A device signs two different operations at one number: its key is on two hosts that have not
    /// heard of each other, which is what a copied or leaked key does.
    fn equivocate(&mut self) {
        let device = self.rng.below(DEVICES.len());
        let node = DEVICES[device];
        let host = self.hosts[device + 1].clone();
        if host.role(node) < 2 {
            return;
        }
        let twin = Trust::open(LIST);
        for entry in &self.log[..self.given[device + 1]] {
            twin.append(entry).unwrap();
        }
        twin.set_identity(node, person(node));
        twin.restore_from_text(&host.to_text());
        for (index, mut sealer) in [host, twin].into_iter().enumerate() {
            let id = self.next_id;
            self.next_id += 1;
            let Ok(Some(sent)) =
                sealer.seal("#server", "todo.write", &write(node, id, 1000 + id as i64))
            else {
                return;
            };
            self.ops.push(("todo.write".to_string(), sent));
            if index == 0 {
                self.delivered[device + 1].insert(self.ops.len() - 1);
            }
        }
    }

    /// A device writes, when its own log says it may.
    fn write(&mut self) {
        let device = self.rng.below(DEVICES.len());
        let host = &self.hosts[device + 1];
        if host.role(DEVICES[device]) < 2 {
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        let wall = 1000 + id as i64;
        let mut sealer = host.clone();
        let Ok(Some(sent)) =
            sealer.seal("#server", "todo.write", &write(DEVICES[device], id, wall))
        else {
            return;
        };
        self.ops.push(("todo.write".to_string(), sent));
        // The device holds what it made.
        self.delivered[device + 1].insert(self.ops.len() - 1);
    }

    fn give_entry(&mut self) {
        let host = self.rng.below(self.hosts.len());
        if self.given[host] < self.log.len() {
            self.hosts[host]
                .append(&self.log[self.given[host]])
                .unwrap();
            self.given[host] += 1;
        }
    }

    fn give_op(&mut self) {
        if self.ops.is_empty() {
            return;
        }
        let host = self.rng.below(self.hosts.len());
        let op = self.rng.below(self.ops.len());
        self.deliver(host, op);
    }

    /// The administrator takes one device off, at the last operation of it that the administrator's
    /// host has seen, once.
    fn remove(&mut self) {
        if self.removed {
            return;
        }
        self.removed = true;
        let node = DEVICES[self.rng.below(DEVICES.len())];
        self.publish(Body::Grant {
            node,
            role: 0,
            key: person(node).public(),
            final_seq: Some(self.hosts[0].last_seq(node)),
        });
    }

    /// Give an operation to a host. One that names an entry the host has not been given yet waits.
    fn deliver(&mut self, host: usize, op: usize) {
        let (event, data) = self.ops[op].clone();
        match self.hosts[host].take_operation(&event, &data) {
            Err(TakeError::NotReached(_)) => {}
            _ => {
                self.delivered[host].insert(op);
            }
        }
    }

    /// Everything to everyone, in a random order and again: what eventual delivery means.
    fn settle(&mut self) {
        for host in 0..self.hosts.len() {
            while self.given[host] < self.log.len() {
                self.give_entry_to(host);
            }
        }
        let mut pairs: Vec<(usize, usize)> = (0..self.hosts.len())
            .flat_map(|host| (0..self.ops.len()).map(move |op| (host, op)))
            .collect();
        for index in (1..pairs.len()).rev() {
            let other = self.rng.below(index + 1);
            pairs.swap(index, other);
        }
        for (host, op) in pairs.iter().chain(pairs.iter()) {
            self.deliver(*host, *op);
        }
    }

    fn give_entry_to(&mut self, host: usize) {
        self.hosts[host]
            .append(&self.log[self.given[host]])
            .unwrap();
        self.given[host] += 1;
    }

    /// The operations a host holds, as the text they travel with.
    fn held(&self, host: usize) -> BTreeSet<String> {
        self.hosts[host]
            .all_operations()
            .into_iter()
            .map(|(_, data)| data)
            .collect()
    }
}

#[test]
fn hosts_that_were_given_the_same_objects_hold_the_same_operations_whatever_the_order() {
    let mut failures = Vec::new();
    for seed in 0..400u64 {
        let mut world = World::new(seed);
        for _ in 0..60 {
            world.step();
        }
        world.settle();
        let first = world.held(0);
        for host in 1..world.hosts.len() {
            if world.held(host) != first {
                failures.push((
                    seed,
                    (0..world.hosts.len())
                        .map(|host| (world.hosts[host].chains(), world.hosts[host].equivocating()))
                        .collect::<Vec<_>>(),
                ));
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "hosts hold different operations after everything was given to everyone: {} of 400 seeds, the first {:?}",
        failures.len(),
        &failures[..failures.len().min(2)]
    );
}

#[test]
fn no_host_holds_an_operation_past_what_the_log_accepted_of_its_device_in_hindsight_too() {
    let mut failures = Vec::new();
    for seed in 0..400u64 {
        let mut world = World::new(seed);
        for _ in 0..60 {
            world.step();
        }
        world.settle();
        // The log is whole at every host now: what each holds of a device must be within the last
        // number the log accepted of it.
        let cutoffs: Vec<(u64, Option<u64>)> = DEVICES
            .iter()
            .map(|node| {
                let cutoff = world.hosts[0]
                    .member_history()
                    .into_iter()
                    .find(|(held, _)| held == node)
                    .and_then(|(_, eras)| eras.iter().filter_map(|era| era.cutoff).min());
                (*node, cutoff)
            })
            .collect();
        for host in 0..world.hosts.len() {
            for (node, count, last) in world.hosts[host].chains() {
                let _ = count;
                if let Some((_, Some(cutoff))) = cutoffs.iter().find(|(held, _)| *held == node) {
                    if last > *cutoff {
                        failures.push((seed, host, node, last, *cutoff));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "a host holds an operation past what the log accepted ({} cases; seed, host, node, held, accepted), the first {:?}",
        failures.len(),
        &failures[..failures.len().min(8)]
    );
}

#[test]
fn a_changed_operation_is_taken_in_by_no_host() {
    for seed in 0..120u64 {
        let mut world = World::new(seed);
        for _ in 0..40 {
            world.step();
        }
        world.settle();
        for op in 0..world.ops.len() {
            let (event, data) = world.ops[op].clone();
            let mut value: Value = serde_json::from_str(&data).unwrap();
            // Change one thing the author signed, or the signature, or where it stands in the chain.
            let field = ["value", "wallTime", "sig", "seq", "prev", "mhh"][world.rng.below(6)];
            let current = value[field].as_str().unwrap().to_string();
            let changed = if field == "value" || field == "wallTime" || field == "seq" {
                (current.parse::<i64>().unwrap() + 1).to_string()
            } else {
                let mut bytes = current.into_bytes();
                bytes[0] = if bytes[0] == b'A' { b'B' } else { b'A' };
                String::from_utf8(bytes).unwrap()
            };
            value[field] = Value::String(changed);
            for host in &world.hosts {
                assert!(
                    host.take_operation(&event, &value.to_string()).is_err(),
                    "seed {seed}: a host took in an operation whose `{field}` was changed"
                );
            }
        }
    }
}

/// Two entries at one position, signed by the administrator's key: whichever hosts were given
/// whichever, any two of them that exchange entries both find the fork, say where, and take in
/// nothing more until it is resolved.
#[test]
fn a_fork_of_the_log_is_found_by_any_two_hosts_that_exchange_entries() {
    for seed in 0..200u64 {
        let mut rng = Rng(seed);
        let mut world = World::new(seed);
        for _ in 0..rng.below(20) {
            world.step();
        }
        world.settle();
        let position = world.log.len();
        // Two different entries at the next position, from the same head.
        let one = world.hosts[0]
            .draft(Body::Epoch { epoch: 1 })
            .sign(&admin())
            .finish();
        let other = world.hosts[0]
            .draft(Body::Grant {
                node: 9,
                role: 1,
                key: person(9).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        // Each host is given one of them, by chance; at least one host is given each.
        let mut sides: Vec<bool> = (0..world.hosts.len()).map(|_| rng.below(2) == 0).collect();
        sides[0] = true;
        let last = sides.len() - 1;
        sides[last] = false;
        for (host, side) in sides.iter().enumerate() {
            world.hosts[host]
                .append(if *side { &one } else { &other })
                .unwrap();
        }
        // They exchange: every host is shown the entry it was not given.
        for (host, side) in sides.iter().enumerate() {
            let shown = if *side { &other } else { &one };
            assert!(
                matches!(
                    world.hosts[host].append(shown),
                    Err(sce_rust_trust::LogError::Fork { index }) if index == position as u64
                ),
                "seed {seed}: host {host} was not told of the fork"
            );
            assert_eq!(world.hosts[host].forked(), Some(position as u64));
            // And it takes in no operation from then on.
            if let Some((event, data)) = world.ops.first() {
                assert!(world.hosts[host].take_operation(event, data).is_err());
            }
        }
    }
}
