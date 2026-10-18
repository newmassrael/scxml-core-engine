# Review brief: the trust, relay and group-key code of a shared list

This is what an independent reviewer is given before they read the code. The design is
`docs/adr/0034-a-shared-list-is-held-by-its-members-and-the-hub-only-relays.md`; what each party can be
trusted with is `docs/SCE_SHARED_LIST_TRUST.md`; the relay is `docs/SCE_SHARED_LIST_RELAY.md` and the
group key `docs/SCE_SHARED_LIST_MLS.md`. **One independent read has been done (2026-10-18), and it found
seven defects that the authors' own checks had not** (listed in the ADR, fixed, and held by
`what_an_independent_review_found_stays_fixed`); it covered the main trust, encryption and MLS-binding
paths and is not a review of the whole. This brief is what makes a review possible, and what the authors
have done in its place is listed in section 7 with what that does and does not show. What that read found
is the best evidence of what the authors' own checks miss: three of its seven were in the code the
authors' simulation, model and mutation round were written against.

## 1. What is claimed

A shared list (the to-do list of `todo_peer`) has copies on its members' devices. Its members need not
trust the host that sits between them. The claims, each of which a reviewer is asked to try to break:

1. **Authorship.** A host takes in a write or a note only if a device that was a writer under the
   membership log signed it, with the key the log gave that device, as the operation it is.
2. **Removal is a position.** A device that is taken off can have no operation past the number the log set
   for it held by any host that has the log, whatever time it claims and whatever entry it names, and what
   a host held before it knew is dropped (so that two hosts that hold the same objects hold the same list).
3. **Order does not matter.** Two hosts that were given the same entries and operations hold the same
   operations, in any order, with repeats.
4. **The relay cannot make a host hold what no member wrote**, nor forget what a settlement does not
   cover, nor read the content of a write, nor decide who the members are.
5. **A removed member derives no key of a later epoch** (MLS), and what was sealed under an epoch the list
   destroyed opens for no host that keeps to the list.
6. **A fork of the membership log, and a device that signs two operations at one number, are detected** by
   any two hosts that exchange what they hold, and the host stops taking what they affect.
7. **An audit with the hash of the genesis and no key** checks a list by the code a host uses.

## 2. Parties and what each is trusted with

| Party | Holds | Trusted to |
|---|---|---|
| Administrator | a key listed in the genesis (and the quorum's keys) | sign entries of the log; not to sign two histories (that is a fork, which is shown) |
| Member's device | its own Ed25519 key, the content keys of epochs it was in | sign what it makes once, in order; not to leak its key |
| Hub / relay | nothing secret | nothing: it may drop, delay, reorder, repeat, withhold, and invent |
| Network | nothing | nothing |
| Person (login) | a login the application owns | out of scope: the application issues the relay's tokens |

**Out of scope, stated plainly:** a compromised device before it is taken off; a member who copies a key
(content they could read stays readable to them); metadata the hub sees (which to-do, which field, when, by
whom, and the size class); traffic analysis; denial of service by a relay that holds everything back (it
is detected, not prevented); login and token issuing; TLS; the machine documents (`todo_peer.scxml`).

## 3. Primitives and where they come from

| Use | Primitive | Source |
|---|---|---|
| Signatures (entries, operations, checkpoints, links) | Ed25519 | `ed25519-dalek` 2 |
| Hash | SHA-256 | `sha2` |
| Content sealing | XChaCha20-Poly1305, nonce from the write's stamp, associated data = list, epoch, stamp | `chacha20poly1305` |
| Key agreement between members (wrapping an epoch key) | X25519 from the same Ed25519 keys, SHA-256 | `x25519-dalek`, `curve25519` conversion in `identity.rs` |
| Group key (MLS, RFC 9420) | MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519, plaintext handshake | `openmls` 0.9.1, `openmls_rust_crypto` |
| Token | HMAC-SHA256 | own (`relay/src/token.rs`), RFC 4231 vectors |
| Backup of a device's secrets | Argon2id (64 MiB, 3 passes) then XChaCha20-Poly1305 | `argon2` 0.6 |

Things a reviewer should look at first because they are ours and not a library's: the **nonce
derivation from the stamp** (`secrecy.rs`: one write is sealed one way, which needs the stamp to be unique per
write under a key); the **conversion of an Ed25519 key to X25519** for wrapping keys; the **HMAC** of
the token; the **canonical byte forms** that are signed (`message.rs`, `membership.rs`, `checkpoint.rs`),
each pinned by a known-answer vector (`trust/tests/a_byte_format_does_not_drift.rs`); the **binding of
the MLS commit to the log by hash** and of the **Welcome to the log by the confirmation tag**
(`mls/src/lib.rs`, `join`); and the padding inside the seal (`secrecy.rs`).

## 4. Code to read, in the order it is worth reading

1. `backends/rust/trust/src/membership.rs`: the log. Every entry kind and what `admit` checks.
2. `backends/rust/trust/src/message.rs`, `trust.rs` (`verify_operation`, `take`, `evict_*`,
   `verify_checkpoint`, `verify_settlement`, `admit_floor`, `seal_*`): what a host does with an operation.
3. `backends/rust/trust/src/secrecy.rs`, `identity.rs`, `backup.rs`, `audit.rs`.
4. `backends/rust/relay/src/relay.rs`, `token.rs`, `store.rs`, `client.rs` (the courier).
5. `backends/rust/mls/src/lib.rs`, `host.rs`; `backends/rust/mls-capi/src/lib.rs` (the only `unsafe`).

About 7,000 lines of Rust outside tests. `#![forbid(unsafe_code)]` holds everywhere but the C ABI.

## 5. The properties and how they are evidenced

| Claim | Evidence | Where |
|---|---|---|
| 1 | tests per forgery (changed field, wrong key, reader, stranger, other list); a mutator that changes one thing of every operation and expects every host to refuse it | `trust/tests/a_host_admits_what_its_members_signed.rs`, `assurance/.../a_changed_operation_is_taken_in_by_no_host` |
| 2, 3 | a seeded simulation of devices, a network and an administrator, with removal below what hosts held, and a device signing twice, over 400 seeds; and a TLA+ model of the rule with and without the dropping rule | `trust/tests/a_list_holds_its_safety_properties_under_every_order_of_delivery.rs`, `docs/formal/` |
| 4 | the relay holds a `Trust` with no key and no person; tests that it refuses spam, readers, the removed, plaintext in a sealed list | `relay/tests/` |
| 5 | MLS group tests with removal; the sealed-content test of a removed host; the destruction tests | `mls/tests/`, `trust/tests/a_person_has_devices_...` |
| 6 | the fork property in the simulation; equivocation in the simulation and the model | as 2 |
| 7 | export and verify tests with each kind of change to a record | `trust/tests/a_device_is_backed_up_...` |

A **finding the authors made by writing the model and the simulation**, which a reviewer should know the
history of: before they were written, a host kept an operation it had taken in before it learned that its
device was taken off at a lower number, so that two hosts that were given the same objects held different
lists for good; and a host kept what it held of a device it found signing two things at one number, and
went on signing itself. Both are fixed (`evict_past_the_cutoff`, `evict_from`, the refusal in
`make_operation`), and both are what the model's `Evict = FALSE` configuration shows.

## 6. What the authors know is weak or open

- **The stamp is in the clear** and the hub's machine orders by it. A hub learns which to-do and field
  changed when and by whom. Hiding it is another design.
- **A member who leaves with a key keeps what it could read.** A destroyed epoch is destroyed for hosts that
  keep to the list.
- **An amendment needs one checkpoint of another writer**, and an administrator's signature. A stricter
  rule (two writers) is a one-line change if the reviewer thinks one is too few.
- **The coverage-guided fuzzing is short.** Every parser is a target of libFuzzer (`scripts/fuzz/run.sh`),
  and what has been run is minutes per target, not a night: the corpus it keeps is not evidence of much
  yet, and a reviewer with a machine to spare should let it run.
- **The MLS and crypto libraries are not audited here.** `openmls` has had its own audits; this project's use
  of it (plaintext handshake, the log's binding, the confirmation-tag check of a Welcome) has not.
- **No lane but Rust has a membership log.** C, C++, Python, Go and Kotlin replay the recorded MLS
  scenario through the C ABI with the log as data, so what verifies that log in those lanes is theirs.
- **Machine documents are out of scope**, including whether `todo_peer` is a correct CRDT.
- **Timing.** Verification is not constant-time where it does not need to be (it handles public data);
  the token tag comparison is constant-time; Argon2id is the library's.

## 7. What the authors have done in place of a review

- Known-answer vectors for every byte form that is signed or sealed (and a recorded MLS scenario that
  Rust, C, C++, Python, Go and Kotlin replay).
- A seeded simulation that checks the safety properties against the code, and two TLA+ models of the
  convergence rule checked with TLC, each with a configuration that passes and one that fails as the
  history says: the dropping of what a later entry puts out, and, since the independent read, the
  conflicts of a device (two operations at a number, a write whose time goes back) held as a set. The
  models are of the rule and not of the code, and they have no stamp repeats, replaced keys or first
  epoch: the simulation is what plays those against the code.
- A fuzz of every parser of the trust, relay and group-key code and of the C ABI: a seeded mutator
  (no panic over the default run and over a run forty times longer, with a harness that is checked to
  find planted defects of each kind) and libFuzzer on the same 21 targets.
- Four mutation rounds, 152 cases in all, each taking one check away and the tests having to notice:
  the log and the operation and checkpoint checks of a host (42,
  `a_host_holds_only_what_its_log_admits_rust.cases`), the relay and its courier (38), sealing, key
  agreement, backup, audit and the checkpoint form (34), and the group key code with its binding to the
  log (38). Cases survived the tests that existed in every round, which is how checks that nothing held in
  place were found; they are held now, and two cases that no input reaches were dropped, with the reason
  in the casefile. A round measures only the checks someone thought to take away: the cases are the
  authors', and the independent read above found seven defects in code the rounds covered. One kind of
  check is outside any round, the wiping of a secret from memory, which safe code cannot observe.

None of this is independent: the people who wrote the code wrote these. It finds what they thought to
look for. It is the reason a reviewer's time can go to what they did not.

## 8. What to ask a reviewer to do

1. Try to make two hosts hold different operations after they were given the same objects.
2. Try to make a host take in an operation no member signed, or one past a cutoff.
3. Try to make a host apply an MLS commit or a Welcome the log did not name, or accept a group whose
   roster is not the log's.
4. Try to recover content, a key, or a member's identity from what a relay holds.
5. Read the nonce, key-wrapping and key-conversion code as if it were new.
6. Say whether the claims in section 1 are the ones the list needs, and which are missing.

Running it: `cargo test -p sce-rust-trust -p sce-rust-relay -p sce-rust-mls -p sce-mls-capi
-p sce-rust-assurance` (the C ABI's replay needs `cc`, `python3` and `go`, or `SCE_REQUIRE_TOOLS` unset);
the model: `scripts/formal/check.sh`; a longer fuzz: `SCE_FUZZ_FACTOR=50 cargo test -p sce-rust-assurance`.
