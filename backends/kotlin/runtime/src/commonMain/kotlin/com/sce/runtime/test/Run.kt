// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The run of one test case of the test profile (docs/SCE_TEST_PROFILE.md §2 to
// §5, §12): its entities, its ports and timers, the round scheduler on the
// logical clock, and the test case verdict.
//
// The package `com.sce.runtime.test` is the runtime of the test profile
// (§12.2): the round scheduler, `NOW`, port queues and snapshots, timer
// bindings, verdicts, the entity table and the guard, written once for every
// suite. A suite's generated units reach it from here: a test component's
// machine through its [ComponentContext], which it keeps from
// [ComponentMachine.start] on; a configuration as a [Configuration] and a
// [MachineFactory]; a test case and a suite as a [TestCase] and a [Suite], which
// [runSuite] runs. The algorithm is the one the document specifies and the C++
// runtime (`sce/include/test/`) and the Rust one (`sce_rust_runtime::test`)
// implement, type for type.
//
// A component's machine is generated from its `test-component` and reaches this
// run only through its context (§12.3): the snapshot its branches' guards read,
// the statement each taken branch runs first, the statements of the profile, and
// the doors of a send and of a PTC. The run knows nothing of the engine or of
// the record types, so it is tested on its own.
//
// Ownership. The state of a run is shared between the run and a context per
// component, as the C++ runtime's context holds a reference to its run: the
// machine keeps its context from `start` on, and so may the host processors it
// registers on its engine for the doors of a send and of a PTC. A run is driven
// on one thread (§12.4), so the state takes no lock, where the Rust runtime's
// takes a mutex because its engine holds those processors as `Send` closures.
//
// The run never holds its state across a call into a machine: every step of it
// marks the state held for its own length ([access]), and a machine is called
// between steps. So every call a machine makes into its context finds the state
// free. A call that finds it held is a call made while the run, or the same
// component, is already inside the state, which on one thread can only be a
// re-entrant call (a snapshot reader that calls its context, say), and it throws
// an IllegalStateException naming the component, as the Rust runtime panics.
// Kotlin has no lock to poison: an exception that leaves a step part way goes
// out of the run, as a panic does.

package com.sce.runtime.test

import com.sce.runtime.StateRefusal

/** The name of the MTC in a configuration; no PTC takes it (§11.6). */
const val MTC_NAME: String = "mtc"

/**
 * A component port named in a configuration: the component's name (`mtc` or a
 * PTC's id) and the port's id.
 */
data class Endpoint(
    /** The component's name. */
    val component: String,
    /** The port's id. */
    val port: String,
) {
    internal fun names(component: String, port: String): Boolean =
        this.component == component && this.port == port
}

/**
 * A `test-configuration` as the run reads it (§11.6).
 *
 * Generated from the configuration document. Each port takes part in one
 * connection or one mapping (§2.2), which the build has checked.
 */
data class Configuration(
    /** The MTC's component type. */
    val mtcType: String = "",
    /** Each PTC's name and its component type. */
    val ptcTypes: Map<String, String> = emptyMap(),
    /** The port ids each component type declares. */
    val portsOfType: Map<String, List<String>> = emptyMap(),
    /** The timer ids each component type declares. */
    val timersOfType: Map<String, List<String>> = emptyMap(),
    /** The `<sce:connect>` pairs (-4 §9.10). */
    val connections: List<Pair<Endpoint, Endpoint>> = emptyList(),
    /** The `<sce:map>` entries: a component port and its system port (-4 §9.32). */
    val mappings: List<Pair<Endpoint, String>> = emptyList(),
)

/**
 * What a line of the run's log records (§12.5): the run as a reader follows
 * it, with the removals of PTCs and the cause of an `error` in the runtime's
 * words. The test log, which two runtimes compare, is [TestCaseResult.records].
 */
enum class LogKind {
    /** The MTC was created and started. */
    START,

    /** A PTC was created and started. */
    CREATE,

    /** A component sent on a port. */
    SEND,

    /** A receive branch removed the head of a port. */
    RECEIVE,

    /** A timeout branch took a timer. */
    TIMEOUT,

    /** A PTC terminated (P10). */
    DONE,

    /** A PTC was removed. */
    KILL,

    /** A component set its local verdict. */
    VERDICT,

    /** A `<log>`. */
    LOG,

    /** A dynamic error ended the run (P13). */
    ERROR,
}

/** One line of the run's log. */
data class LogRecord(
    /** `NOW` when it was recorded. */
    val at: Instant,
    /** The component it is of. */
    val component: String,
    /** What it records. */
    val kind: LogKind,
    /**
     * The port and event, the timer, the verdict and reason, the text or the
     * error, by kind; empty for a kind that has none.
     */
    val detail: String,
)

/**
 * The outcome of one test case: its verdict, the instant it ended at, the lines
 * of its log, and its test log.
 */
data class TestCaseResult(
    /** The test case verdict (P14). */
    val verdict: Verdict = Verdict.NONE,
    /** `NOW` when the run ended. */
    val endedAt: Instant = 0,
    /** The lines of the run's log, for a reader. */
    val log: List<LogRecord> = emptyList(),
    /**
     * The test log (docs/adr/0091, C3): the records of TCI's test logging
     * interface, in the order the run produced them, which [canonicalLog]
     * writes.
     */
    val records: List<TestLogRecord> = emptyList(),
) {
    /**
     * The test log of this run of the test case [testCase] of the suite
     * [suite], in its one canonical text: the JSON Canonicalization Scheme of
     * `schema`, `suite`, `test_case`, `verdict` and `records`, each record in
     * the order the run produced it.
     *
     * @throws IllegalStateException when a message's value is not in the
     *   canonical form ([TestValue.canonical]), which no writer of [Canonical]
     *   builds.
     */
    fun canonicalLog(suite: String, testCase: String): String = writeTestLog(suite, testCase, verdict, records)
}

/**
 * The SUT's side of the mapped ports (§12.2).
 *
 * Shaped after TRI (ES 201 873-5): [send] is `triSend`, and the inputs
 * [takeInputsDue] returns are what `triEnqueueMsg` would enqueue. The adapter
 * owns the codec and the transport; the run sees values only (ADR 0091 C2). The
 * instants are the logical clock's, so a recorded trace replays to the same run
 * (§3.1).
 */
interface SystemAdapter {
    /** A component sent [message] on a port mapped to [systemPort]. */
    fun send(systemPort: String, message: Message, now: Instant)

    /** The instant of the earliest input not yet taken, if any. */
    fun nextInput(): Instant?

    /**
     * Every input due at or before [now], in arrival order, each with the
     * system port it arrives on.
     */
    fun takeInputsDue(now: Instant): List<Pair<String, Message>>
}

/**
 * A component's machine, as the run drives it (§12.3).
 *
 * Generated from a `test-component`: the backend's statechart machine, with
 * each branch a transition on the round's snapshot event.
 */
interface ComponentMachine {
    /**
     * Enter the initial configuration and complete the first macrostep. The
     * machine keeps [context], and every call it makes, now and on each later
     * delivery, goes to it.
     */
    fun start(context: ComponentContext)

    /**
     * Deliver the round's snapshot event: the machine takes the first branch
     * whose guard holds on the snapshot (P3) and completes its macrostep (P4),
     * or takes none.
     */
    fun deliverSnapshot()

    /** The machine has entered a top-level `<final>` (P10). */
    fun terminated(): Boolean
}

/**
 * Builds the machine of a component type, or null for a type it does not make,
 * which ends the run with `error` where a component of that type is created.
 */
typealias MachineFactory = (componentType: String) -> ComponentMachine?

/** What one component holds in the run. */
internal class ComponentState {
    val ports = mutableMapOf<String, Port>()
    val timers = mutableMapOf<String, Timer>()
    var doneSnapshot: Set<String> = emptySet()
    var verdict = Verdict.NONE
    var branchTaken = false
}

internal class Entity(
    /**
     * Which creation this is: a PTC removed and created again in one round is a
     * new component, which took no snapshot in it.
     */
    val serial: Long,
    val state: ComponentState,
) {
    /** The machine, or null while the run has it out to call it. */
    var machine: ComponentMachine? = null
    var terminated = false
}

/** The state of one run, shared by the run and every component's context. */
internal class RunState(
    val configuration: Configuration,
    val factory: MachineFactory,
    val adapter: SystemAdapter,
    val guard: Duration?,
    val moduleParameters: ModuleParameters,
) {
    var now: Instant = 0
    private var created: Long = 0

    /** The entities, by name; a name's order here is not the creation order, [order] is. */
    private val entities = mutableMapOf<String, Entity>()
    private val order = mutableListOf<String>()
    private val done = mutableSetOf<String>()
    private var removedVerdict = Verdict.NONE
    private var mtcTerminated = false
    /** A dynamic error ended the run (P13). Not `error`, which would hide the standard library's. */
    private var failed = false
    private val log = mutableListOf<LogRecord>()
    private val records = mutableListOf<TestLogRecord>()

    /** A step of the run is inside the state ([access]). */
    var held = false

    // -- The round (§3.2, §12.4) --------------------------------------

    /**
     * Step 2, the update: inputs and timer expiries due at `NOW`. Then step 3,
     * the snapshot: every waiting component at once. Returns the components
     * that took it, in creation order, each with its serial.
     */
    fun updateAndSnapshot(): List<Pair<String, Long>> {
        for ((systemPort, message) in adapter.takeInputsDue(now)) {
            deliverInput(systemPort, message)
        }
        for (entity in entities.values) {
            for (timer in entity.state.timers.values) {
                timer.update(now)
            }
        }
        val waiting = mutableListOf<Pair<String, Long>>()
        for (name in order) {
            val entity = entities[name] ?: continue
            if (entity.terminated) {
                continue
            }
            for (port in entity.state.ports.values) {
                port.takeSnapshot()
            }
            for (timer in entity.state.timers.values) {
                timer.takeSnapshot()
            }
            entity.state.doneSnapshot = done.toSet()
            waiting.add(name to entity.serial)
        }
        return waiting
    }

    /**
     * The machine of a component that took this round's snapshot, out of its
     * entity to be delivered; null for one removed or terminated earlier in the
     * round, which does not select (§3.2 step 5).
     */
    fun takeForDelivery(name: String, serial: Long): ComponentMachine? {
        val entity = entities[name] ?: return null
        if (entity.serial != serial || entity.terminated) {
            return null
        }
        entity.state.branchTaken = false
        val machine = entity.machine
        entity.machine = null
        return machine
    }

    fun branchTaken(name: String, serial: Long): Boolean {
        val entity = entities[name] ?: return false
        return entity.serial == serial && entity.state.branchTaken
    }

    /**
     * No branch fired: time advances to the next instant something can happen
     * at (§3.3), or the test case can never end (P13).
     */
    fun advance() {
        var next: Instant? = adapter.nextInput()
        for (entity in entities.values) {
            if (entity.terminated) {
                continue
            }
            for (timer in entity.state.timers.values) {
                val expiry = timer.expiry ?: continue
                val earliest = next
                next = if (earliest == null) expiry else minOf(earliest, expiry)
            }
        }
        val guardAt = guard
        if (guardAt != null) {
            val earliest = next
            next = if (earliest == null) guardAt else minOf(earliest, guardAt)
        }
        val at = next
        if (at == null) {
            fail(MTC_NAME, "no branch can fire and no instant is pending: the test case can never end")
            return
        }
        now = maxOf(now, at)
        if (guardAt != null && now >= guardAt) {
            fail(MTC_NAME, "the guard of the test case was reached")
        }
    }

    /**
     * A component's machine has just run and is put back. If it terminated, P10
     * applies.
     *
     * The entity is still there: a component whose machine is running is not
     * removed ([kill]), and nothing else removes one mid-round.
     */
    fun settle(name: String, serial: Long, machine: ComponentMachine, terminated: Boolean) {
        val entity = entities[name]?.takeIf { it.serial == serial }
            ?: error("the component $name was removed while its machine ran")
        entity.machine = machine
        if (entity.terminated || !terminated) {
            return
        }
        entity.terminated = true
        trace(TestLogEvent.ComponentDone(name, entity.state.verdict))
        if (name == MTC_NAME) {
            mtcTerminated = true
            return
        }
        done.add(name)
        record(name, LogKind.DONE, "")
    }

    fun ended(): Boolean = mtcTerminated || failed

    /**
     * The MTC terminated or the run ended in error: every PTC still present is
     * removed, in ascending order of name as the other runtimes remove them, and
     * the test case verdict is the worst local verdict (P10, P14).
     */
    fun finish(): TestCaseResult {
        for (ptc in entities.keys.filter { it != MTC_NAME }.sorted()) {
            removeEntity(ptc)
        }
        var verdict = removedVerdict
        val mtc = entities[MTC_NAME]
        if (mtc != null) {
            verdict = worse(verdict, mtc.state.verdict)
        }
        if (failed) {
            verdict = Verdict.ERROR
        }
        trace(TestLogEvent.TestCaseEnded(verdict))
        return TestCaseResult(verdict = verdict, endedAt = now, log = log.toList(), records = records.toList())
    }

    // -- Entities (§2.1, P9) ------------------------------------------

    /**
     * Enters a new component of the type [componentType] into the table, with
     * its ports and timers, and logs its creation. Returns its serial.
     */
    fun enter(name: String, componentType: String): Long {
        created += 1
        val state = ComponentState()
        for (port in configuration.portsOfType[componentType].orEmpty()) {
            state.ports.getOrPut(port) { Port() }
        }
        for (timer in configuration.timersOfType[componentType].orEmpty()) {
            state.timers.getOrPut(timer) { Timer() }
        }
        entities[name] = Entity(created, state)
        order.add(name)
        val kind = if (name == MTC_NAME) LogKind.START else LogKind.CREATE
        record(name, kind, componentType)
        trace(TestLogEvent.ComponentCreated(name, componentType))
        return created
    }

    /**
     * The type of the PTC [ptc] that [creator] creates, or null, with the
     * dynamic error, when it cannot be created.
     */
    fun ptcToCreate(creator: String, ptc: String): String? {
        val componentType = configuration.ptcTypes[ptc]
        if (componentType == null) {
            fail(creator, "creates '$ptc', which the configuration does not declare")
            return null
        }
        if (entities.containsKey(ptc)) {
            fail(creator, "creates '$ptc', which is already present")
            return null
        }
        done.remove(ptc)
        return componentType
    }

    /**
     * The kill of the PTC [ptc] by [killer] (P9). A component whose machine is
     * running, up the call stack of this one, is not removed under it: that is a
     * dynamic error.
     */
    fun kill(killer: String, ptc: String) {
        val entity = entities[ptc]
        if (entity != null && entity.machine == null) {
            fail(killer, "kills '$ptc', whose machine is running")
            return
        }
        removeEntity(ptc)
    }

    /**
     * Removes a PTC: its local verdict still counts (-4 §9.29a.2), and its ports
     * are gone, so a send to one of them is an error.
     */
    private fun removeEntity(name: String) {
        val entity = entities.remove(name) ?: return
        removedVerdict = worse(removedVerdict, entity.state.verdict)
        record(name, LogKind.KILL, "")
        order.removeAll { it == name }
        done.remove(name)
    }

    // -- Ports (§2.2, P6) ---------------------------------------------

    fun send(sender: String, port: String, message: Message) {
        record(sender, LogKind.SEND, "$port ${message.event}")
        trace(TestLogEvent.MessageSent(sender, port, message.typeName, message.canonicalValue()))
        val peer = configuration.connections.firstNotNullOfOrNull { (a, b) ->
            when {
                a.names(sender, port) -> b
                b.names(sender, port) -> a
                else -> null
            }
        }
        if (peer != null) {
            val entity = entities[peer.component]
            if (entity != null) {
                entity.state.ports.getOrPut(peer.port) { Port() }.enqueue(message)
                return
            }
            fail(sender, "sends on '$port', whose peer '${peer.component}' is not present")
            return
        }
        val mapped = configuration.mappings.firstOrNull { (endpoint, _) -> endpoint.names(sender, port) }
        if (mapped != null) {
            adapter.send(mapped.second, message, now)
            return
        }
        fail(sender, "sends on '$port', which is neither connected nor mapped")
    }

    private fun deliverInput(systemPort: String, message: Message) {
        val endpoint = configuration.mappings.firstOrNull { (_, mapped) -> mapped == systemPort }?.first
        if (endpoint == null) {
            fail(MTC_NAME, "an input on system port '$systemPort', which no port maps")
            return
        }
        val entity = entities[endpoint.component]
        if (entity != null) {
            entity.state.ports.getOrPut(endpoint.port) { Port() }.enqueue(message)
            return
        }
        fail(endpoint.component, "an input on system port '$systemPort' found no component")
    }

    // -- A component's own state, through its context -----------------

    fun component(name: String, serial: Long): ComponentState =
        entities[name]?.takeIf { it.serial == serial }?.state
            ?: error("the component $name was removed, and its context outlived it")

    fun receive(name: String, serial: Long, port: String): Message {
        val component = component(name, serial)
        component.branchTaken = true
        val queue = component.ports[port]
            ?: error("$name receives on '$port', which its component type does not declare")
        val message = queue.removeHead()
        record(name, LogKind.RECEIVE, "$port ${message.event}")
        trace(TestLogEvent.MessageReceived(name, port, message.typeName, message.canonicalValue()))
        return message
    }

    fun timeout(name: String, serial: Long, timer: String) {
        val component = component(name, serial)
        component.branchTaken = true
        val binding = component.timers[timer]
            ?: error("$name times out on '$timer', which its component type does not declare")
        binding.timeoutTaken()
        record(name, LogKind.TIMEOUT, timer)
        trace(TestLogEvent.TimerTimedOut(name, timer))
    }

    fun setverdict(name: String, serial: Long, verdict: Verdict, reason: String) {
        val component = component(name, serial)
        component.verdict = worse(component.verdict, verdict)
        val detail = if (reason.isEmpty()) verdict.spelling else "${verdict.spelling} $reason"
        record(name, LogKind.VERDICT, detail)
        trace(TestLogEvent.VerdictSet(name, verdict, reason.ifEmpty { null }))
    }

    // -- The verdict and the log (§5, §12.5) --------------------------

    /**
     * A dynamic error (P13): the run ends with `error`. The test log records
     * the verdict without [why], which is this runtime's own prose.
     */
    fun fail(component: String, why: String) {
        failed = true
        record(component, LogKind.ERROR, why)
        trace(TestLogEvent.VerdictSet(component, Verdict.ERROR, null))
    }

    fun record(component: String, kind: LogKind, detail: String) {
        log.add(LogRecord(at = now, component = component, kind = kind, detail = detail))
    }

    /** A record of the test log, at `NOW`. */
    fun trace(event: TestLogEvent) {
        records.add(TestLogRecord(at = now, event = event))
    }
}

/**
 * [step] on the state, for [component], with the state held for its length; a
 * call that finds it held already is re-entrant.
 */
private fun <R> RunState.access(component: String, step: (RunState) -> R): R {
    check(!held) { "re-entrant access to the test run from $component" }
    held = true
    try {
        return step(this)
    } finally {
        held = false
    }
}

/**
 * Creates the component [name] of the type [componentType] and starts its
 * machine, with the state let go while the factory and the machine run.
 */
private fun createEntity(run: RunState, name: String, componentType: String) {
    val machine = run.factory(componentType)
    if (machine == null) {
        run.access(name) {
            it.fail(
                name,
                "is of the component type '$componentType', which the factory of machines does not make",
            )
        }
        return
    }
    val serial = run.access(name) { it.enter(name, componentType) }
    machine.start(ComponentContext(run, name, serial))
    val terminated = machine.terminated()
    run.access(name) { it.settle(name, serial, machine, terminated) }
}

/** One round (§3.2, §12.4). */
private fun round(run: RunState) {
    val waiting = run.access(MTC_NAME) { it.updateAndSnapshot() }
    // Steps 4 and 5: one component at a time, in creation order, each selects
    // on its own snapshot and runs the macrostep of the branch it took. A
    // component removed or terminated earlier in the round does not select, and
    // nothing does once the MTC has terminated (§3.2).
    var anyTaken = false
    for ((name, serial) in waiting) {
        if (run.access(name) { it.ended() }) {
            break
        }
        val machine = run.access(name) { it.takeForDelivery(name, serial) } ?: continue
        machine.deliverSnapshot()
        val terminated = machine.terminated()
        val taken = run.access(name) {
            val branch = it.branchTaken(name, serial)
            it.settle(name, serial, machine, terminated)
            branch
        }
        anyTaken = anyTaken || taken
    }
    run.access(MTC_NAME) {
        if (!it.ended() && !anyTaken) {
            it.advance()
        }
    }
}

/**
 * What a component's machine reads and does through the run (§12.3).
 *
 * The machine keeps it from `start` on, and so may the host processors the
 * machine registers for the doors of a send and of a PTC. Each call takes the
 * run's state for its own length and lets it go before it returns, so no call
 * holds it across a call into a machine. A call made while the state is held,
 * from inside a snapshot reader say, is re-entrant and throws an
 * IllegalStateException naming the component.
 */
class ComponentContext internal constructor(
    private val run: RunState,
    /** The component's name: `mtc`, or the PTC's id. */
    val name: String,
    private val serial: Long,
) {
    private fun <R> access(step: (RunState) -> R): R = run.access(name, step)

    override fun toString(): String = "ComponentContext(name=$name, serial=$serial, ..)"

    // -- The snapshot, read by the branches' guards (P2) --------------

    /**
     * Hands [read] the message the snapshot saw at the head of [port], or null,
     * and returns what it returns. [read] runs while the run's state is held, so
     * it must not call this context: such a call throws.
     */
    fun <R> withSnapshot(port: String, read: (Message?) -> R): R = access {
        read(it.component(name, serial).ports[port]?.snapshot())
    }

    /** The snapshot saw a message at the head of [port]: the guard of a receive any branch. */
    fun snapshotPresent(port: String): Boolean = withSnapshot(port) { head -> head != null }

    /** The snapshot saw [timer] timed out. */
    fun timedOut(timer: String): Boolean = access {
        it.component(name, serial).timers[timer]?.snapshotTimedOut == true
    }

    /** The snapshot saw the PTC [ptc] in `DONE`. */
    fun done(ptc: String): Boolean = access { ptc in it.component(name, serial).doneSnapshot }

    // -- Taking a branch (P4 step 1), the first statement of each -----

    /**
     * A receive branch: remove the head of [port], which its content reads as
     * `_event.data`.
     *
     * @throws IllegalStateException when the component type declares no port
     *   [port], or its queue is empty: a receive branch is taken only on a
     *   snapshot that saw a message there.
     */
    fun receive(port: String): Message = access { it.receive(name, serial, port) }

    /**
     * A timeout branch: [timer] becomes idle.
     *
     * @throws IllegalStateException when the component type declares no timer [timer].
     */
    fun timeout(timer: String) {
        access { it.timeout(name, serial, timer) }
    }

    /**
     * A done branch on the PTC [ptc]: nothing is removed (-4 §9.16); the
     * component took a branch in this round.
     */
    @Suppress("UNUSED_PARAMETER")
    fun doneTaken(ptc: String) {
        access { it.component(name, serial).branchTaken = true }
    }

    // -- Statements of the profile ------------------------------------

    /**
     * `<sce:setverdict>` (P12): the local verdict becomes the worse of the two,
     * and the verdict and its reason are logged.
     */
    fun setverdict(verdict: Verdict, reason: String) {
        access { it.setverdict(name, serial, verdict, reason) }
    }

    /** `<sce:start-timer>` (P7), for [duration] microseconds from `NOW`. */
    fun startTimer(timer: String, duration: Duration) {
        access {
            it.component(name, serial).timers.getOrPut(timer) { Timer() }.start(it.now, duration)
            it.trace(TestLogEvent.TimerStarted(name, timer, duration))
        }
    }

    /**
     * `<sce:stop-timer>` (P7). The test log records the stop of a running
     * timer; the stop of one that is not running changes nothing.
     */
    fun stopTimer(timer: String) {
        access {
            val binding = it.component(name, serial).timers.getOrPut(timer) { Timer() }
            val running = binding.status == TimerStatus.RUNNING
            binding.stop()
            if (running) {
                it.trace(TestLogEvent.TimerStopped(name, timer))
            }
        }
    }

    /** A `<log>` (P8): recorded in the test log with `NOW` and the component. */
    fun log(text: String) {
        access {
            it.record(name, LogKind.LOG, text)
            it.trace(TestLogEvent.Log(name, text))
        }
    }

    // -- Doors (§12.3) ------------------------------------------------

    /**
     * A send on [port] (P6): to the queue of the connected port, or to the
     * system adapter when the port is mapped.
     */
    fun send(port: String, message: Message) {
        access { it.send(name, port, message) }
    }

    /**
     * The start of the invocation that creates the PTC [ptc] (P9): the PTC is
     * created and its machine started before this returns.
     */
    fun create(ptc: String) {
        val componentType = access { it.ptcToCreate(name, ptc) } ?: return
        createEntity(run, ptc, componentType)
    }

    /** The cancel of that invocation: the PTC is removed (P9). */
    fun kill(ptc: String) {
        access { it.kill(name, ptc) }
    }

    // -- Module parameters (§11.5, §11.8) -----------------------------

    /**
     * Hands [read] the module parameters of the run, which it reads into the
     * variables the component declares as module parameters, before the
     * component's machine starts. A value the run was not given
     * ([ModuleParameters.of]), or one [read] refuses as not of the type the
     * component declares, throws a [StateRefusal], which is a dynamic error
     * (P13): the run ends with `error` and false is returned, so the machine is
     * not started on a value made up in its place. [read] runs with the run's
     * state let go.
     */
    fun readModuleParameters(read: (ModuleParameters) -> Unit): Boolean {
        val given = access { it.moduleParameters.copy() }
        return try {
            read(given)
            true
        } catch (refused: StateRefusal) {
            access { it.fail(name, refused.message.orEmpty()) }
            false
        }
    }

    /** `NOW`. */
    fun now(): Instant = access { it.now }
}

/**
 * Whether a receive branch with a template matches the snapshot of [context]
 * (P2, docs/SCE_TEST_PROFILE.md §12.3).
 *
 * The snapshot saw a message at the head of [port], under [event], the event
 * name of the template's record, whose value is a [V], and [matches], the
 * template's `match` bound to its arguments, holds of that value. A message of
 * another record, or one whose value is of another type, does not match.
 * [matches] runs while the run's state is held, as a reader of
 * [ComponentContext.withSnapshot] does.
 */
inline fun <reified V : Any> receives(
    context: ComponentContext,
    port: String,
    event: String,
    crossinline matches: (V) -> Boolean,
): Boolean = context.withSnapshot(port) { head ->
    val value = if (head != null && head.event == event) head.valueAs<V>() else null
    value != null && matches(value)
}

/**
 * One run of one test case (§2.3, §3, §4.9 to §4.11, §5).
 *
 * The components run on the calling thread, in the order they were created;
 * §3.2 shows that the order within a round does not change the result.
 *
 * [guard] is the test case's guard duration (P11), if it has one, and
 * [moduleParameters] the values of the module parameters the components read
 * (§11.8): the suite's, with any the caller overrides. They are copied, so a
 * later change to the caller's leaves the run as it was built.
 */
class TestCaseRun(
    configuration: Configuration,
    factory: MachineFactory,
    adapter: SystemAdapter,
    guard: Duration?,
    moduleParameters: ModuleParameters,
) {
    private val state = RunState(configuration, factory, adapter, guard, moduleParameters.copy())

    // The Rust run is consumed by its `run`; a Kotlin object cannot be, so a
    // second run of it, whose adapter has already been read, is refused.
    private var started = false

    /**
     * Runs the test case to its end and returns its verdict and logs.
     *
     * @throws IllegalStateException when this run has been run already.
     */
    fun run(): TestCaseResult {
        check(!started) { "a test case run is run once; build another to run the test case again" }
        started = true
        state.access(MTC_NAME) { it.trace(TestLogEvent.TestCaseStarted) }
        createEntity(state, MTC_NAME, state.configuration.mtcType)
        while (!state.access(MTC_NAME) { it.ended() }) {
            round(state)
        }
        return state.access(MTC_NAME) { it.finish() }
    }
}
