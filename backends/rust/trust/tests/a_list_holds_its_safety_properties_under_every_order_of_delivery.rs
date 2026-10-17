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
//
// Some seeds play what an honest device does not do, with its key, by hand: a write whose time goes
// back along its chain, one under the stamp of another, one that names the wrong operation before it;
// and some have the list begin to seal in the middle of the run, or the administrator replace a device's
// key. What is asked of those is the same: that every host, however the operations and entries came,
// holds the same operations, finds the same conflicts at the same numbers, and holds the same proof.

use std::collections::BTreeSet;

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::message::{self, Link, SIGNATURE_FIELD};
use sce_rust_trust::{encoding, Identity, TakeError, Trust};
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

/// The id, time and write number of an operation as it travels.
fn id_and_time_of(data: &str) -> ([u8; 32], i64, u64) {
    let mut value: Value = serde_json::from_str(data).unwrap();
    value.as_object_mut().unwrap().remove(SIGNATURE_FIELD);
    let link = Link::take_from(&mut value).unwrap();
    let signing = message::signing(LIST, "todo.write", &value)
        .unwrap()
        .unwrap();
    let id = message::operation_id(&message::operation_bytes(&signing.message, &link));
    let wall = value["wallTime"].as_str().unwrap().parse().unwrap();
    let write = value["id"].as_str().unwrap().parse().unwrap();
    (id, wall, write)
}

/// A write of `node` signed by `signer` by hand, numbered `seq`, naming `prev` before it and the entry
/// `host` is at.
#[allow(clippy::too_many_arguments)]
fn forged_write(
    host: &Trust,
    signer: &Identity,
    node: u64,
    seq: u64,
    prev: [u8; 32],
    id: u64,
    wall: i64,
    value: i64,
) -> String {
    let mut data = json!({"id": id.to_string(), "field": "3", "value": value.to_string(), "text": "",
        "wallTime": wall.to_string(), "counter": "0", "nodeId": node.to_string()});
    let signing = message::signing(LIST, "todo.write", &data)
        .unwrap()
        .unwrap();
    let (next, log_hash) = host.head();
    let link = Link {
        seq,
        prev,
        log: next - 1,
        log_hash,
    };
    let bytes = message::operation_bytes(&signing.message, &link);
    link.write_into(&mut data).unwrap();
    data[SIGNATURE_FIELD] = Value::String(encoding::encode(&signer.sign(&bytes)));
    data.to_string()
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
    /// Whether a device in this run signs, by hand, a write whose time goes back, one under a stamp
    /// it used, or one that names the wrong operation before it.
    forging: bool,
    /// Whether the list begins to seal somewhere in this run.
    sealing: bool,
    sealed: bool,
    /// The device whose key the administrator replaces in this run (with `person(node + 10)`).
    replacing: Option<u64>,
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
            forging: seed % 4 == 1,
            sealing: seed % 5 == 2,
            sealed: false,
            replacing: (seed % 7 == 3).then(|| DEVICES[(seed / 7) as usize % DEVICES.len()]),
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
            _ => self.mischief(),
        }
    }

    /// What this seed has a device do that an honest one does not, when it has any.
    fn mischief(&mut self) {
        let mut kinds: Vec<u8> = Vec::new();
        if self.equivocating {
            kinds.push(0);
        }
        if self.forging {
            kinds.push(1);
        }
        if self.sealing && !self.sealed {
            kinds.push(2);
        }
        if kinds.is_empty() {
            return self.give_op();
        }
        match kinds[self.rng.below(kinds.len())] {
            0 => self.equivocate(),
            1 => self.forge(),
            _ => {
                // The list begins to seal: the devices hold no key, so they write nothing more, and
                // what they wrote before names an entry before the epoch.
                self.sealed = true;
                self.publish(Body::Epoch { epoch: 1 });
            }
        }
    }

    /// A device signs by hand, with its key, what its own host does not: a write that goes back in
    /// time along its chain, one under the stamp of the last, or one that names another operation than
    /// the last before it. It is made from what the device's own host holds of it.
    fn forge(&mut self) {
        let device = self.rng.below(DEVICES.len());
        let node = DEVICES[device];
        let host = self.hosts[device + 1].clone();
        let last = host.last_seq(node);
        if last == 0 || host.role(node) < 2 {
            return;
        }
        let Some((_, data)) = host.operations(node, last, last).into_iter().next() else {
            return;
        };
        let (last_id, last_wall, last_write) = id_and_time_of(&data);
        let id = self.next_id;
        self.next_id += 1;
        let (prev, wall, write_id) = match self.rng.below(3) {
            // Its time goes back.
            0 => (last_id, last_wall - 500, id),
            // The same stamp as the last: another content under it.
            1 => (last_id, last_wall, last_write),
            // Another operation than the last before it.
            _ => ([7; 32], last_wall + 1, id),
        };
        let forged = forged_write(
            &host,
            &person(node),
            node,
            last + 1,
            prev,
            write_id,
            wall,
            id as i64 + 1000,
        );
        self.ops.push(("todo.write".to_string(), forged));
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
            let entry = self.log[self.given[host]].clone();
            self.hosts[host].append(&entry).unwrap();
            self.follow_key(host, &entry);
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
        // In some runs the administrator does not take a device off but gives it another key, which
        // ends its chain at the number the administrator's host has seen and begins another there.
        if let Some(node) = self.replacing {
            self.publish(Body::Grant {
                node,
                role: 2,
                key: person(node + 10).public(),
                final_seq: Some(self.hosts[0].last_seq(node)),
            });
            return;
        }
        let node = DEVICES[self.rng.below(DEVICES.len())];
        self.publish(Body::Grant {
            node,
            role: 0,
            key: person(node).public(),
            final_seq: Some(self.hosts[0].last_seq(node)),
        });
    }

    /// A device's host that is given the entry that replaces its key signs with the new key from then on.
    fn follow_key(&mut self, host: usize, entry: &Entry) {
        let (
            Some(node),
            Body::Grant {
                node: named, key, ..
            },
        ) = (self.replacing, &entry.body)
        else {
            return;
        };
        let identity = person(node + 10);
        if node == *named && *key == identity.public() && host == node as usize {
            self.hosts[host].set_identity(node, identity);
        }
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
        let entry = self.log[self.given[host]].clone();
        self.hosts[host].append(&entry).unwrap();
        self.follow_key(host, &entry);
        self.given[host] += 1;
    }

    /// The operations a host tells its machine, as the text they travel with.
    fn held(&self, host: usize) -> BTreeSet<String> {
        self.hosts[host]
            .all_operations()
            .into_iter()
            .map(|(_, data)| data)
            .collect()
    }

    /// Every operation a host holds, the proof of a conflict included.
    fn signed(&self, host: usize) -> BTreeSet<String> {
        self.hosts[host]
            .signed_operations()
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
        // What each host tells its machine, what it holds in all (the proof of a conflict included),
        // and where it finds the devices' chains to conflict, are the same at every host.
        let (first, all, conflicts) = (
            world.held(0),
            world.signed(0),
            world.hosts[0].equivocating(),
        );
        for host in 1..world.hosts.len() {
            if world.held(host) != first
                || world.signed(host) != all
                || world.hosts[host].equivocating() != conflicts
            {
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
                // A device that was given another key (some runs) goes on past the number its old chain
                // ended at, with the new key: that is a new chain, not an operation past the cutoff,
                // and what is asked of it is what the convergence test asks of every device.
                if world.replacing == Some(node) {
                    continue;
                }
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
            .draft(Body::Epoch {
                epoch: world.hosts[0].epoch() + 1,
            })
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
