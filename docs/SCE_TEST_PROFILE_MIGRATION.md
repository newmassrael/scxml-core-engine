# Moving existing test documents onto the test profile

A consumer whose tests are SCE statecharts reads this file when it moves them onto the test profile of ADR 0091. It
is order of work step 7 of that ADR. It says what each construct of the old shape becomes, what the profile refuses
and what to write instead, in which order to write the new documents, and how to check that a moved test still
decides what the old one decided.

The meaning of every form named here is in `docs/SCE_TEST_PROFILE.md`; this file only says what the consumer writes.
Citations:

- a section with no document named, such as §4.7 or P7, is a section or rule of `docs/SCE_TEST_PROFILE.md`;
- **ADR §n** and **answer n** are sections and answers of
  `docs/adr/0091-a-test-is-a-document-of-the-test-profile-and-its-ttcn-3-export-runs-and-agrees-with-it.md`;
- **subset §n** is a section of `docs/SCE_ACCEPTED_SUBSET.md`.

## 1. Who this is for, and what changes

This file is for a consumer whose tests are ordinary SCE statecharts. Such a test has some of these properties:

- its verdict is the final state it ends in (`<final id="pass"/>`, `<final id="fail"/>`, the convention of the W3C
  conformance documents under `resources/`), or inline `<donedata><content>` text such as `{"verdict":"pass"}` (ADR
  Context; ADR 0003 Context);
- it sends stimuli with `<send>` and recognises the expected reply by a transition's `event` and a `cond` that reads
  `_event.data`, in ECMAScript or as a native `cpp:` condition under `datamodel="null"` (ADR 0003);
- it waits for a reply by a `<send delay>` to itself and a transition on that event;
- its message types are host types (a `<sce:context>` of a C++ type, subset §2.3), and its stimulus and expected
  values live in a host header beside the document (ADR Context);
- it may invoke helper machines, and it may come with a scenario set (subset §2.18).

What changes:

- **A test becomes a set of documents of six kinds** (ADR §2, answer 5): `test-template`, `test-port-type`,
  `test-component`, `test-configuration`, `test-case` and `test-suite`, over the existing `event-schema` and `enum`
  kinds. Their XML is §11.
- **Its meaning is TTCN-3's**, restricted to a subset (ADR §1; §1 to §10). Where SCXML and TTCN-3 differ, the profile
  follows TTCN-3 and refuses the SCXML construct (ADR §1, §3). Section 3 below lists where that changes what a moved
  test does.
- **Its outcome is a test verdict** (`none`, `pass`, `inconclusive`, `fail`, `error`; §5) set by an element, never a
  state's name or a text, **and a test log** in one canonical text (§12.5).
- **It runs on every backend.** Every backend generates every kind of the profile (§11.9) and runs a suite through
  its runner (§12.4). It no longer depends on the test system's implementation language (ADR Context 3).
- **It exports to TTCN-3** (§9.5), so it can be reviewed and, once the conformance lane exists, run outside SCE
  (ADR §6, §7).

What does not change: the old documents keep working where they work today, under ADR 0003's accounting of the
escape hatch (ADR Consequences). Moving is per test, and an old test and its new form can run side by side, which
section 7 needs.

## 2. Construct by construct

Each row names the old form, the profile form, the rule that gives the profile form its meaning, and the diagnostic
a `test-component` gets if the old form is left in it (subset §2.1, the paragraphs on the `test-` kinds; §8.2).

| Old form | Profile form | Rule | Old form left in a component |
|---|---|---|---|
| `<final id="pass"/>` and `<final id="fail"/>` read by the harness; `<donedata><content>{"verdict":"pass"}</content></donedata>` | `<sce:setverdict value="pass\|inconclusive\|fail\|none" reason="…"/>` in the branch, then a top-level `<final>`, whose id means nothing to the verdict | P10, P12, §11.5; ADR §4 | `<donedata>`: `test-profile/component-construct` |
| an expected event: `<transition event="e" cond="…">`, the `cond` reading `_event.data` | a receive branch `<transition sce:receive="p" sce:test-template="t">` over an imported `test-template`; a `cond` that stays reads component variables only | P2, P3, §6, §11.4, §11.5; answer 9 | `event`: `test-profile/branch`; `_event` in a `cond`: `test-profile/message-read`; `cpp:`: `test-profile/native-code` |
| a stimulus: `<send event="e" target="…">` with `<param>`, `namelist` or `<content>` | `<send type="http://sce.dev/ext/test-port" target="<port>" sce:test-template="t" sce:template-args="v …"/>`, the template specific | P6, §6.2, §11.5 | `test-profile/send`; a template that is not specific: `test-profile/specific-template`; a record not on the port's `out` list: `test-profile/port-direction` |
| a timeout: `<send event="t" delay="d"/>` to itself and `<transition event="t">`; `<cancel>` of it | `<sce:test-timer id="T" duration="d"/>`; `<sce:start-timer timer="T"/>`; a timeout branch `<transition sce:timeout="T">`; `<sce:stop-timer timer="T"/>` | P7, §11.5 | `delay`: `test-profile/send`; `<cancel>`: `test-profile/component-construct`; a duration that is not whole microseconds: `test-profile/duration` |
| the harness's time limit for the whole test | `guard` on the test case's `<sce:execute>` | P11; answer 4 | none: the guard is the suite's |
| an invoked helper: `<invoke type="scxml" src="helper.scxml">`, talking over `#_parent` and `#_<id>` | a `test-component` started by `<invoke type="http://sce.dev/ext/test-component" src="…" id="…"/>`, declared as a `<sce:ptc>` of the configuration, its ports joined by `<sce:connect>`; `done.invoke.<id>` as a done branch that exits the invoking state | P9, §11.5, §11.6; answer 10 | another `type`, `srcexpr`, `<param>`, `namelist`, `autoforward`, `<finalize>`: `test-profile/invoke`; a send to `#_parent`: `test-profile/send`; a done branch that keeps its state: `test-profile/done-branch` |
| a child chosen at run time: `srcexpr` with `sce:candidates` (subset §2.13) | one configuration per candidate, each run by its own test case; where a feature of the product chose the child, the suite selects the test case by `requires` | §11.6 to §11.8 | `srcexpr`: `test-profile/invoke` |
| concurrent regions: `<parallel>` | one PTC per region, joined by ports | ADR §3; §8.1 | `test-profile/component-construct` |
| the implementation under test reached from the test document: an invoked machine, an `<sce:action>` (subset §2.11) | the system under test behind a system port the configuration maps (`<sce:system-port>`, `<sce:map>`), reached through the consumer's system adapter | §11.6, §12.2; answer 10 | an `<invoke>` of anything but a test component: `test-profile/invoke`; `<sce:action>` is refused by name on Rust, Kotlin, Go, Python, C11 and Swift (§11.9), and the export writes no element outside its list (§9.5) |
| `datamodel="ecmascript"` or `datamodel="null"` | `datamodel="sce-static"` (subset §2.15) | §11.2; answer 9 | `test-profile/datamodel`; a construct that needs a script engine: `test-profile/script-engine` |
| message types as host types, or `_event.data` with no type | one `event-schema` per message, whose `sce:event-name` is the old event name, and `enum` documents for its enumerations | §6.3, §11.3; subset §2.1 | a template over a float: `test-profile/template-float` |
| elements gated by a feature inside the test (`sce:requires`, subset §2.22) | a component per variant, each in its own test case, selected by `requires` on its `<sce:execute>`; boolean PICS are features of the suite's `feature-model` | §11.8, §11.10; ADR §2 | importing a `feature-model`: `test-profile/component-construct`; `sce:requires` with no model: `validation/feature-undeclared` |
| a value the test bench or the product decides (PIXIT, a numeric PICS item), in a `<data expr>` or the host header | `<data id="x" sce:type="…" sce:module-parameter="true"/>` in the component and `<sce:module-parameter id="x" type="…" value="…"/>` in the suite | §11.5, §11.8; ADR §2 | an `expr` on it: `test-profile/module-parameter-written`; no declaration in the suite: `test-profile/module-parameter` |
| expected and stimulus values in a host header | literals and formal parameters of `test-template` documents | §6, §11.4 | not applicable |
| `<raise>`, or a `<send>` to `#_internal` | states and eventless transitions | §8.2 | `<raise>`: `test-profile/component-construct`; `#_internal`: `test-profile/send` |
| `<script>` | `<assign>`, `<if>`, `<elseif>`, `<else>` over `sce-static` variables | §8.2, §9.5 | `test-profile/component-construct` |
| `<history>` | the states it would restore, written out | §8.2 | `test-profile/component-construct` |
| a transition on `error.*`, on `done.state.<id>` or on `*` | for `error.*`, nothing: a dynamic error is the runtime's `error` (§5.2); for `done.state.<id>`, the next state named as the target of the branch that ends the compound state's work; for `*` as "anything unexpected", a receive any branch on an ancestor state (a default, P5) | P5, §5.2 | `test-profile/branch` |

ADR §3 refuses two more things that a moved test can meet: a receive branch whose template's record is not on its
port type's `in` list (`test-profile/port-direction`), and a test case selected by a feature its suite's
`feature-model` does not declare (`validation/feature-undeclared`).

## 3. Where a moved test means something else

The rows above keep a test's intent. These five points are where the profile, following TTCN-3, gives the same
document a different run. Each is the first place to look when section 7 finds a disagreement.

1. **A message that no branch matches stays at the head of its port.** SCXML removes an event that enables no
   transition and discards it; the profile removes nothing and waits for the next round (§7, the third row; ADR §5).
   Behind it, every later message of that port is unseen. An old test that ignored an unexpected reply and passed on
   a later one now times out, unless the new one says what happens to the unexpected message:
   - a default that judges it: a receive any branch on an ancestor state that sets `fail` or `inconclusive` (P5), the
     form TTCN-3 gives unexpected messages (ADR §5);
   - a branch that drops it: a receive any branch with no target, after the expected branches (the form -4 gives
     `trigger`, §8.1), which reproduces the old discard exactly.
2. **A verdict is the greatest one set, and every component's counts.** `setverdict` never lowers a verdict, in the
   order `none < pass < inconclusive < fail` (P12), and the test case verdict is the greatest over the MTC and every
   PTC, removed or not (P14). A helper that sets `fail` fails the test case even when the MTC passes, where the old
   parent decided alone. A component that reaches a top-level `<final>` with no verdict set keeps `none` (§2.1, P10);
   §8.2 and §11.5 do not refuse it, so set one on every path.
3. **A reply and a timeout due at one instant are taken in document order.** Everything due at an instant is
   delivered before the snapshot, and the component tries its branches in document order (P3; answer 3). Write the
   receive branch before the timeout branch if a reply at the deadline passes, after it if it fails. In the old
   test the order was its engine's queue order.
4. **Time is the runtime's.** `NOW` is a count of microseconds the runtime advances to the next timer expiry, input
   or guard when no branch can fire (§3.3, §12.4). A run is a function of the document and of the inputs the system
   adapter gives with their instants, so it does not depend on how fast the machine is.
5. **`error` is not `fail`.** The runtime alone sets `error`: a guard that expires, a test case that can never end, a
   codec or adapter failure, a dynamic error (§5.2). The document cannot set it. An old test whose harness counted
   its time limit as `fail` ends in `error` when its guard expires (P11).

## 4. A worked example

### 4.1 Before

A test in the old shape. It asks the system under test for job 7 and passes when a reply of success for that job
arrives within a second. Its route to the system under test is whatever the consumer's harness gives it; SCE does
not run or check this document here.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" initial="asking">
  <datamodel>
    <data id="job" expr="7"/>
  </datamodel>
  <state id="asking">
    <onentry>
      <!-- target: the route the harness gives the system under test -->
      <send event="job.request" target="sut">
        <param name="job" expr="job"/>
      </send>
      <send id="deadline" event="deadline" delay="1s"/>
    </onentry>
    <transition event="job.reply"
                cond="_event.data.code === 0 &amp;&amp; _event.data.job === job"
                target="pass">
      <cancel sendid="deadline"/>
    </transition>
    <transition event="deadline" target="fail"/>
  </state>
  <final id="pass"/>
  <final id="fail"/>
</scxml>
```

### 4.2 After

The profile form of this test is the `answers_the_job` test case of the fixtures the native runtime runs end to end on
every backend, under `sce-build/tests/fixtures/test_profile/runtime/`. Each block below is that file's `<scxml>`
element as committed; the files are the reference, and this section follows them.

The two message types, `job_request.scxml` and `job_reply.scxml` (`event-schema`, §6.3). Each keeps the old event
name as its `sce:event-name`:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_request"
       sce:event-name="job.request">
  <datamodel>
    <data id="job" sce:type="uint32" sce:direction="in"/>
  </datamodel>
</scxml>
```

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_reply"
       sce:event-name="job.reply">
  <datamodel>
    <data id="job" sce:type="uint32" sce:direction="in"/>
    <data id="code" sce:type="uint32" sce:direction="in"/>
  </datamodel>
</scxml>
```

The port type of the side that asks, `exchange_port.scxml` (§11.3): requests out, replies in.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="exchange_port">
  <sce:import kind="event-schema" src="job_request.scxml" as="JobRequest"/>
  <sce:import kind="event-schema" src="job_reply.scxml" as="JobReply"/>
  <sce:message type="JobRequest" direction="out"/>
  <sce:message type="JobReply" direction="in"/>
</scxml>
```

The two templates (§11.4): `request_of.scxml`, the specific request for the job its parameter names, which the
component sends; and `reply_ok.scxml`, a reply of success for any job, which it receives.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="request_of"
       sce:type="record:JobRequest">
  <sce:import kind="event-schema" src="job_request.scxml" as="JobRequest"/>
  <datamodel>
    <data id="wanted" sce:type="uint32" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:specific field="job" param="wanted"/>
  </sce:record-match>
</scxml>
```

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="reply_ok"
       sce:type="record:JobReply">
  <sce:import kind="event-schema" src="job_reply.scxml" as="JobReply"/>
  <sce:record-match>
    <sce:any-value field="job"/>
    <sce:specific field="code" value="0"/>
  </sce:record-match>
</scxml>
```

The component, `param_asker.scxml` (§11.5), the MTC:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="param_asker"
       datamodel="sce-static" initial="asking">
  <sce:import kind="test-port-type" src="exchange_port.scxml" as="ExchangePort"/>
  <sce:import kind="test-template" src="request_of.scxml" as="RequestOf"/>
  <sce:import kind="test-template" src="reply_ok.scxml" as="ReplyOk"/>
  <sce:import kind="event-schema" src="job_reply.scxml" as="JobReply"/>
  <sce:test-port id="sut" type="ExchangePort"/>
  <sce:test-timer id="deadline" duration="1s"/>
  <datamodel>
    <data id="job" sce:type="uint32" sce:module-parameter="true"/>
    <data id="answered" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="asking">
    <onentry>
      <send type="http://sce.dev/ext/test-port" target="sut"
            sce:test-template="RequestOf" sce:template-args="job"/>
      <sce:start-timer timer="deadline"/>
    </onentry>
    <transition sce:receive="sut" sce:test-template="ReplyOk" target="checking">
      <sce:stop-timer timer="deadline"/>
      <assign location="answered" expr="_event.data.job"/>
    </transition>
    <transition sce:timeout="deadline" target="failed">
      <sce:setverdict value="fail" reason="'no matching reply in time'"/>
    </transition>
  </state>
  <state id="checking">
    <transition cond="answered === job" target="passed">
      <sce:setverdict value="pass"/>
    </transition>
    <transition target="failed">
      <sce:setverdict value="fail" reason="'the reply answers another job'"/>
    </transition>
  </state>
  <final id="passed"/>
  <final id="failed"/>
</scxml>
```

The configuration, `asking_configuration.scxml` (§11.6): the component is the MTC, and its port `sut` is mapped to the
port `sut` of the test system interface, where the system adapter stands.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-configuration" name="asking_configuration">
  <sce:import kind="test-component" src="param_asker.scxml" as="ParamAsker"/>
  <sce:import kind="test-port-type" src="exchange_port.scxml" as="ExchangePort"/>
  <sce:mtc type="ParamAsker"/>
  <sce:system-port id="sut" type="ExchangePort"/>
  <sce:map component="mtc" port="sut" system-port="sut"/>
</scxml>
```

The test case, `answers_the_job.scxml` (§11.7):

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-case" name="answers_the_job"
       sce:configuration="Asking">
  <sce:import kind="test-configuration" src="asking_configuration.scxml" as="Asking"/>
</scxml>
```

The suite, `runtime_suite.scxml` (§11.8). It runs `answers_the_job` in every world, with a guard of ten seconds, and
gives the module parameter `job` its default, 7. Its three other test cases are other fixtures of the runtime and have
no counterpart in the old test.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="runtime_suite">
  <sce:import kind="feature-model" src="runtime_features.scxml" as="Features"/>
  <sce:import kind="test-case" src="answers_the_job.scxml" as="AnswersTheJob"/>
  <sce:import kind="test-case" src="serves_a_ptc.scxml" as="ServesAPtc"/>
  <sce:import kind="test-case" src="keeps_a_ptc.scxml" as="KeepsAPtc"/>
  <sce:import kind="test-case" src="answers_before_the_guard.scxml" as="AnswersBeforeTheGuard"/>
  <sce:module-parameter id="job" type="uint32" value="7"/>
  <sce:execute test-case="AnswersTheJob" guard="10s"/>
  <sce:execute test-case="ServesAPtc" requires="ptcs"/>
  <sce:execute test-case="KeepsAPtc" requires="all(ptcs, removal)"/>
  <sce:execute test-case="AnswersBeforeTheGuard" guard="0.1ms" requires="any(guard, removal)"/>
</scxml>
```

### 4.3 What became what

| Before | After |
|---|---|
| `datamodel="ecmascript"` | `datamodel="sce-static"` on a `test-component` (§11.2) |
| `<data id="job" expr="7"/>` | `<data id="job" sce:type="uint32" sce:module-parameter="true"/>`, its value the suite's `<sce:module-parameter id="job" type="uint32" value="7"/>`, which a run may override (§12.4). Where the value is no parameter of the bench, it stays a variable with its `expr`, as the fixture `asker.scxml` writes it |
| the event `job.request` and its `<param name="job">` | the record `job_request` with its field `job`; the specific template `RequestOf`, its parameter bound to `job` by `sce:template-args` |
| the route `sut` | the port `sut` of type `exchange_port`, mapped to the system port `sut` |
| `<send id="deadline" event="deadline" delay="1s"/>` | `<sce:test-timer id="deadline" duration="1s"/>` and `<sce:start-timer timer="deadline"/>` |
| `<cancel sendid="deadline"/>` | `<sce:stop-timer timer="deadline"/>` |
| `event="job.reply"` with a `cond` on `code` and `job` | the receive branch over `ReplyOk`, which matches `code` 0 and any `job`; the value redirect `<assign location="answered" expr="_event.data.job"/>`; and the state `checking`, whose `cond` reads the variable, not the message (answer 9) |
| `<transition event="deadline" target="fail"/>` | the timeout branch, which sets `fail` with a reason |
| `<final id="pass"/>`, `<final id="fail"/>` | `<sce:setverdict>` on each path, then `passed` or `failed`, whose names mean nothing to the verdict |
| the document is the test | the test case `answers_the_job` names the configuration, and the suite runs it under a guard |

The old `cond` asked two things of the message. The new test asks the template what can be matched without the
component's state (a reply of success) and asks the component's variable the rest. A parameterised template could ask
both: `reply_of.scxml` in the same directory matches a job and a code given as parameters, each bound to a component
variable by `sce:template-args` (§11.5). On the behaviours of section 4.4 the two forms decide the same verdicts; they
differ in when the test ends on a reply of success for another job, which the parameterised form leaves at the head
of `sut` until the deadline.

### 4.4 Where the two differ

Four behaviours of the system under test, each with the old outcome and the new one:

| The system under test | Before | After |
|---|---|---|
| answers job 7 with code 0 within the second | `pass` | `pass`. Its test log, when the reply comes 500 microseconds after the request, is `expected_logs/answers_the_job.json` (section 7) |
| answers with a code other than 0 | the reply enables no transition and is discarded; `fail` at the deadline | the reply matches no branch and stays at the head of `sut` (section 3, point 1); `fail` at the deadline, and the log has no `message-received` |
| answers code 0 for another job | the reply is discarded; `fail` at the deadline | the reply is taken, and `checking` sets `fail` at once with the reason `the reply answers another job` |
| answers with a code other than 0, then with code 0 for job 7, both within the second | the first is discarded, the second passes: `pass` | the first blocks the second: `fail` at the deadline |

The first three agree in verdict; the third ends earlier and says why. The fourth disagrees, by section 3, point 1.
Whether the old test meant to ignore a failing reply is the consumer's question. If it did, one branch on `asking`,
after the timeout branch, takes and drops any other message of the port:

```xml
<transition sce:receive="sut"/>
```

It is a receive any branch with no target (P2; §8.1, `trigger`). It is not in the fixture, whose test fails on such a
reply, as the Rust test `fails_a_job_the_system_under_test_answers_with_a_failure` holds.

## 5. The order of the work

Write the documents from the bottom of the import graph up, so that each one is checked against documents that exist.

1. **Types.** One `event-schema` per message the test sends or receives, its `sce:event-name` the event name the
   implementation uses, each field of a type §6.3 lists; an `enum` for each enumeration.
   These replace the host types and the host header's field layouts (ADR Context 3). A record that holds a float has
   no template (§11.4, `test-profile/template-float`), because the profile leaves `float` out (§8.1); a template over
   a union is generated on Rust and Kotlin and refused on the other backends (§11.9).
2. **Port types.** One `test-port-type` per interface of the implementation, each message with its direction seen
   from the tester (§11.3). Two messages of one port type have two event names. Give each point of control and
   observation its own port: queues of different ports are never merged (§2.2), which is what the old shape could
   not say (ADR Context 4).
3. **Templates.** One `test-template` per expected message shape, and one specific template per stimulus (§6.2,
   §11.4). The values the host header held become literals and formal parameters.
4. **Components.** Rewrite each test statechart as a `test-component` by the table of section 2, then read section
   3 against it. Write a default on an outer state for messages no branch expects, and a verdict on every path to a
   top-level `<final>`.
5. **Configuration.** One `test-configuration` per arrangement of components: the MTC, each PTC, the system ports,
   and a `<sce:map>` or `<sce:connect>` for every port, each port joined exactly once (§11.6).
6. **Test cases.** One `test-case` per test, naming the configuration it runs on (§11.7). Its name is the name the
   suite, the test log and the export give it.
7. **Suite.** One `test-suite`: the test cases in run order, each with its guard and its selection, and the module
   parameters every component reads, with their defaults (§11.8).
8. **Check.** `sce-codegen check <suite.scxml> -l <language>` holds the suite and every document it reaches through
   its test cases and their configurations to the profile's rules (subset §2.1). Fix every refusal before generating.
9. **Generate and run on a backend.** Generate each document with `sce-codegen generate <document> -l <language>`.
   §11.9 says what each kind writes on each backend, and what each backend refuses by name; Go needs
   `--go-module-prefix` for a component that uses a template and for the tables. Then write the system adapter, the
   implementation's side of each mapped port (§12.2), and call the backend's runner with the suite's tables, a world
   of its features and a factory of adapters (§12.4):

   | Backend | Runner | Adapter interface | An adapter to read |
   |---|---|---|---|
   | C++ | `SCE::Test::runSuite` (`sce/include/test/Suite.h`) | `SCE::Test::SystemAdapter` (`sce/include/test/TestCaseRun.h`) | `tests/integration/ATestSuiteRunsEndToEndTest.cpp` |
   | Rust | `sce_rust_runtime::test::run_suite` | `sce_rust_runtime::test::SystemAdapter` | `backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs` |
   | Kotlin | `com.sce.runtime.test.runSuite` | `com.sce.runtime.test.SystemAdapter` | `backends/kotlin/tests/src/test/kotlin/com/sce/integration/ATestSuiteRunsThroughItsRunnerTest.kt` |
   | Go | `testprofile.RunSuite` | `testprofile.SystemAdapter` | `backends/go/tests/integration/static_datamodel/a_test_suite_runs_through_its_runner_test.go` |
   | Python | `sce_runtime.testprofile.run_suite` | `sce_runtime.testprofile.run.SystemAdapter` | `backends/python/tests/integration/static_datamodel/test_a_test_suite_runs_through_its_runner.py` |
   | C11 | `sce_test_run_suite` (`sce/test/suite.h`) | `sce_test_system_adapter_t` (`sce/test/test_case_run.h`) | `backends/c/tests/integration/test_a_test_suite_runs_through_its_runner.c` |
   | Swift | `SceTestProfile.runSuite` | `SceTestProfile.SystemAdapter` | `backends/swift/tests/Tests/StaticDatamodelTests/ATestSuiteRunsThroughItsRunnerTests.swift` |

   The adapter is shaped after TRI: a send is `triSend`, and the inputs it returns with their instants are what
   `triEnqueueMsg` would enqueue (§12.2). It owns the codec and the transport, and the run sees values only (ADR C2).
   A test case for which the factory builds no adapter ends with `error` (§12.4).
10. **Read the result.** Each test case's result is its verdict, the lines of its log, and its test log (§12.5).
    The lines are for a reader, and name the cause of an `error`. The test log is the records of TCI's test
    logging interface in one canonical text, which each runtime writes by the function §12.5 names for it, and two
    runs agree exactly when its bytes do (§12.5).
11. **Export, and run the export once the lane exists.** `sce-codegen export-ttcn3 <suite.scxml> --output-dir <dir>`
    writes the types module, its manifest, the behaviour module and the correspondence witness (§9.5), and
    `sce-ttcn3-validate <suite.scxml> <dir>` checks the witness apart from the exporter (§9.4). The export refuses
    names TTCN-3 cannot take (`test-profile/export-name`, never renamed) and constructs with no TTCN-3 form that
    computes the same value, among them integer arithmetic in a component (§9.5, **Expressions**); a document
    meant for export is written within them from the start. The conformance lane that runs the export on Eclipse
    Titan against the same input traces and compares the two test logs (ADR §7, C3; order of work steps 5 and 6) is
    not in the tree yet. Until it lands, the export is read and validated, and nothing in SCE runs it.

A corpus that runs in C++ today can take C++ first, as the native runtime did (§12.1). Nothing in the profile ties a
test to one backend: the documents are the same on all seven.

## 6. What stays where it was

- **A scenario set keeps its own role.** A scenario set (subset §2.18) is a specification owner's examples, judged
  against a design by `sce-codegen judge-scenarios` over an observation trace (subset §2.19), and its verdicts are
  `pass`, `fail`, `not-judged`, `blocked` and `awaiting-decision` (ADR Context). It is not lowered to test cases, and
  none of its verdicts maps onto a test verdict (answer 7). Do not rewrite a scenario as a test case to move it: keep
  the set, its traces and its judgement as they are. Its runnable scenarios are the test purposes from which a later
  ADR derives test cases (answers 7 and 11); that derivation does not exist yet.
- **A design's examples are not tests.** A scenario says what a specification says should be observed. It does not
  say what the tester does with every other output, so it is not a test case (answer 7). A test case written by hand
  for the same behaviour is a separate document, and the scenario stays.
- **A replay scenario stays.** The scenarios every backend's driver replays against a generated `sce-static` machine
  (subset §2.18, "A replay scenario") drive a design's machine directly, not an implementation through ports, and
  stay as they are.
- **The W3C conformance fixtures stay.** The documents registered in `tests/w3c/conformance/fixtures.json`
  (`docs/SCE_W3C_TEST_REGISTRATION.md`) test an SCXML processor against the W3C suite, and every backend must pass
  them (ADR §8, Key Principle 1). Their `pass` and `fail` finals are the convention the old shape of section 1
  copies; they are not test-profile documents and are not moved.

## 7. Checking that a moved test is equivalent

A moved test is equivalent to the old one when, run against the same implementation behaving the same way, it
decides the same outcome. The old test has no canonical log, so the old and the new are compared by outcome. From
the move on, the new test's test log is the comparison surface: between runs, between backends, and with the TTCN-3
run once the lane exists (§12.5; ADR C3, answer 8).

1. **List the behaviours.** For each final state of the old test (each donedata verdict), name a behaviour of the
   implementation that leads to it: a reply in time, a wrong reply, no reply. Add the behaviours section 3 makes
   different: an unexpected message before the expected one, a reply at the instant of the deadline, a helper that
   fails. Section 4.4 is this list for the worked example.
2. **Run both against each behaviour.** Drive the implementation, or a scripted stand-in for it, the same way under
   the old harness and under the new system adapter. The end-to-end tests of section 5, step 9 are adapters of this
   kind: each plays a system under test that answers a request after a fixed delay with a given code.
3. **Compare the outcomes.** Old `pass` and new `pass` agree, and old `fail` and new `fail` agree. A new verdict the
   old test had no word for is a finding to explain:
   - `none`: a path reached a top-level `<final>` with no verdict set (P10, P12);
   - `inconclusive`: set by the new document, which the move introduced on purpose or by mistake;
   - `error`: the guard expired, the test case could never end, the adapter failed, or a dynamic error happened
     (§5.2); the result's lines name which.
4. **Explain every disagreement.** Each is either one of the five points of section 3 or a defect of the move. Fix
   the document, and decide the case by what the old test meant; a disagreement is never settled by taking either
   side's verdict as right, which is the rule C3 applies to the native and the TTCN-3 runs.
5. **Check the stimuli.** Each `message-sent` record of the test log holds the port, the record's type name and the
   value in the profile's canonical form (§12.5). Compare them with what the old test sent, its `<param>` values or
   its host header's.
6. **Keep the log as the reference.** For each behaviour whose exchange is scripted, keep the canonical test log of
   the passing run and hold every later run to its bytes. The tree does this with its own example: the run of
   `answers_the_job` in a world with no feature, answered with success, is held on all seven backends to one file,
   `sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json` (§12.5), by tests such as
   `ATestSuite.AGeneratedTestCaseWritesItsTestLog` in C++ and `a_generated_test_case_writes_its_test_log` in Rust:

   ```json
   {"records":[{"at":"0","kind":"test-case-started"},{"at":"0","component":"mtc","kind":"component-created","type":"param_asker"},{"at":"0","component":"mtc","kind":"message-sent","port":"sut","type":"job_request","value":{"job":"7"}},{"at":"0","component":"mtc","duration_us":"1000000","kind":"timer-started","timer":"deadline"},{"at":"500","component":"mtc","kind":"message-received","port":"sut","type":"job_reply","value":{"code":"0","job":"7"}},{"at":"500","component":"mtc","kind":"timer-stopped","timer":"deadline"},{"at":"500","component":"mtc","kind":"verdict-set","verdict":"pass"},{"at":"500","component":"mtc","kind":"component-done","verdict":"pass"},{"at":"500","kind":"test-case-ended","verdict":"pass"}],"schema":"sce-test-log.v1","suite":"runtime_suite","test_case":"answers_the_job","verdict":"pass"}
   ```

   A run is a function of the document and of the input trace on the logical clock (§3.1, §3.3, §12.4), so the same
   script gives the same bytes on every backend, and a difference is a finding. Against a live implementation whose
   timing is its own, compare the verdict and the message records, not the instants.
7. **Once the lane exists,** the same test log, written by the TTCN-3 run of the export, is compared with the native
   one, and a difference is `error` for that test case (ADR C3). The TTCN-3 run of a scripted exchange is then held
   to the same bytes as the reference kept in step 6.
