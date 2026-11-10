# SCE TTCN-3 conformance lane — design

- Status: design of ADR 0091 order of work step 5, written 2026-11-10. Nothing in it is built yet. The lane is split
  between SCE and a separate downstream repository, `sce-ttcn3-titan`, which is still to be created. The SCE-side
  slices S1 and S2 come first, then the downstream slices D1 to D5 (§13).
- Decision: `docs/adr/0091-a-test-is-a-document-of-the-test-profile-and-its-ttcn-3-export-runs-and-agrees-with-it.md`,
  answers 1, 3, 8 and 12, §6 (C1 to C3) and §7. The ADR was amended on 2026-11-10 with the repository split and the
  licensing record of §3.
- Semantics: `docs/SCE_TEST_PROFILE.md`, cited as **TP §x**: the scheduler (TP §3), the export (TP §9), what the
  lane requires of the tool (TP §10) and the native runtime (TP §12).
- Sources, pinned by ADR 0091 answer 2: ETSI ES 201 873-1 V4.17.1 (**-1 §x**) and -4 V4.6.1 (**-4 §x**); -5 V4.9.1,
  the TTCN-3 Runtime Interface (TRI), and -6 V4.14.1, the TTCN-3 Control Interface (TCI). The texts of -5 and -6 were
  not at hand when this design was written. Their operations are named here as ADR 0091 and the native runtimes name
  them, which are the names TRI and TCI publish. Slice D1 reads each operation's name and parameters from the two
  texts before it writes a header, and corrects this document where they differ.
- The split of §2, the licensing record of §3, the layout of §6, the Main Controller change of §9.2, the trace of
  §10, the lane of §11 and the slices of §13 were decided by the main session on the owner's delegation of
  2026-11-10. They are recorded here with their reasons, and this document does not reopen them. Where they leave
  something open and this document chooses it, the choice is marked **design choice**, so that it can be told apart
  from what was decided.

## 1. What the lane is for

C1 asks for the export to run on a TTCN-3 tool that SCE did not write. C3 asks that two runs of one suite on one input
trace, the native run and the run of the export, agree per test case in the test verdict and in every decoded
message, and it makes a disagreement `error`. §7 asks for a reference adapter set, so that both can be checked without
any consumer's test system. The lane is that check. It runs every test case of the profile's corpus natively and
through the export on Eclipse Titan, from the same recorded inputs, and compares the canonical logs of the two runs.

The comparison means something only under three conditions:

- **The two runs see the same inputs at the same instants**: the recorded input trace (§10) and the logical clock
  (§9.2).
- **The two runs take the same schedule**: the rounds of TP §3.2, carried in Titan by its Main Controller (§9.2).
- **The TTCN-3 side's codecs and logger are not SCE's**: the codec bindings (§8.4) and the logger plugin (§9.3.3) run
  on libraries that neither SCE nor Titan wrote, in a repository that holds no SCE code (§2).

## 2. Repository boundary

### 2.1 The split

Everything that touches Titan lives in a separate downstream repository, `sce-ttcn3-titan`, published under EPL-2.0:

- the adapter interface shaped after TRI and TCI (C++17, §7);
- the reference adapters: the trace player and recorder, and the logical clock (§8);
- the codec bindings, over libcbor (MIT) and libical (MPL-2.0) (§8.4);
- the Titan binding: test ports, the `enc_`/`dec_` external functions and the logger plugin (§9.3);
- the pinned Titan build and the Main Controller patches, which go upstream under the ECA (§9.1, §9.2, §9.4);
- the lane runner, which applies C3 (§11).

SCE keeps what it owns as a tool and as a specification:

- the exporter (`sce-codegen export-ttcn3`) and the validator (`sce-ttcn3-validate`);
- the wire surfaces the downstream reads and writes, with their schemas and their rows of `SCE_WIRE_CONTRACTS.md`
  (§4);
- the native runtimes and their runners, with trace replay (§10.5);
- the corpus, with its recorded traces and its expected logs (§12).

SCE holds no directory of the downstream's code, and no copy of it.

### 2.2 Why

- **The tool and its adapters belong together.** A test port is written to Titan's C++ Test Port API and compiled and
  linked with Titan's runtime, and the Main Controller patches are changes to Titan. They are kept, built and
  released with the Titan they are written for.
- **SCE is used, not combined.** The downstream runs SCE as a program and reads its published files, as any user of
  the generator does. No SCE file is part of the downstream, and no downstream file is part of SCE. The licensing
  question of combining SCE's code with Titan's therefore does not arise (§3).
- **The interface between the two is SCE's published surfaces.** These are versioned and their stability is stated
  in one registry (§4), so the downstream depends on a contract and not on SCE's internals. A second TTCN-3 tool, or a
  consumer's own harness, can depend on the same contract.

### 2.3 What crosses the boundary

| Artefact | Written by | Read by |
|---|---|---|
| `S.ttcn`, `S_Types.ttcn` | `sce-codegen export-ttcn3` (TP §9.5) | Titan's compiler, in the downstream |
| `S_Types.manifest.json` | the export | the downstream's generator (§9.3), value model (§8.3) and logger plugin |
| `S.witness.json` | the export | `sce-ttcn3-validate`, which the downstream runs as a process |
| a trace, `sce-test-trace.v1` (§10) | the corpus (SCE); the recorder (downstream, §8.1) | the native runners (SCE); the trace player (downstream) |
| a canonical log, `sce-test-log.v1` (ADR 0091 answer 8) | the native runners (SCE); the logger plugin (downstream) | the lane's comparison (downstream); SCE's corpus check (§12) |
| a codec document (`sce:kind="codec"`, `SCE_FORGE.md` §4.6) | the corpus and consumers | the downstream's codec bindings, for the format's parameters only (§8.4) |
| `sce-codegen`, `sce-ttcn3-validate`, each backend's runner (§10.5) | SCE | the downstream's lane, run as processes |

The downstream pins the SCE revision it runs: a commit of SCE, from which it builds `sce-codegen`, the validator and
the native runners (design choice: in a file `SCE_PIN` at its root). When SCE's CI runs the lane (§11.5), it runs the
downstream at a pinned revision of the downstream against SCE's own checkout, which then stands in for `SCE_PIN`.

### 2.4 How the boundary is kept

- **In the downstream.** Its slice D1 adds a check, run in its own CI, that every file carries
  `SPDX-License-Identifier: EPL-2.0` (but `LICENSE`, the verbatim text), and that no file includes, imports, links or
  copies a path of SCE. SCE is reached only through the pinned revision's programs and published files.
- **In SCE.** No build file, gate or source of SCE names a file of the downstream. SCE's CI fetches the downstream at
  a pinned revision and runs it as an external tool (§11.5), in the same way it fetches and runs mnemosyne-cli.

## 3. Licensing

This is the record ADR 0091 answer 1 carries, amended 2026-11-10. Each statement gives its basis.

- **The split removes the combination question.** No SCE file is relicensed. The downstream holds no SCE code and is
  a separate work: it runs SCE as a program and reads and writes SCE's published files (§2.3). Before the split,
  EPL-2.0 code (Titan's and a binding linked with it) and SCE's AGPL-3.0 code would have been combined in one
  executable. EPL-2.0 allows that only where the EPL file's notice names a GPL-family licence as a Secondary License,
  and Titan's notices name none (answer 1). After the split no such executable exists.
- **The exported modules are generated output.** The `.ttcn` modules are text `sce-codegen` writes from the suite's
  documents, as it writes any target language. They are governed by `LICENSE-GENERATED.md`, as every output of the
  generator is, and the downstream compiles them as any user of the generator compiles its output.
- **CI-only use distributes nothing.** EPL-2.0's obligations arise when the program is distributed. AGPL-3.0's arise
  when it is conveyed, or when users interact with a modified version over a network. Running Titan and the
  downstream in CI, or on a developer's machine, is neither: they are run, not handed to anyone. No SCE product
  contains them (answer 1).
- **The upstream patch is contributed under the ECA.** A contribution to an Eclipse project, here `eclipse/titan.core`,
  is made under the project's licence, EPL-2.0, by a contributor who has signed the Eclipse Contributor Agreement. The
  patches are EPL-2.0 from their first line, in a repository under EPL-2.0, so they need no relicensing on the way
  upstream. The signer is the patches' author.
- **The codec libraries keep their own licences.** libcbor is MIT. libical is offered under MPL-2.0 or LGPL-2.1, and
  the downstream takes MPL-2.0. Both are linked into the downstream's executable test suite with EPL-2.0 code. MIT is
  permissive, and MPL-2.0 is a file-level copyleft that stays on libical's own files. The downstream pins each library
  by version and by the SHA-256 of its source archive, and carries each licence text with the pin.
- **No legal review was made.** None is needed for the combination question, which no longer exists. The statements
  above are standard facts of the licences named, each with its basis. A consumer that distributes a binary built
  from the downstream takes on EPL-2.0's obligations for it, and those of MIT and MPL-2.0 for the libraries, as for
  any EPL-2.0 program.

`LICENSE-THIRD-PARTY.md` records, in its section "The TTCN-3 conformance lane", that the lane is a separate
repository, that nothing of it is vendored into SCE, and that SCE's CI runs it as a tool.

## 4. What SCE provides

The downstream depends on these SCE surfaces and on nothing else of SCE. Each has its row in `SCE_WIRE_CONTRACTS.md`,
which states its stability and how a change is announced.

| Surface | Schema | Status | Row today |
|---|---|---|---|
| the exported modules `S.ttcn` and `S_Types.ttcn`, under the name contract of TP §9.5 | the TTCN-3 the export writes (TP §9.5) | as the export's manifest and witness | the manifest's and the witness's rows describe them |
| the types manifest `S_Types.manifest.json` | `schemas/sce-ttcn3-types-manifest.v1.schema.json` | `pre-release` | yes |
| the correspondence witness `S.witness.json` | `schemas/sce-ttcn3-witness.v1.schema.json` | `pre-release` | yes |
| the recorded input trace `sce-test-trace.v1` | `schemas/sce-test-trace.v1.schema.json` (S1) | `pre-release` | planned (§10.4) |
| the canonical log `sce-test-log.v1` | `schemas/sce-test-log.v1.schema.json` (order of work step 6) | `pre-release` | planned (ADR 0091 answer 8) |
| the authoring grammar, of which the codec bindings read codec documents | `schemas/sce-forge.xsd`, `schemas/sce-forge-ext.xsd` | `pre-release` | yes |

Beside the files, the downstream runs SCE's programs. Their command lines are part of the contract, stated here and
in each program's usage. Design choice: a change to one is announced as a change to the surface it writes.

| Program | Contract |
|---|---|
| `sce-codegen export-ttcn3` | writes the two modules, the manifest and the witness of one suite, all or nothing (TP §9.5) |
| `sce-ttcn3-validate <suite.scxml> <export directory>` | exit 0 with no violation, 1 with one or more, 2 when an input is not read (TP §9.4) |
| `sce-codegen check-trace <suite> <trace>` (S1, design choice) | refuses a trace that does not fit the suite (§10.2) |
| each backend's runner (S1) | runs one test case of a generated suite against one trace and writes its result, the canonical log once step 6 lands (§10.5) |

While every surface above is `pre-release`, SCE may change one in a way that is not additive, under the policy of
`SCE_WIRE_CONTRACTS.md`. The downstream's `SCE_PIN` is what keeps it working across such a change: it moves to a new
SCE revision only together with its own adaptation.

## 5. What the downstream provides to SCE

The downstream provides one program to SCE's CI: the lane runner. Given an SCE checkout, a list of suites and the
directories of their traces, it runs §11's steps and writes a report. A test case whose two logs disagree, or whose
run failed on either side, is `error`, and the runner exits non-zero. SCE does not read any other downstream file.

## 6. Layout of `sce-ttcn3-titan`

The four directories under the root are decided. The names below them are a design choice.

```text
sce-ttcn3-titan/
  LICENSE                       EPL-2.0, verbatim
  README.md                     what the repository holds, its licence and its boundary (§2.4)
  SCE_PIN                       the SCE revision the lane runs by default (§2.3)
  adapter/                      the interface (§7): C++17 headers, no implementation
    include/sce_ttcn3/
      tri_types.hpp             TRI's identifiers, status and message; the logical instant
      tri_sa.hpp                TRI system adapter: TriCommunicationSA, TriCommunicationTE
      tri_pa.hpp                TRI platform adapter: TriPlatformPA, TriPlatformTE
      value.hpp                 the canonical value (§10.3) that the codec and logging operations carry
      tci_cd.hpp                TCI codec operations
      tci_tl.hpp                TCI test logging operations
      tci_tm.hpp                TCI test management operations, the lane's driver side
  reference/                    the reference adapter set (§8)
    sa/                         the system adapter: trace player and recorder
    pa/                         the platform adapter: the logical clock
    value/                      canonical JSON of values (JCS) and the manifest's type table
    log/                        the writer of the canonical log, behind the test logging operations
    codec/cbor/                 the CBOR binding, over libcbor
    codec/icalendar/            the iCalendar binding, over libical
    third_party.lock            each library's version, source URL and SHA-256 of its source archive
  titan/                        the Titan binding (§9)
    PIN                         Titan's release tag, commit SHA and SHA-256 of its source archive
    install.sh                  builds the pinned Titan from source, with patches/ applied in order
    patches/                    the Main Controller change (§9.2), as a series of patches
    tests/                      the patches' own TTCN-3 tests (§9.2.6)
    generate/                   writes a suite's test ports and external functions from its manifest
    testport/                   the part of a test port that is the same for every port type
    logger/                     the logger plugin that writes the canonical log (§9.3.3)
  lane/                         the lane runner (§11)
    run.sh                      one suite, natively and on Titan, from the same traces, compared
    compare/                    C3: the comparison of two canonical logs
    check-boundary.sh           the boundary of §2.4
    suites/                     one entry per suite: its codec table (§8.4)
```

## 7. The adapter interface (`adapter/`)

### 7.1 Why it is the standard's operations

Titan implements neither TRI nor TCI. Its adapters are test ports written to its own Test Port API, and its
Programmers' Technical Reference cites parts 1, 4, 7 and 9 of ES 201 873, not parts 5 and 6 (ADR 0091 answer 1).
Writing the adapters to Titan's API would make "the suite runs elsewhere" mean "the suite runs on Titan". So the
adapters are written to an interface whose operations are TRI's and TCI's, one for one and under their names, and a
tool is reached through a binding of that interface. On a tool that implements TRI and TCI natively, the binding is a
renaming of types (answer 1).

### 7.2 The operations

| Interface | Direction | Operations |
|---|---|---|
| TRI system adapter (-5) | test executable to adapter | `triSAReset`, `triExecuteTestCase`, `triMap`, `triUnmap`, `triEndTestCase`, `triSend` |
| | adapter to test executable | `triEnqueueMsg` |
| TRI platform adapter (-5) | test executable to adapter | `triPAReset`, `triStartTimer`, `triStopTimer`, `triReadTimer`, `triTimerRunning` |
| | adapter to test executable | `triTimeout` |
| TCI codec (-6) | test executable to codec | encode, decode |
| TCI test logging (-6) | test executable to logger | one operation per record kind of ADR 0091 answer 8 (below) |
| TCI test management (-6) | lane to test executable, and back | `tciStartTestCase`, `tciStartControl`, `tciTestCaseStarted`, `tciTestCaseTerminated`, `tciControlTerminated`, `tciGetModulePar` |

TRI's procedure-based operations (`triCall` and the rest) are left out, because the profile has no procedure-based
ports (TP §8.1). So are `triSendBC` and `triSendMC`, because a port of the profile is connected or mapped to one other
port only (TP §2.2), and `triMapParam` and `triUnmapParam`, because a profile `map` takes no parameters.

Answer 1 names TCI's codec and test control operations among those the interface keeps, beside TRI's and TCI's test
logging, so both are here. The codec bindings of §8.4 provide the codec operations. The test management operations are
the lane's side of a run: the lane starts a test case or the control part with the run's module parameters, and is
told when it ends and with which verdict. Titan's binding implements them by writing Titan's configuration file and
running the executable test suite (§11.1).

The test logging operations, one per record kind of answer 8 (design choice: `tliCtKill` is added, because removing a
PTC is a step of P9 that adds the PTC's local verdict to `TC-VERDICT`, and the native log records it):

| Record kind (answer 8) | Operation |
|---|---|
| test case started | `tliTcStart` |
| test case ended, with its test verdict | `tliTcTerminated` |
| component created | `tliCtCreate` |
| component done | `tliCtTerminated` |
| component killed | `tliCtKill` |
| message sent | `tliMSend_m` on a mapped port, `tliMSend_c` on a connected one |
| message received and matched | `tliMReceive_m`, `tliMReceive_c` |
| timer started, stopped, timed out | `tliTStart`, `tliTStop`, `tliTTimeout` |
| verdict set | `tliSetVerdict` |
| log | `tliLog` |

Where -6 has two operations close to one kind, as `tliTcStop` and `tliTcTerminated` are to "test case ended", slice D1
takes the one whose definition in -6 V4.14.1 is the kind's, and corrects this table if it differs.

### 7.3 Where the interface departs from TRI and TCI

Each departure keeps an operation's meaning and changes only how a value is held:

- **Time is integer microseconds.** TRI's timer duration is a float of seconds. The profile's `NOW` and every duration
  are whole microseconds (TP §3.3, §4.7), so `Instant` and `Duration` are signed 64-bit integers. A binding to a tool
  that hands floats converts each duration once, by the rule of §9.2.5.
- **The SUT address is always empty.** TRI's `triSend` and `triEnqueueMsg` carry an address of the SUT. The profile
  has no `to` and no `from` clause (TP §2.2, §8.1), so the parameter is kept, to keep the operation whole, and is
  always empty.
- **Values are one concrete model.** TCI's codec and logging operations carry TCI's abstract values and types. Here
  they carry the canonical value of §10.3, typed by the suite's types manifest. Every binding converts its tool's
  values to and from that model, so the codecs and the log writer are written once.
- **A log operation's timestamp is the instant and the round.** -6 leaves the timestamp's form to the tool. Here it is
  the pair of `NOW` and the round the record was made in, which the canonical order of §11.3 sorts by. -6's
  parameters that name a place in a tool's source (`am`, `src`, `line`) are left out: they differ between the two
  sides by construction, and nothing compares them.
- **The language mapping is the downstream's.** The headers are a C++17 mapping in the namespace `sce_ttcn3`, not the
  C++ mappings of -5 and -6. They keep the operations' names, so a binding to a tool that implements a standard
  mapping renames types and does nothing else.

### 7.4 Sketches

The headers of slice D1, as signatures. Each file opens with `// SPDX-License-Identifier: EPL-2.0`.

```cpp
// tri_types.hpp
namespace sce_ttcn3 {

/// A logical instant: microseconds since the test case started (TP §3.3).
using Instant = std::int64_t;
/// A duration of whole microseconds (TP §4.7).
using Duration = std::int64_t;

/// TRI's TriStatus.
enum class TriStatus { ok, error };

/// TRI's TriComponentId: the component's name in the test system (TP §9.5, E11) and its type.
struct TriComponentId {
    std::string name;
    std::string type;
};

/// TRI's TriPortId: a component's port, or a port of the test system interface, whose component is `system`.
struct TriPortId {
    std::string component;
    std::string port;
};

/// TRI's TriTimerId.
struct TriTimerId {
    std::string component;
    std::string timer;
};

/// TRI's TriMessage: the encoded octets of one message.
using TriMessage = std::vector<std::uint8_t>;

/// TRI's TriAddress. The profile names no address of the SUT (TP §2.2), so it is always empty.
struct TriAddress {};

}  // namespace sce_ttcn3
```

```cpp
// tri_sa.hpp
namespace sce_ttcn3 {

/// The system adapter's operations, called by the test executable.
class TriCommunicationSA {
public:
    virtual ~TriCommunicationSA() = default;
    virtual TriStatus triSAReset() = 0;
    virtual TriStatus triExecuteTestCase(const std::string &testCase, const std::vector<TriPortId> &tsiPorts) = 0;
    virtual TriStatus triMap(const TriPortId &componentPort, const TriPortId &tsiPort) = 0;
    virtual TriStatus triUnmap(const TriPortId &componentPort, const TriPortId &tsiPort) = 0;
    virtual TriStatus triEndTestCase() = 0;
    virtual TriStatus triSend(const TriComponentId &component, const TriPortId &tsiPort,
                              const std::optional<TriAddress> &sutAddress, const TriMessage &message) = 0;
};

/// The test executable's operation, called by the system adapter.
class TriCommunicationTE {
public:
    virtual ~TriCommunicationTE() = default;
    virtual void triEnqueueMsg(const TriPortId &tsiPort, const std::optional<TriAddress> &sutAddress,
                               const TriComponentId &component, const TriMessage &message) = 0;
};

}  // namespace sce_ttcn3
```

```cpp
// tri_pa.hpp
namespace sce_ttcn3 {

/// The platform adapter's operations, called by the test executable.
class TriPlatformPA {
public:
    virtual ~TriPlatformPA() = default;
    virtual TriStatus triPAReset() = 0;
    virtual TriStatus triStartTimer(const TriTimerId &timer, Duration duration) = 0;
    virtual TriStatus triStopTimer(const TriTimerId &timer) = 0;
    virtual TriStatus triReadTimer(const TriTimerId &timer, Duration &elapsed) = 0;
    virtual TriStatus triTimerRunning(const TriTimerId &timer, bool &running) = 0;
};

/// The test executable's operation, called by the platform adapter.
class TriPlatformTE {
public:
    virtual ~TriPlatformTE() = default;
    virtual void triTimeout(const TriTimerId &timer) = 0;
};

}  // namespace sce_ttcn3
```

```cpp
// value.hpp
namespace sce_ttcn3 {

/// A value of a type of the suite's types module, in the canonical form of §10.3. Its type is named apart, by the
/// type's name in the manifest, so the same model holds a value of every type.
struct Value {
    enum class Kind { integer, boolean, enumerated, text, octets, record, list, alternative };

    Kind kind = Kind::integer;
    bool negative = false;             // integer: the sign
    std::uint64_t magnitude = 0;       // integer: the absolute value, so int64 and uint64 both fit
    bool truth = false;                // boolean
    std::string name;                  // enumerated: the variant; alternative: the union's case
    std::string text;                  // text: UTF-8
    std::vector<std::uint8_t> octets;  // octets
    std::vector<std::string> fields;   // record: the names of the fields present, in declared order
    std::vector<Value> elements;       // record: the fields' values; list: the elements; alternative: one value
};

}  // namespace sce_ttcn3
```

```cpp
// tci_cd.hpp
namespace sce_ttcn3 {

/// TCI's codec operations, provided by a codec binding.
class TciCDProvided {
public:
    virtual ~TciCDProvided() = default;
    /// The octets of @p value, a value of the type the manifest names @p type.
    virtual TriStatus encode(const std::string &type, const Value &value, TriMessage &octets) = 0;
    /// Decodes a value of @p type from the front of @p octets and removes the octets it took. Returns what -1's
    /// `decvalue` returns: 0 decoded, 1 not decoded, 2 the octets end before the message does (TP §9.5).
    virtual int decode(const std::string &type, TriMessage &octets, Value &value) = 0;
};

}  // namespace sce_ttcn3
```

```cpp
// tci_tl.hpp
namespace sce_ttcn3 {

enum class Verdict { none, pass, inconc, fail, error };

/// The timestamp of every logging operation: the logical instant, and the round it was made in (§11.3).
struct Stamp {
    Instant now = 0;
    std::uint64_t round = 0;
};

/// TCI's test logging operations, provided by the log writer.
class TciTLProvided {
public:
    virtual ~TciTLProvided() = default;
    virtual void tliTcStart(const Stamp &ts, const std::string &testCase) = 0;
    virtual void tliTcTerminated(const Stamp &ts, const std::string &testCase, Verdict verdict) = 0;
    virtual void tliCtCreate(const Stamp &ts, const TriComponentId &creator, const TriComponentId &created) = 0;
    virtual void tliCtTerminated(const Stamp &ts, const TriComponentId &component, Verdict local) = 0;
    virtual void tliCtKill(const Stamp &ts, const TriComponentId &killer, const TriComponentId &killed) = 0;
    virtual void tliMSend_m(const Stamp &ts, const TriPortId &port, const TriPortId &tsiPort,
                            const std::string &type, const Value &value) = 0;
    virtual void tliMSend_c(const Stamp &ts, const TriPortId &port, const TriPortId &peer,
                            const std::string &type, const Value &value) = 0;
    virtual void tliMReceive_m(const Stamp &ts, const TriPortId &port, const TriPortId &tsiPort,
                               const std::string &type, const Value &value) = 0;
    virtual void tliMReceive_c(const Stamp &ts, const TriPortId &port, const TriPortId &peer,
                               const std::string &type, const Value &value) = 0;
    virtual void tliTStart(const Stamp &ts, const TriTimerId &timer, Duration duration) = 0;
    virtual void tliTStop(const Stamp &ts, const TriTimerId &timer) = 0;
    virtual void tliTTimeout(const Stamp &ts, const TriTimerId &timer) = 0;
    virtual void tliSetVerdict(const Stamp &ts, const TriComponentId &component, Verdict verdict,
                               const std::string &reason) = 0;
    virtual void tliLog(const Stamp &ts, const TriComponentId &component, const std::string &text) = 0;
};

}  // namespace sce_ttcn3
```

```cpp
// tci_tm.hpp
namespace sce_ttcn3 {

/// TCI's test management operations the test executable provides, called by the lane.
class TciTMProvided {
public:
    virtual ~TciTMProvided() = default;
    virtual void tciStartTestCase(const std::string &testCase) = 0;
    virtual void tciStartControl() = 0;
};

/// TCI's test management operations the lane provides, called by the test executable.
class TciTMRequired {
public:
    virtual ~TciTMRequired() = default;
    virtual void tciTestCaseStarted(const std::string &testCase) = 0;
    virtual void tciTestCaseTerminated(const std::string &testCase, Verdict verdict) = 0;
    virtual void tciControlTerminated() = 0;
    /// The value of the module parameter @p name in this run (the suite's own, or a feature's `PICS_<NAME>`,
    /// TP §9.5 E15), or none, where the module's default applies.
    virtual std::optional<Value> tciGetModulePar(const std::string &name) = 0;
};

}  // namespace sce_ttcn3
```

## 8. The reference adapter set (`reference/`)

### 8.1 The system adapter: trace player and recorder

- **The player** implements `TriCommunicationSA`. At `triExecuteTestCase` it reads the trace the lane names for the
  test case (§10). It tells the clock the instant of its next input, as the native runtime's system adapter does
  (`nextInput`, TP §12.2). When `NOW` reaches an input's instant, it hands each input due then to `triEnqueueMsg`, in
  trace order, encoded by the codec binding of the input's type from its canonical value. A `triSend` is the test
  system's output. The player checks nothing about it, because the canonical log records every send.
- **The recorder** is the same adapter run in front of a live SUT. It writes each message it hands to `triEnqueueMsg`,
  decoded by the codec binding into its canonical value, as a trace record at the instant it was enqueued. A run
  against a real implementation can therefore be replayed, on both sides, as many times as needed.

### 8.2 The platform adapter: the logical clock

`NOW` and every timer are integers of microseconds (TP §3.3). `NOW` changes only when the run's driver advances it,
to the least of the next timer expiry, the next input instant and the guard instant, and never by itself. The adapter
reports its next expiry, and a timer whose expiry is at or before `NOW` is timed out.

Titan keeps timers inside its runtime and has no platform adapter to bind (ADR 0091 answer 1), so on Titan the same
arithmetic is in the patched runtime (§9.2.5). The reference clock is then the definition the patch's tests are held
to (§9.2.6), and the clock a tool that delegates timers through TRI runs on.

### 8.3 Values

One implementation reads and writes canonical values as JCS text (RFC 8785, the JSON Canonicalization Scheme), typed
by the suite's types manifest: the trace player and the recorder, the codec bindings, the log writer and the logger
plugin all use it. A value that does not belong to its type (a field the record does not declare, an integer outside
its width, a list longer than its bound) is refused where it is read, and a refusal during a run is an adapter
failure, so `error` (P13).

### 8.4 The codec bindings

C2 asks that the TTCN-3 side's codec be independent of SCE's codec generator. Titan's own encoding engines cannot
write either format SCE generates codecs for: a CBOR map with integer keys and byte strings (`SCE_FORGE.md` §4.6.1),
and one RFC 5545 component (`SCE_FORGE.md` §4.6.4) (ADR 0091 answer 1). The codec bindings therefore implement
`TciCDProvided` on two libraries written by neither SCE nor Titan:

| Format | Library | Licence |
|---|---|---|
| CBOR (RFC 8949) | libcbor | MIT |
| iCalendar content lines (RFC 5545) | libical | MPL-2.0 |

Each library is pinned with its version and the SHA-256 of its source archive in `reference/third_party.lock`, and the
lane builds it from source, as it builds Titan (§9.1), so that a host needs no distribution package of either.

**Where a format's parameters come from (design choice).** The lane entry of each suite (`lane/suites/`) names, for
each message type a mapped port carries, the SCE codec document that encodes it. The binding reads from that document
only the format's parameters: for CBOR, each field's key and how its value is written; for content-line, the component
and each field's property. It reads no generated code. A codec document that uses a feature the binding does not
implement is refused by name before any run. This is the first kind of independence of C2, an independent encoding
engine, which is what answer 1 claims for the lane. A consumer who wants the second kind, an independent reading of
the protocol, replaces the entry with tables written from the protocol's own specification.

Positional binary codecs, the default layout of `SCE_FORGE.md` §4.6, have no reference binding in this design. Answer
1 names CBOR and content-line as the formats Titan cannot write. A suite whose mapped types are encoded by a
positional codec cannot run in the lane until a binding for it is added (§14).

## 9. The Titan side (`titan/`)

### 9.1 The pinned build

The lane runs Titan 11.2.1, the release ADR 0091's probe built from source and ran. `titan/PIN` records its release
tag in `eclipse/titan.core`, the commit SHA of that tag, and the SHA-256 of the source archive. Slice D2 reads the
three from the upstream repository and writes them. `titan/install.sh` downloads that archive, checks its digest,
applies `titan/patches/` in order and builds Titan into the prefix it is given. The packages the build needs are those
the pinned release's own build instructions name, read from that release in slice D2.

### 9.2 The Main Controller change

#### 9.2.1 One protocol for rounds and the logical clock

Titan by itself follows neither TP §3.2 nor TP §3.3 (TP §10). Each of its components is a process that takes a
snapshot whenever it waits, and its time is the wall clock. The change makes Titan's Main Controller carry both, in
one protocol, because both turn on one condition. A round may start only at quiescence. Time may advance only at
quiescence, and only when the round before fired no branch. So one decision at quiescence either grants a round at
the present instant, or advances time and then grants one, or ends the test case.

#### 9.2.2 What a component tells the Main Controller

A component is **waiting** when it has completed a macrostep and waits in an `alt` (TP §3.2). When it starts to wait,
it reports:

- `WAITING(epoch, sent, received, due, fired)`:
  - `epoch`, the epoch the report belongs to (§9.2.3);
  - `sent` and `received`, its counts in this epoch of port messages it sent and received on connected ports. A
    message counts as received when it is in the receiving port's queue;
  - `due`, the least instant it is due at: the expiry of each of its running timers, the next input instant of each
    of its mapped test ports (the trace player's next handler timeout), and, for the MTC, the guard instant (P11);
  - `fired`, whether the evaluation it reports took a branch.

A waiting component that receives a port message reports again, with its new `received`. A component that reaches
its end reports `TERMINATED(epoch)`, through Titan's existing path for a component that is done.

#### 9.2.3 The states of the Main Controller in a test case

Each grant of a snapshot starts a new **epoch**. A report from an earlier epoch does not count. The test case starts
in epoch 0, with the MTC running its initial macrostep.

1. **Collect (epoch e).** The Main Controller waits until **quiescence**:
   - every component that is present has reported `WAITING` in epoch e, or has terminated;
   - the sums of `sent` and of `received` over the latest report of each component are equal;
   - no create, connect, map, start, kill or done that the Main Controller itself carries is pending.
2. **Decide.**
   - If the MTC has terminated, the test case ends as Titan ends it today: every PTC still present is removed, and
     the test case verdict is computed (P10, P14).
   - Otherwise, if epoch e is 0 or some report of epoch e says `fired`, the next round is granted at the present
     `NOW`.
   - Otherwise no branch fired, and `NOW` advances to the least `due` reported. When no component reports a `due`,
     there is no instant to advance to and the test case can never end: the Main Controller ends it with `error`
     (P13).
3. **Snapshot (epoch e + 1).** The Main Controller sends `SNAPSHOT(e + 1, NOW)` to every waiting component. Each one
   sets its clock to `NOW`, then:
   - for the MTC, if `NOW` has reached the guard instant, ends the test case with `error` before any snapshot at that
     instant (P11; the native runtime ends it in the same place, TP §12.4);
   - runs the handlers of its test ports that are due at `NOW`, so the trace player enqueues every input due then,
     in trace order;
   - sets each running timer whose expiry is at or before `NOW` to timed out;
   - takes its snapshot (-4 §9.3.1), and replies `SNAPPED(e + 1)`.

   The Main Controller waits for every reply. This is TP §3.2 steps 2 and 3: every input and timer expiry due at
   `NOW` is delivered before any snapshot, and every waiting component takes its snapshot at once.
4. **Evaluate (epoch e + 1).** For each component in the order TP §3.2 step 4 fixes, the MTC first and then each PTC
   in the order of its creation, that took a snapshot in step 3 and is still present and running, the Main Controller
   sends `EVALUATE(e + 1)`. The component evaluates its branches over its own snapshot (P3). If one is selected, it
   runs the branch's block to its next wait or to its end (P4). It then reports `WAITING(e + 1, …)` or `TERMINATED`.
   The Main Controller grants the next evaluation only after that report, after each operation the component asked of
   it, and after each PTC the component started in this evaluation has reported its first `WAITING` or has
   terminated. A component created in this round took no snapshot in it and does not evaluate. A component removed or
   terminated earlier in the round is skipped (TP §3.2 step 5).
5. After the last evaluation, the Main Controller returns to **Collect** in epoch e + 1. Collect then waits only for
   the messages still in transit.

**Why one wave of counts is enough.** Mattern's counting methods need a second wave because a process that has
reported itself passive can be made active again by a message that reaches it afterwards. Under this protocol a
message never makes a component active: only a grant does. In epoch e only the component being evaluated, and the
PTCs it started, can send. Once each of them has reported, the sums of `sent` are final, the sums of `received` can
only grow towards them, and equal sums mean every message of the epoch is in its queue. This is the conservative
synchronisation of Chandy and Misra (IEEE TSE 5(5), 1979) with termination detected by counting (Mattern,
Distributed Computing 2(3), 1987), as answer 3 states.

**Why a started PTC finishes its first macrostep inside its creator's turn.** The native runtime starts a PTC inside
its creator's macrostep (TP §12.4). Waiting for the PTC's first report before the next evaluation gives each
evaluation the same effect on both sides, whatever the order in which the processes are scheduled.

#### 9.2.4 Time

Titan reads the time that decides behaviour at two functions (ADR 0091 answer 3): `TTCN_Snapshot::time_now()`
(`core/Snapshot.cc`), which the timers, the test case's guard, the snapshot wait and the test ports' timed handlers
use, and `TTCN_Runtime::now()` (`core/Runtime.cc`), the TTCN-3 `now` operation. Under the change both return the
`NOW` the Main Controller granted, which is an integer of microseconds. Every timer holds its start instant and its
duration as integers, and its expiry is their integer sum. The other reads of the clock (log timestamps, the profiler,
the debugger) decide nothing and are left as they are.

#### 9.2.5 Durations

The generated code hands Titan a timer's duration as a `float` of seconds. The export writes it as the exact decimal
of a whole number of microseconds (TP §9.5). The patched runtime converts it once, when the timer starts, to the
nearest integer of `seconds * 1000000`. For every duration below 2^50 microseconds (about 35 years) that integer is
the duration the document wrote. The `double` nearest the decimal, and the product, are each within a relative 2^-53
of the exact value, so the result is less than 0.5 from the right integer. The sum of a start instant and a duration
is then never computed in `double`. In `double` arithmetic it could differ in its last bit from the instant a trace
gives for the same moment, and order a timeout and an input differently on the two sides (TP §10).

#### 9.2.6 What the change leaves alone, and its tests

The change is opt-in, by an option of the Main Controller's configuration (design choice). Without the option, Titan
behaves as it does today: wall-clock time, and components that take snapshots whenever they wait. The option's name
is agreed with upstream in the merge request (§9.4). The lane runs every suite in Titan's parallel mode, the mode a
suite with PTCs runs in, so that one code path serves every suite.

The patches carry their own tests, in `titan/tests/`, as TTCN-3 suites written by hand. They are also the tests the
merge request offers upstream:

- a round with an MTC and two PTCs, whose result under any other order of the evaluations differs. The case is TP
  §3.2's: components that remove a component the other sends to;
- a timeout and an input due at one instant fall in the same snapshot, and document order decides which branch is
  taken (answer 3);
- no branch and no instant ends the test case with `error`;
- an input or a timer due at the guard instant does not run: the test case ends with `error` first;
- a timer of 0.1 s started at 0.2 s times out at the same instant as an input at 300000 microseconds, which `double`
  arithmetic would order apart;
- a message still in transit when its sender reports does not let a round start (the counts);
- a PTC created in a round takes no snapshot in it;
- each suite runs twice, and its two logs are the same bytes.

### 9.3 The binding

#### 9.3.1 Test ports

For each port type of a suite that is mapped to the test system, there is one Titan test port. Its skeleton is the
one Titan's compiler writes for the port type. `titan/generate` writes its bodies from the suite's manifest, over the
code that is the same for every port (`titan/testport/`):

- **outgoing**: the message the component sends is encoded by the suite's `enc_<R>` (§9.3.2) and handed to the system
  adapter's `triSend`;
- **incoming**: an input the system adapter hands to `triEnqueueMsg` is decoded by the suite's `dec_<R>` and enqueued
  on the component port mapped to that system port;
- **the trace player's instants** are the test port's handler timeouts (answer 3). The next one is part of the
  component's `due` (§9.2.2), and the handlers due at a granted `NOW` run before the snapshot (§9.2.3 step 3).

A `connect` between components is Titan's own connection between component processes, and does not pass through the
binding (answer 1).

#### 9.3.2 External functions

The types module declares `enc_<R>` and `dec_<R>` for each type a port of the suite carries, and does not implement
them (TP §9.5). `titan/generate` implements them from the manifest, for each type: the conversion of Titan's value of
that type to and from the canonical value (§8.3), then the codec binding's encode or decode (§8.4). `dec_<R>` returns
-1's `decvalue` results (0, 1, 2), as the export declares. A decode that fails during a run is a codec failure, so
`error` (P13).

#### 9.3.3 The logger plugin

The test logging operations are a Titan logger plugin, Titan's interface for a logger a user writes (answer 8). The
plugin receives Titan's log events for ports, timers, verdicts, components, test cases and user `log` statements. For
each event of a record kind of answer 8, it calls the operation of §7.2 on the log writer of `reference/log/`. The
stamp of each record holds the granted `NOW` and the epoch of the round (§9.2.3).

Titan passes a message's value to the plugin as text in TTCN-3 value notation (-1 §6.2), not as a structured value.
The plugin parses that text against the message type's entry in the manifest, into the canonical value. Slice D4
first records which fields Titan 11.2.1 passes with each event, as answer 8's probe recorded the value text. Where an
event does not name the message's type, the port type's message list gives it. A port type with two message types
whose texts the parser cannot tell apart is refused by the lane by name, before any run.

Each component process writes its own records. The lane merges them by the canonical order of §11.3, never by the
order in which processes happened to write.

### 9.4 Upstream

The change is offered to Eclipse Titan, so that the logical clock and the rounds are a property of the tool and not
of one copy of it (answer 3): an issue and a merge request to `eclipse/titan.core`, under the ECA, carrying the
patches of `titan/patches/` and the tests of §9.2.6. Until a Titan release contains the change, the lane builds the
pinned release with `patches/` applied. When a release contains it, `PIN` moves to that release, `patches/` is
emptied, and the tests stay.

## 10. The recorded input trace

### 10.1 The record

A trace is the inputs of one run of one test case from the SUT, each at its logical instant. It is a new SCE wire
surface, `sce-test-trace.v1`, `pre-release`. Each record holds:

| Field | Holds |
|---|---|
| `at` | the instant, in microseconds since the test case started, as a decimal string: digits only, no leading zero |
| `port` | the system port it arrives on: the `id` of an `<sce:system-port>` of the test case's configuration |
| `type` | the message's type: the record's name, as the types module and the manifest name it (TP §9.5, the name contract) |
| `value` | the message, in the canonical value form (§10.3) |

`at` is a string because the manifest writes every value that may pass 2^53 as the text of its digits, and an
instant is a 64-bit integer.

### 10.2 The file

A trace file holds one test case's inputs as one JCS text, so that a trace has exactly one spelling and two traces
are equal exactly when their bytes are. Design choice: the envelope is
`{"v": 1, "suite": <suite>, "test_case": <test case>, "inputs": [<record>, …]}`. The inputs are in non-decreasing
`at`, and the inputs of one port at one instant are in the order they are enqueued (TP §3.2 step 2). A test case may
have several traces. Design choice: they live beside the suite's document, as
`traces/<test case>/<name>.trace.json`, with the expected log of each beside it (§12).

A trace is refused before the run, on both sides, when:

- a record's `port` is not a system port of the test case's configuration;
- a record's `type` is not in the `in` list of that port's type;
- a record's `value` does not belong to its type;
- the records are not in the order above.

The native runners and the Titan system adapter both read the same file. So the two runs see the same inputs at the
same instants by construction, which is what C3's comparison assumes.

### 10.3 The canonical value form

The trace's values and the canonical log's values are one form. TP §12.5 names the log's records but does not yet fix
the form of their values: it says the log's schema is fixed with the equivalence law (order of work step 6). This
design states what the form must be. The first slice that lands a schema using it (S1, for the trace, or step 6, for
the log) writes it into TP §12.5, which then owns it.

| SCE type (TP §6.3) | Canonical value |
|---|---|
| `intN`, `uintN` | a string of decimal digits, with `-` before a negative, no leading zero |
| `bool` | `true` or `false` |
| `enum:X` | a string: the variant's name |
| `string[N]` | a JSON string |
| `bytes[N]` | a string of lower-case hexadecimal digits, two to an octet |
| `record:X`, an event schema | an object with one member for each field that is present, named as the field is declared. An absent optional field has no member |
| `list<T>[M..B]` | an array of the elements' values |
| `union:X` | an object with one member, named by the variant of the case, whose value is the case's record, or `{}` for a case that holds none |
| `float32`, `float64` | none: not in the profile (TP §8.1) |

The text is JCS: object members in its order, strings in its escaping, and no number anywhere, since every integer is
a string. JCS writes a number as the shortest form of an IEEE 754 double, which cannot hold every `int64` or `uint64`.
The value is always read against its type from the manifest, so a text, a byte string and a variant's name are never
mistaken for one another.

### 10.4 The surface, planned for slice S1

This design writes no schema. Slice S1 lands the surface as `SCE_WIRE_CONTRACTS.md` requires every surface to land:

- `schemas/sce-test-trace.v1.schema.json`, with `x-sce-schema-status: pre-release`;
- the const `TEST_TRACE_SCHEMA_STATUS` in `sce-build/src/test_trace.rs`. That module holds SCE's reader of a trace
  and its check of a trace against a suite, the refusals of §10.2, offered as a command of `sce-codegen` (design
  choice: `sce-codegen check-trace <suite> <trace>`);
- a row of `SCE_WIRE_CONTRACTS.md`:
  - Format owner: SCE;
  - Status source: `TEST_TRACE_SCHEMA_STATUS` (`sce-build/src/test_trace.rs`) and `x-sce-schema-status`;
  - Status: `pre-release`;
  - Shape governance: this document §10, and `sce-build/src/test_trace.rs`, the reader the native runners' checks
    share;
  - Producers: the corpus's traces, written by hand, and the downstream's recorder (§8.1);
  - Readers: the native trace players (§10.5) and the downstream's trace player.

### 10.5 Native trace replay

Every backend's test runtime already has a system adapter shaped after TRI (TP §12.2), and it is pulled by the run:
the instant of the next input, and the inputs due at an instant. Slice S1 adds to each of the seven runtimes a trace
player behind that interface, which reads a trace file, and generates for each record type of a suite the conversion
from the canonical value to the backend's record. Step 6's log writer needs the same conversion in the other
direction. The two are generated as one pair per type, so that the value form has one implementation per backend.
Slice S1 also adds, per backend, a runner program that runs one test case of a generated suite against one trace and
writes the result. When step 6 lands the canonical log, the runner writes that log. The downstream's lane calls the
runners as processes.

## 11. The lane

### 11.1 Its steps

The downstream's lane runner, for each suite it is given:

1. **Export and validate.** It runs `sce-codegen export-ttcn3`, which writes the two modules, the manifest and the
   witness, and `sce-ttcn3-validate` on them (answer 12). A violation makes the suite red, and nothing of it runs.
2. **Check the traces.** It runs `sce-codegen check-trace` on each of the suite's traces (§10.2).
3. **Build the native side.** It has SCE generate the suite for every backend whose runtime writes the canonical log
   (§11.4), and builds each backend's runner, from the SCE checkout it was given.
4. **Build the TTCN-3 side.** `titan/generate` writes the test ports and the external functions from the manifest and
   the suite's lane entry. The pinned Titan compiles them with the modules into an executable test suite.
5. **Run the test cases.** For each test case and each of its traces:
   - each native runner of step 3 runs the test case against the trace and writes its canonical log;
   - the executable test suite runs the test case alone, in parallel mode under the Main Controller change, with the
     same trace in its system adapter, the run's module parameters in its configuration, and the logger plugin
     writing its canonical log.
6. **Run the control part (design choice).** For each world of the suite's feature model (ADR 0057), both sides run
   the whole suite in that world: the native `runSuite` (TP §12.4), and the control part of the export (E15). Each
   test case the world selects runs with its first trace, by name. This compares the selection (P11) as well as the
   runs. Step 5 runs each test case directly, and so never reaches the control part.
7. **Compare (C3).** For each run of steps 5 and 6, every native log is compared with the Titan log, byte for byte,
   in the canonical order of §11.3. Two logs that differ are a disagreement. The test case is `error` in the lane's
   report, with the first record at which the logs differ, and neither side is taken as right.
8. **Check the refusals.** For each refused fixture of the corpus, `export-ttcn3` must refuse and write no file. The
   export reads no document that `check` refuses (TP §9.5), and the lane holds that over every refusal of ADR 0091 §3.

The lane is red when any test case is `error`. A run that fails on one side (an adapter or codec failure, a crash of
the executable) is `error` for its test case too. Both sides' logs are kept as artefacts of the run.

### 11.2 Why a disagreement is never resolved

Picking one side treats that side as the oracle, and the lane exists because neither is one. Agreement of the two runs
is evidence that both are right only to the degree that their failures are independent (ADR 0091 C2, after Knight and
Leveson). A disagreement shows that at least one side is wrong, and does not say which. So the disagreement is
recorded as `error`, the test system's own fault (P13), and the cause is found and fixed where it lies: in the
document, the exporter, a native runtime, the binding or the tool. It is never fixed in the export (ADR 0091 §8).

The corpus's expected logs (§12) do not change this. They are SCE's record of what its native runs wrote, and they
let SCE notice its own native runs changing. They never decide a disagreement between the two sides.

### 11.3 The canonical order

Titan runs each component in its own process. The order in which two processes' records reach their files is a matter
of scheduling, not of semantics. A byte comparison is therefore made after both logs are put in one order. This
design requires of `sce-test-log.v1` (order of work step 6) that each record carry its instant, its round, its
component, and its position among that component's records. The canonical text of a log orders its records by round,
then by the component's creation index (the MTC first), then by position. The test case's start comes first and its
end last.

A round is one snapshot. Round 0 is the start of the test case. Each later round is one snapshot of TP §3.2 step 3
(TP §12.4 on the native side, epoch e + 1 of §9.2.3 on Titan). Within a component, the order of its records is the
order of its own run, which is sequential on both sides. Across the components of one round, the evaluations are
serialised by the order of TP §3.2 step 4 on both sides. The key therefore gives one order, and the same one, to every
run that takes the same steps.

### 11.4 Which backends take part

The lane compares every backend whose runtime writes the canonical log. Today none does. The native runtimes keep
their log in memory, as records whose details are text and which hold no message values (TP §12.5; C++:
`SCE::Test::LogRecord`). Step 6 adds the writers. The lane's report names, for each backend, whether it took part, and
a backend that did not is reported as not compared, never as agreeing.

### 11.5 Where the lane runs

- **In the downstream's CI**, against its `SCE_PIN`: every change to the binding, the patches or the adapters is
  checked against a fixed SCE.
- **In SCE's CI**, as an external tool. SCE's workflow `.github/workflows/ttcn3-lane.yml` fetches the downstream at a
  pinned revision, builds it (Titan included), and runs its lane runner against SCE's own checkout and corpus. This is
  use of a tool, not a combination: SCE vendors nothing of the downstream and builds nothing of it into SCE. Design
  choices:
  - the pin and the install are a bx tool in `bx.toml`, `[platform.linux.tools.sce-ttcn3-titan]`, whose version is
    the downstream's commit, written the way GenMC's tool is, with `needed_by` naming the lane's gate, so that a host
    without it still takes every other job;
  - the gate `ttcn3-lane` in `tools/git-hooks/gate_registry.py` is `ci_only`, since it builds Titan, the codec
    libraries and every backend that takes part, which is far beyond the push budget;
  - the workflow's `paths:` cover the exporter (`sce-build/src/ttcn3_export/**`), the validator
    (`sce-ttcn3-validator/**`), the corpus and its traces and expected logs, the test runtimes of every backend, the
    generators of their test code, and the pin.

## 12. The corpus

The corpus is SCE's: the suites, their recorded traces, their expected logs, and the codec documents of the message
types their mapped ports carry. ADR 0091 §7 asks it to cover every row of its §2 and every refusal of its §3. An
expected log is the canonical log a reviewed native run wrote for one trace. SCE's own tests hold every backend's run
to it, so a change of native behaviour shows in SCE before it reaches the lane. This is what the tree holds on
2026-11-10.

### 12.1 What runs today

- **`runtime_suite`** (`sce-build/tests/fixtures/test_profile/runtime/`) runs end to end on all seven backends: four
  test cases, an MTC alone and with PTCs, each verdict the profile has, and a guard that ends in `error`.
- **`job_suite`, `ping_suite`, `orders_suite` and `nested_suite`** (`sce-build/tests/fixtures/test_profile/`, its
  `assembly/`, and `sce-build/tests/fixtures/ttcn3_export/`) are exported and validated, and their documents are
  generated, but no suite of them runs end to end.
- **No suite has a trace or an expected log.** No mapped message type of any suite has a codec document (§8.4).

### 12.2 The rows of ADR 0091 §2

| Row | Runs end to end (`runtime_suite`) | Exported only | Missing from every run |
|---|---|---|---|
| Type definitions | records of `uint32` | enums, optional fields, texts, lists and records within records (`job_suite`, `orders_suite`); a union and a byte string (`orders_suite`) | every type but a record of `uint32`; signed and 64-bit integers are in no suite at all |
| Template | specific values, `?`, record matching, value parameters | every mechanism of TP §6.1 (`every_matcher`, in `job_suite`); union matching and template references (`orders_suite`) | every mechanism but the four that run |
| Port type | yes | | |
| Component type | yes | | |
| Test configuration | `connect` and `map` | | |
| Test case | yes | | |
| Module parameters | a feature (PICS), `<sce:module-parameter>` | | |
| Control part | the order, selection by `all` and `any`, guards | | |
| `setverdict` | `pass`, `inconclusive`, `fail` | | `none`, written by a document |
| `alt` with receive branches | yes | | |
| `timer` | start, stop, timeout, a default duration | a restart, a duration given at the start (`nested_suite`) | the restart and the given duration |
| `altstep` and `activate` (ancestor transitions) | | `nested_suite` | in every run |
| `send` | yes | | |
| PTC `create`, `start`, `done`, `kill` | all four | | |
| `log` | yes | | |

Also missing from every run:

- a test case that can never end, which is `error` with no instant pending (P13);
- a dynamic error, such as a send to a removed PTC's port (P6, P13).

### 12.3 The refusals of ADR 0091 §3

Every refusal has a refused fixture that `sce-build`'s tests hold to its diagnostic, with one exception:

| Refusal | Fixture (`refused_*.scxml`) |
|---|---|
| a native expression | `component_native_condition` |
| a construct that raises a script-engine cause | **none**. `test-profile/script-engine` is declared in `sce-build/src/forge/diagnostic.rs`, and no code outside that file raises it |
| a `datamodel` that is not `sce-static` | `component_datamodel`, `port_type_datamodel` |
| a `cond` that reads the received message | `component_cond_reads_message` |
| `<parallel>`, `<history>`, `<script>` | `component_parallel`, `component_history`, `component_script` |
| `<donedata>` as a verdict | `component_donedata` |
| a receive branch whose template is not an incoming type of its port | `component_receive_not_in` |
| a test case selected by an undeclared feature | `suite_feature_undeclared` |
| `<raise>`, and a `<send>` to `#_internal` | `component_raise`, `component_send_internal` |
| a `<send>` whose target is not a port of the component | `component_send_unknown_port` |
| a `<send>` with a delay, and `<cancel>` | `component_send_delay`, `component_cancel` |
| an `<invoke>` of anything but a test component | `component_invoke_not_component`, `component_invoke_type` |
| a transition that is not one branch kind | `component_branch_event`, `component_branch_two_kinds` |
| a done branch whose target keeps the invoking state active | `component_done_branch_stays` |

### 12.4 What the corpus must gain (slice S2)

- a trace for every test case of every runnable suite, and the expected log of each trace;
- a codec document for every message type a mapped port carries, in CBOR or content-line;
- runnable suites that cover the "missing" column of §12.2, including the two `error` cases;
- `job_suite`, `orders_suite` and `nested_suite` (or suites that replace them) made runnable end to end, so that
  their rows run as well as export;
- a fixture that raises `test-profile/script-engine`, or a recorded reason why no profile document can reach that
  refusal once `datamodel` is held to `sce-static`.

## 13. The slices

SCE's slices come first, because the downstream depends on the surfaces they land. Each slice lists what it builds,
the tests it brings, and, for SCE's, the files it changes, so that merges can be planned.

### 13.1 In SCE

**S1: the trace surface and native trace replay.**

- The schema, the const and the row of §10.4, `sce-codegen check-trace`, and the canonical value form written into TP
  §12.5 (§10.3), if step 6 has not written it first.
- The trace player and the generated value conversions of every backend, and each backend's runner (§10.5).
- Tests: per backend, a trace of `runtime_suite` replays to the verdicts its test cases expect, and `check-trace`
  refuses each case of §10.2.
- Files: `schemas/`, `sce-build/src/test_trace.rs`, the command in `sce-build/src/bin/sce_codegen.rs`,
  `SCE_WIRE_CONTRACTS.md`, `docs/SCE_TEST_PROFILE.md`, the seven test runtimes and their generators, and the
  generated test trees those generators write.

**S2: a corpus with traces and expected logs.**

- The additions of §12.4.
- Tests:
  - per backend, every trace of the corpus replays to its expected log, byte for byte;
  - a test of SCE holds the corpus to ADR 0091 §2 and §3: each row and each refusal names the suite or the fixture
    that covers it.
- Depends on the schema and the native writers of `sce-test-log.v1` (order of work step 6), which the expected logs
  are written in. The traces and the codec documents do not depend on it, and can land first.
- Files: the corpus fixtures, their traces, expected logs and codec documents, and the test that holds the coverage.

### 13.2 In the downstream

**D1: the skeleton, the licence and the adapter interface.** The repository, `LICENSE` with the verbatim text of
EPL-2.0, `README.md`, `SCE_PIN`, the headers of `adapter/` (§7.4) after each operation's name and parameters are read
from -5 and -6 (§7.2), and `lane/check-boundary.sh` (§2.4). Tests: the headers compile as C++17 on their own, and the
boundary check fails on a file without the SPDX line and on a path of SCE.

**D2: the pinned Titan build and the Main Controller patch.** `titan/PIN`, `titan/install.sh`, `titan/patches/`
(§9.2) and `titan/tests/` (§9.2.6), and the issue and the merge request upstream (§9.4). Tests: §9.2.6, each run on
the pinned release with the patches applied, and twice for determinism.

**D3: the reference adapters and the codecs.** `reference/`: the value model and its JCS text, the manifest's type
table, the logical clock, the trace player and recorder, the log writer, and the two codec bindings with
`third_party.lock`. Tests:

- the clock against TP §3.3, its boundary cases included: a timer due at the same instant as an input, and the guard;
- the player and the recorder: a recorded trace replays to the same records;
- each codec: the round trip of a value of each type form, and the decode of bytes SCE's own codecs write for the
  same value (the `codec_cbor_map` shape of answer 1, among others), read as files;
- the value model's refusals (§8.3).

**D4: the binding and the logger.** `titan/generate` (test ports and external functions from the manifest),
`titan/testport/` and `titan/logger/`, after the record of what Titan passes with each event (§9.3.3). Tests: the
parser of value notation on the text of each type form, a test port's round trip through its external functions, and
each corpus suite compiled and run on the pinned Titan, writing a log. Depends on the schema of `sce-test-log.v1`
(step 6).

**D5: the lane and C3.** `lane/run.sh`, `lane/compare/` and the suites' lane entries (§11.1). Tests:

- the comparison reports `error` on two logs that differ in one record, and on two that differ only in the order of
  their records before canonical ordering, and reports agreement after it;
- the lane's own run over the corpus at `SCE_PIN`.

When D5 lands, SCE adds the workflow, the bx tool and the gate of §11.5. Depends on the native writers of
`sce-test-log.v1` (step 6): with none, no backend takes part (§11.4).

### 13.3 How the slices meet order of work step 6

Step 6 lands the schema of the canonical log, its row of `SCE_WIRE_CONTRACTS.md`, and the native runtimes' writers.
S2's expected logs, D4's logger and D5's comparison all need them. So the schema and the writers of step 6 land
before S2's expected logs and before D4. Step 6 stays the step that holds the law over the whole corpus.

## 14. What this design leaves open

- **The names of -5 and -6.** §7.2 is checked against the two texts in slice D1.
- **Where the canonical value form is owned.** §10.3 states it, and TP §12.5 takes it over in S1 or in step 6.
- **SCE's own codecs in the lane.** In the lane the native side holds values and uses no codec (TP §12.2), so only the
  TTCN-3 side encodes. Comparing SCE's generated codecs with the reference bindings, byte for byte on the same values,
  would be a check of its own. D3's tests decode SCE's bytes for a few shapes. A check over the corpus is not part of
  this design.
- **Positional binary codecs.** They have no reference binding (§8.4).
- **A second TTCN-3 tool.** Running the suite on a second tool, by a binding of the same interface, remains the
  stronger form of C1 (answer 1). It is not planned.
