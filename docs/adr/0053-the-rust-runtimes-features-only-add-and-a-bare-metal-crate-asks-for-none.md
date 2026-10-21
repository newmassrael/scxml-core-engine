# ADR 0053 — The Rust runtime's features only add, and a bare-metal crate asks for none

- Status: Accepted (2026-10-21) and implemented; amended 2026-10-21, when the owner answered the
  three questions it left open (see the amendment at the end)
- Date: 2026-10-21
- Scope: the `[features]` of `sce-rust-runtime` (`backends/rust/runtime/Cargo.toml`), the `cfg`
  that selects its two profiles in every file of `backends/rust/runtime/src`, the manifests of the
  crates in this repository that depend on it, the MCU probes under `backends/rust/probes` and the
  gates that build them (`scripts/gates/nostd-mcu.sh`, `scripts/gates/rustdoc-links.sh`), and the
  text of the two `sce-codegen generate --no-std` refusals that named the removed feature
- Related: SCE Protocol-Synthesis RFC §5.J.2 (the bare-metal variant of the Rust statechart
  runtime, whose "`no_std` feature gate" this ADR respells), ADR 0045 (the session id a bare-metal
  host gives), `backends/rust/probes/portable-emit` (one `--no-std` emission compiles against both
  profiles), `backends/rust/probes/nostd-footprint/footprint.budget`

## Context

A downstream workspace holds a crate for a microcontroller and a crate for an application
processor, both on `sce-rust-runtime`. The MCU crate wrote `default-features = false, features =
["no_std"]`, the application crate took the defaults, and each built alone. `cargo check
--workspace`, or one `cargo test -p A -p B`, stopped in the runtime:

```text
error: `no_std` and `http-send` are mutually exclusive: tokio/reqwest are std-coupled. ...
```

Cargo builds one copy of a crate for every package of one build that depends on it, with the union
of the features they ask for, so that union held `no_std` from one crate and the default
`http-send` from the other. Cargo's rule is that features add; this crate's did not.

What the two modes changed, read in the code on 2026-10-21:

- **The crate root.** `no_std` made the crate `#![no_std]`, with no allocator.
- **Public types, by alias.** `SceString` was `String` or `heapless::String<256>`;
  `SceTransitionBuf<T>` and `helpers::hierarchy::StateChain<S>` were `Vec` or a fixed
  `heapless::Vec`; `SceBytes<N>` was `Vec<u8>` or `heapless::Vec<u8, N>` through
  `sce-portable-bytes/alloc`, which the default features turned on; `EventQueueManager` and
  `PullScheduler` held a `VecDeque`/`Vec` or a fixed `heapless` ring; `EventMetadata` carried the
  `_event` strings or elided them.
- **Whole modules and methods, present with `std` only.** `event_payload`, `host_processor`,
  `http`, `invoke`, `json`, `saved_state`, `scripting` and their re-exports; the `Engine` methods
  that serve invoke, host processors, HTTP and saved state; a session id `given` or `drawn`.
- **Names present without `std` only.** `PullScheduler::pop_ready_event_at` (public), and the
  `BoundedPush` implementation for `heapless::Vec`.
- **Dependencies.** `heapless` was optional and turned on by `no_std`; `sce-portable-bytes/alloc`
  was in `default`; `getrandom` is selected by target, not by feature.
- **`http-send` gated nothing.** No `cfg` read it except the `compile_error!`; the HTTP module
  and the engine's HTTP callback were compiled whenever `no_std` was off.

So removing the `compile_error!` alone would not have repaired the workspace. `no_std` took away:
in a build that unified it with an application crate, the application crate would have lost the
script engine seam, invoke, the HTTP processor and saved state, and failed to compile, or, if it
used only the aliases, had its strings silently become 256-byte bounded ones.

What is to be decided: a shape of the features under which every combination compiles and each
crate of a unified build still gets what it asked for; the bare-metal build unchanged in behaviour
and footprint; the application default build unchanged.

## Decision

**`std` is a feature, on by default, and the bare-metal profile is its absence.**

```toml
default = ["std", "http-send"]
std = ["sce-portable-bytes/alloc"]
http-send = ["std"]
```

Every `cfg(feature = "no_std")` is `cfg(not(feature = "std"))` and every
`cfg(not(feature = "no_std"))` is `cfg(feature = "std")`; the crate root is
`#![cfg_attr(not(feature = "std"), no_std)]`. `heapless` is an unconditional dependency: Cargo can
turn a dependency on with a feature but not with the absence of one, and `sce-portable-bytes`
already depends on it unconditionally, so every build compiled it before. The `compile_error!` is
gone, and nothing in the crate refuses a combination of features.

**The `no_std` feature is removed, not kept as an alias.** An empty `no_std = []` would be additive,
and would say the opposite of what it does: a manifest that names it and keeps the default features
would get `std`, where it used to get a loud refusal. Without it Cargo refuses the old manifest at
resolution and names the feature it does not have, and the repair is to delete
`features = ["no_std"]`.

**`http-send` asks for `std`.** It is never inert: a crate that sends over HTTP and names the
feature turns `std` on for the whole build, so it cannot lose the processor beside a bare-metal
crate. It adds nothing beyond `std` today, as it added nothing before; the processor lives in the
`std` profile. Whether it should take the processor out of a `std` build that does not ask for it
is left open below.

**The `std` profile declares every public name the bare-metal profile declares.** That is what
lets a crate written against the bare-metal profile compile in a build where another crate turned
`std` on. Two names broke it and are repaired: `BoundedPush` is implemented for `heapless::Vec` in
both profiles (it is `core`-only code, and a bare-metal crate's own bounded list pushes through it),
and `PullScheduler::pop_ready_event_at`, whose only caller is the engine's bare-metal drain and
which has no `std` counterpart of the same shape, is private to the crate.

**What each crate of a unified build gets.** The union is the `std` profile, and the bare-metal
crate is compiled against it on the host; that is what a host `cargo check --workspace` or a host
test run of that crate means. Its firmware is built by a command that selects it alone for its
target (`cargo build -p <mcu crate> --target thumbv7em-none-eabihf`), and Cargo unifies features
only across the packages one command builds, so that build gets the bare-metal profile: the image,
the only place its bounded storage and its allocator-freedom matter, is what it asked for. A build
that selected the application crate for a bare-metal target would fail with or without this ADR.

**The profile aliases change storage, not names.** Generated code names the aliases and the
runtime's constructors and never a storage type (`portable-emit` holds that), so one `--no-std`
emission compiles in both profiles. Hand-written bare-metal code that is to live beside an
application crate does the same: it uses what both storages offer (`sce_string_from_str`,
`push_bounded`, `SceBytes::from_slice`, the `EventMetadata` constructors rather than a struct
literal, whose fields differ by profile). Code that calls a method only `heapless` has on an alias
(`heapless::String::push_str` answers a `Result`, `String::push_str` does not) compiles in the
bare-metal profile alone, and a workspace that holds it beside an application crate fails to
compile, loudly, in that crate.

**A probe holds the decision.** `backends/rust/probes/feature-unification` is a workspace of a
bare-metal crate (no features, the `portable-emit` machine, its own `heapless::Vec` pushed through
`BoundedPush`) and an application crate (the defaults, with code that compiles only where
`SceString` is `String` and the HTTP processor exists). The `nostd-mcu` gate checks the workspace
on the host and builds the bare-metal crate alone for `thumbv7em-none-eabihf`; the second build
fails if a feature of the application crate reaches it, since that target has no `std`. The two
steps replace the two builds `portable-emit` made of itself, which they contain.

**The refusals name what is true now.** `codegen/no-std-script-not-supported` and
`codegen/no-std-http-not-supported` said the runtime's "no_std feature is incompatible with" the
script engine and `http-send` features. They now say that a script engine needs an allocator the
runtime without `std` has not, and that the BasicHTTP processor is part of the `std` feature
`http-send` asks for. A diagnostic's id excludes its message, so no id moved.

## Consequences

- A downstream bare-metal crate writes
  `sce-rust-runtime = { ..., default-features = false }` and nothing else. An application crate
  that takes the defaults writes nothing new; one that wrote `default-features = false` to leave
  out `http-send` writes `default-features = false, features = ["std"]`, as `sce-rust-lua`,
  `sce-rust-mesh`, `sce-rust-world`, `sce-rust-scenarios`, `sce-rust-host-features` and
  `sce-specs` now do. Before, `default-features = false` alone meant the `std` profile; it now
  means the bare-metal one, which those crates would have met as missing names.
- The bare-metal build compiles the same code. Its features were `no_std` (which turned on
  `heapless`) with `http-send` and `sce-portable-bytes/alloc` off; they are now none, with
  `heapless` a plain dependency and the same two off, so every `cfg` in the crate evaluates as it
  did. The footprint budget is expected to hold unchanged and is re-measured on the first gate run.
- Those crates now get `sce-portable-bytes/alloc` with `std`. In this workspace they always did,
  through `sce-build`'s default dependency; a lone build of one of them used to hold `SceBytes` in
  a fixed `heapless::Vec` inside the `std` profile, a combination nothing chose.
- A host test run of a workspace runs a bare-metal crate's tests against the `std` profile, where a
  bounded store cannot overflow. Bounded behaviour is tested by a build of that crate alone, as the
  queue-size probe does on the MCU target.
- The RFC's §5.J.2 text still spells the old feature; the RFC is the ledger's mirror and is not
  edited here, and this ADR is where the spelling changed.

## Alternatives considered

- **Delete the `compile_error!` and keep `no_std`.** The workspace would have compiled the
  runtime and then failed in the application crate, which lost every `std` module, or kept
  compiling with its strings bounded at 256 bytes without a sign.
- **Make `http-send` inert under `no_std`.** It would have added a silent loss to the same
  subtraction: an application crate beside a bare-metal one would send no HTTP request and be told
  nothing.
- **Both profiles in one build, the profile chosen by type.** The engine, its events and every
  generated machine would take the profile as a type parameter (as `StatePolicy::EventQueue` and
  `ScheduledSendId` already choose storage per machine), and `std` would only add the allocating
  profile. A bare-metal crate would keep its bounded types even inside a unified host build. It
  changes the public types of the default application build and every generated machine, against
  the requirement that build stay as it is, and the image it would protect is already built alone.
  It is the direction to take if a unified host test run must exercise bounded behaviour.
- **Name the feature `alloc`.** The profile needs more than an allocator: `HashMap`,
  `std::error::Error`, the system's randomness and clock. A no-`std` profile with an allocator would
  be a third, additive feature between the two; nothing asks for it.

## Questions left open

Answered by the owner on 2026-10-21; the answers are the amendment below, and the questions stand
here as they were asked.

1. **`no_macrostep_diagnostics` still takes away.** It removes `truncated_macrosteps` and its
   siblings, so a build that unifies a crate naming it with one that reads them fails to compile in
   the reader. The failure is loud, never a silent loss of the report, which is the property the
   opt-out was chosen for on 2026-08-21 ("a consumer who does nothing must keep the report"). An
   additive `macrostep-diagnostics` in `default` would drop the report from every bare-metal crate
   that writes `default-features = false`, the opposite of that decision; accessors that answer "not
   recorded" would change the type every reader reads. The choice is the owner's.
2. **Whether `http-send` should take the HTTP processor out of a `std` build that does not name
   it.** Six crates here ask for `std` without it and still compile the processor. Gating it would
   make the feature mean what it says, and would need the engine's BasicHTTP route and the
   generator to agree on what a `std` build without it does with an HTTP send.
3. **The forge `bounded_collection` template emits `cfg(feature = "no_std")` into a generated forge
   crate**, naming that crate's own feature, the same subtracting shape. It is the consumer's
   feature, not this crate's, and is not changed here.

## Amendment, 2026-10-21: the three questions, answered toward features that only add

The owner's rule, which decided all three: a feature only adds, so that one Cargo build can unify
a crate for a microcontroller with a crate for an application processor.

### 1. The macrostep report is the feature `macrostep-diagnostics`, in `default`

```toml
default = ["std", "http-send", "macrostep-diagnostics"]
macrostep-diagnostics = []
```

Every `cfg(not(feature = "no_macrostep_diagnostics"))` is `cfg(feature = "macrostep-diagnostics")`.
The report is `truncated_macrosteps`, `last_truncated_macrostep_state`, `truncated_event_chains`,
`last_truncated_event`, the counters behind them and the log line of a refused macrostep; the bound
(`MAX_MACROSTEP_MICROSTEPS`, the external-event budget, `last_macrostep_truncated`) is in every
build, as before.

- **A consumer who writes nothing keeps the report**, which is the 2026-08-21 decision: it is in
  `default`.
- **A bare-metal crate that writes `default-features = false` does not carry it**, the same
  pattern as `std`: the counters are per engine, which is the RAM the opt-out was offered to
  save. The question above called this "the opposite of that decision"; the owner weighed it and
  chose it, because the crate that writes `default-features = false` has stated what it asks for,
  and a feature that removes cannot be unified. A bare-metal crate that reads the report asks for
  `macrostep-diagnostics`.
- **A unified host build gets it** whenever any crate of the build takes the defaults or names
  it, so an application crate that reads the accessors compiles beside a bare-metal crate that
  does not. The bare-metal crate's firmware, built alone for its target, gets what it asked for.
- **`no_macrostep_diagnostics` is removed with no alias**, for the reason `no_std` was: an empty
  feature of that name would say the opposite of what it does. Cargo refuses a manifest that still
  names it, with the feature named, and the repair is to delete it; a bare-metal crate that named
  it already has, with `default-features = false`, the build it asked for.
- **An application crate that writes `default-features = false`** to leave out `http-send` names
  `macrostep-diagnostics` beside `std` when it reads the report or wants the refusal logged. In
  this repository `sce-rust-tests` reads it and names it; the library crates that ask for `std`
  alone (`sce-rust-lua`, `sce-rust-mesh`, `sce-rust-world`, `sce-rust-scenarios`,
  `sce-rust-host-features`, `sce-specs`) read nothing of it and do not ask, so their lone builds
  carry no report and the application that links them decides.

What holds it: the application crate of `backends/rust/probes/feature-unification` reads
`truncated_macrosteps` and `truncated_event_chains`, and the bare-metal crate beside it asks for
nothing, so the gate's host `cargo check --workspace` fails if the union loses the report and its
thumb build of the bare-metal crate alone builds the runtime without it. The footprint probe weighs
the same two builds it weighed before: `TOTAL_BYTES` with its `macrostep-diagnostics` feature
(formerly no feature), `TOTAL_BYTES_NO_DIAGNOSTICS` with no feature (formerly
`no_macrostep_diagnostics`). The code each build compiles did not change, so both figures are
expected to hold; they are re-measured on the first run of the `nostd-mcu` gate.

### 2. `http-send` is the HTTP processor, and a `std` build without it refuses the send

`http-send` gates what it names: the `http` module (`HttpSendRequest`, `HttpSendResponse`),
`Engine::set_http_send_callback` and the callback behind it. `default` keeps it, and it still asks
for `std`.

A `std` build without it has no BasicHTTP processor, and a BasicHTTP send there names a type the
platform does not support. W3C SCXML 6.2 answers that case already ("If the SCXML Processor does
not support the type that is specified, it MUST place the event error.execution on the internal
event queue"), and the runtime already has the place it decides it,
`helpers::send::is_supported_send_type`, which the generated `typeexpr` check calls. That function
now counts the BasicHTTP type only with `http-send`, and every BasicHTTP route asks it:

- **The generated send site.** The generator cannot see the runtime's features, so the Rust
  template wraps the BasicHTTP arm in a run-time question to that function, asked before the
  payload is read, as the build-time refusals are: in a build without the processor the send
  raises one `error.execution` carrying its send id, with the words the unsupported-type arm uses,
  posts nothing and ends its block (W3C SCXML 4.9). With the processor the arm is the one it was.
- **`Engine::perform_http_send` and `Engine::schedule_http_send`** stay in every `std` build,
  because a generated machine names them whatever its build's features are, so one generation
  compiles in both builds. Without the processor they refuse the same way, and a delayed send is
  refused when it runs rather than armed for a deadline nothing could perform. What reaches the
  engine's refusal is a host's own call, or a delayed send a saved state carried from a build that
  had the processor; its deadline refuses it as an unregistered host processor's is.

The six library crates that ask for `std` without `http-send` no longer compile the processor;
none of them registers a transport for it, and `sce-rust-scenarios`, which compiles the
`static_send_http` machine, replays no scenario of it. `sce-rust-tests` names `http-send`, and
the W3C suite manifest
`sce-codegen generate-w3c` writes names it, so the W3C C.2 tests run as before. The text of
`codegen/no-std-http-not-supported` now says the processor is the `http-send` feature, which asks
for `std`; its id did not move.

What holds it: `a_basic_http_send_needs_the_http_send_feature` in the runtime's tests and
`helpers::send`'s `supported_send_types`, each with one half for each build. The default build runs
the half with the processor; the `nostd-mcu` gate now lints and tests the runtime with
`--no-default-features --features std`, the one build in this repository with neither
`http-send` nor `macrostep-diagnostics`, and that run is the other half.
