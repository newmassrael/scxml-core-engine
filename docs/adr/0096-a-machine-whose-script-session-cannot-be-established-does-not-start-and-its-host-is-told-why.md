# ADR 0096 — A machine whose script session cannot be established does not start, and its host is told why

- Status: Accepted (2026-11-09). Decided under the owner's standing rule of 2026-11-09: the most long-term correct
  structure, textbook in its foundations, cost ignored, decided without asking. The owner chose this item and the order
  of the three that came with it ("broadcast dead branch, then this, then the wire-17 reply_to").
- Date: 2026-11-09
- Amends:
  - ARCHITECTURE.md, "Static Hybrid", whose "Lazy Initialization: Script engine session created only when needed"
    describes one backend of seven and is replaced by "Decision" 2;
  - SCE_ERROR_CONTRACT.md, which states no rule for a session that cannot be established and gains one ("Decision" 3);
  - the start refusal every engine carries since `needs_parent` (SCE_ERROR_CONTRACT.md §10.1), which gains a second
    reason ("Decision" 4).
- Scope: the script-session setup of the generated machine in all seven backends (templates
  `tools/codegen/templates/{scriptengine_helpers.jinja2,rust,go,kotlin,python,swift,c}/…`), the session interfaces of
  the runtimes that inject an engine (C++, Rust, Go, Kotlin, Python), the engines that implement them, and each
  engine's start API.

## Context

A machine whose document needs a datamodel runs its conditions, assignments and scripts in a script session. The
session has to be created, the shared ECMAScript-semantics library loaded into it, and the system variables
`_sessionid`, `_name` and `_ioprocessors` bound (§scxml-5.10) before any of that can run. A survey of the seven
backends on 2026-11-09 found the same failure handled seven ways, and in none of them reaching the host:

| Backend | When the session is set up | A failure today |
|---|---|---|
| C++ | at `initialize` / `enterAt`, and again lazily on first use | `createSession`'s `bool` discarded; a failed bind logged; the session marked set up anyway |
| Rust | the same | `create_session` returns nothing, a library that does not load is logged; a failed bind logged; marked set up |
| Go | at `Initialize` / `EnterAt`, lazily elsewhere | both results discarded with `_ =`; marked set up |
| Kotlin | at `initialize`, inside `start`'s coroutine, at `enterAt`, lazily elsewhere | an exception propagates (from `start`, out of a coroutine the caller has no handle on); every engine returns silently from a bind on a missing session |
| Python | session at construction, bind at `initialize` | an exception, raised after the run is marked running and the inbox opened |
| Swift | lazily, at the first condition, action or `<data>` | turned into `error.execution`, `false` or `nil`; retried on every later use |
| C11 | at `_init*` | a `NULL` Lua state skips setup silently; the library chunks run as `(void)luaL_dostring`; a failed bind prints and queues `error.execution` |

In C++ and Rust an invoked child's session is set up on the parent's stack, by the parameter channel, before the
child's own start runs; the child's start then finds it done.

What can fail is not the document. The sources are the engine's allocation (`luaL_newstate`, `JS_NewContext`, a
QuickJS memory limit), a shared library chunk that does not load (which is a build defect), and a bind that fails
because that library is absent. A machine started over such a session runs with no datamodel: every condition
evaluates to an error, every assignment is lost, and in the backends that log and continue nothing outside a log says
so. The document cannot handle it either: its means of handling an error, a transition on `error.execution` with a
condition, runs in the session that failed.

## Decision

### 1. Establishing the session is one operation that reports failure

Establishing a machine's script session is three steps in this order: create the session, load the shared library
(ECMAScript semantics, JSON builtins, and the DOM read surface where the backend has one), bind the system variables.
Each step reports failure as a value or a typed exception, never as a log line alone:

- the runtimes' session interfaces gain a failure result on creation where they have none (Rust `create_session`
  returns `ScriptResult<()>`; Kotlin and Python already signal by exception);
- an engine whose library chunk does not load fails creation and keeps no session (Rust's `setup_builtins`, C++'s
  `registerBuiltins`, the Kotlin QuickJS session left in its map, C11's `(void)luaL_dostring`);
- an engine asked to bind on a session it does not hold fails (Kotlin's three engines return silently today).

The failure names the step: `create`, `library` or `system-variables`, and the engine's message.

### 2. It is a step of the start, and the only place a session is made

`interpret` (§scxml-D-interpret) creates the datamodel before it enters the initial configuration, and §scxml-5.3 binds the
data before any state is entered. Establishing the session is therefore the first step of every start: the plain
start, the checked root start, `enterAt` (a resumed run), and an invoked child's start. It runs before the inbox opens,
before any state is entered and before an invoke's parameters are applied to the child.

The lazy setup sites (`ensureScriptEngine` and its spellings, Swift's `ensureLua`) stop setting anything up. What
remains of them reads the established session and treats its absence as a broken invariant of the generated code
(a debug assertion, as the Go and Kotlin "engine must be set" checks already are). In C++ and Rust the parameters a
parent hands an invoked child are deferred to the child's start, as Go, Kotlin and Swift already defer them, so a
child's session is made by the child.

### 3. A session that cannot be established refuses the start

When establishing fails, the start is refused: no state is entered, no executable content runs, no event is taken,
the inbox is not opened, and what the steps made is released. The host receives the failure, typed, in the idiom of
its language that a caller cannot drop without saying so:

| Backend | Plain start | Failure type |
|---|---|---|
| Rust | `initialize(&mut self) -> Result<(), P::StartFailure>` | `ScriptSessionFailure`; a policy without a script engine has `Infallible`, so its start cannot fail (`.unwrap_or_else(\|never\| match never {})`) |
| C++ | `[[nodiscard]] std::optional<ScriptSessionFailure> initialize()` | `ScriptSessionFailure` |
| Go | `Initialize() error` | `*ScriptSessionFailure` (an `error`) |
| Kotlin | `initialize()` and `start(scope)` throw | `ScriptSessionUnavailableException`; `start` establishes before it launches its coroutine |
| Python | `initialize()` raises | `ScriptSessionUnavailable`, before the run is marked running |
| Swift | `initialize() throws` | `ScriptSessionFailure: Error` |
| C11 | `_init*` returns `<name>_start_status_t` (`warn_unused_result`) | `<NAME>_START_SCRIPT_SESSION_UNAVAILABLE` |

`enterAt` and its spellings already return a `ConfigurationRejection`; it gains the variant `ScriptSessionUnavailable`
carrying the same failure, as C11's rejection already carries `_CONFIG_HOST_ACTIONS_INCOMPLETE` for a construction
that cannot complete.

### 4. The checked root start refuses for either reason

The checked root start (`initialize_as_root` and its spellings, SCE_ERROR_CONTRACT.md §10.1) refuses a machine that
needs a parent before it establishes anything, and a machine whose session cannot be established after. Its refusal
type gains the variant `ScriptSession(ScriptSessionFailure)` beside `NeedsParent`.

### 5. An invoked child that cannot start is a failed invoke

A child whose start is refused is not kept. The parent raises `error.execution` on its internal queue and the
`<invoke>` ends without a child, the path a namelist or `src` that cannot be read already takes (§scxml-6.4); no
`done.invoke` is raised for it. The error's data names the invoke id and the failure's step.

### 6. Held by one scenario on every backend

Each backend's tests gain a script engine double whose creation, library load or bind fails on request, and the same
cases run on all seven: the start is refused with the step named; no `<onentry>` ran and no event was taken; a resumed
start is refused the same way; a child whose session fails leaves its parent with `error.execution` and no child. A
mutation casefile per backend holds the refusal (a mutant that continues past the failure, or marks the session
established, is killed by these cases).

## Consequences

- Every backend's start API changes shape, and every generated tree and every caller of a start moves. The API is
  pre-release, and a start that returned nothing while the machine it started could not work was the defect.
- A host that starts a machine now handles one failure it could not see before. A machine without a script engine
  cannot fail this way; Rust says so in the type, the other backends in the documentation of the start.
- The lazy re-establishment Kotlin and Swift performed after a failure is gone: a session that failed once is not
  retried mid-run, where a retry that succeeded would run a machine whose earlier conditions all read as errors.
- ARCHITECTURE.md's "Lazy Initialization" line is replaced: a session is made once, at the start.

## Considered alternatives

- **Log and continue** (Rust and C++ today): runs a machine with no datamodel and tells only a log. Rejected.
- **Raise `error.execution` or `error.platform` in the machine** (Swift and C11 today, in part): the machine's handling
  of an error runs in the session that failed, so the event reports into the thing that cannot read it. The error
  events of §scxml-5.10 are for errors of the document's content; this one is the platform's, and belongs to the host.
- **Abort or panic**: right for a broken invariant of the generated code (an engine never injected), wrong for an
  allocation or a memory limit, which a host on a constrained device may meet and recover from by starting fewer
  machines.
- **Keep the lazy setup and make it report**: a failure found at the first condition is found in the middle of a
  macrostep, after states were entered and executable content ran; the start is the one point where refusing leaves
  nothing half done.

## Order of landing

Each landing carries a backend's runtime, its template, its regenerated trees, its tests and its casefile, and the
documents it amends where it is the first:

1. Rust (the failure type in the policy, `create_session` with a result, the child's parameters deferred), with
   ARCHITECTURE.md and SCE_ERROR_CONTRACT.md;
2. C++ (the same, and the parameter channel moved into the child's start);
3. Go;
4. Kotlin (`start` establishing before it launches);
5. Python (establishing before the run is marked running);
6. Swift (establishing at the start, not at the first use);
7. C11 (`_init*` returning a status).
