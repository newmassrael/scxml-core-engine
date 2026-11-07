// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test profile's runtime (docs/SCE_TEST_PROFILE.md §12.2), held to the
// rules of §2 to §5 with components written here by hand, before any machine is
// generated: each one keeps its context from `start` on, reads its snapshot and
// calls its context as a generated machine does (§12.3). The runner of a suite
// is held to P11 over test cases made of the same components.
//
// Each class and case is the twin of the Rust module and case of the same name
// in backends/rust/runtime/tests/a_test_case_runs_as_the_test_profile_says.rs
// (`test_case_run_test::an_answered_request_...` is
// `TestCaseRunTest.an answered request ...`), itself the twin of the C++ gtest in
// tests/test_runtime/TestCaseRunTest.cpp, with the same scenario, verdict, `NOW`
// and log. The cases of `TheSharedHandleTest` hold what the Rust runtime's
// handle holds, which the Kotlin context holds too: re-entrant use is refused,
// and a context outlives no component it names. `TheRunIsRunOnceTest` holds what
// only the Kotlin runtime has to, since its run is not consumed by running it.
// The case of `TheDoorsTest` reaches the run only through the two host
// processors a generated component registers on its engine (§12.3), written
// here with the types the engine registers them under.

package com.sce.runtime.test

import com.sce.runtime.SavedValues
import com.sce.runtime.StateMachineEngine
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

private const val SECOND: Long = 1_000_000L

/** What a hand-written component does when it starts, or on a snapshot. */
private typealias Step = (ComponentContext, Scripted) -> Unit

private fun idle(): Step = { _, _ -> }

/**
 * A component written by hand: what it does when it starts, and what it does on
 * each snapshot. Either may end it, as entering a top-level final does.
 */
private class Scripted(private val onStart: Step, private val onSnapshot: Step) : ComponentMachine {
    private var context: ComponentContext? = null
    private var ended = false

    fun end() {
        ended = true
    }

    override fun start(context: ComponentContext) {
        this.context = context
        onStart(context, this)
    }

    override fun deliverSnapshot() {
        val started = checkNotNull(this.context) { "a snapshot is delivered only to a started machine" }
        onSnapshot(started, this)
    }

    override fun terminated(): Boolean = ended
}

/** The machines of a test, by component type. */
private class Machines {
    private val types = mutableMapOf<String, Pair<Step, Step>>()

    fun with(componentType: String, steps: Pair<Step, Step>): Machines {
        types[componentType] = steps
        return this
    }

    fun factory(): MachineFactory {
        val known = types.toMap()
        return { componentType ->
            known[componentType]?.let { (onStart, onSnapshot) -> Scripted(onStart, onSnapshot) }
        }
    }
}

/** One input of a trace, or one send the adapter was handed. */
private data class Input(val at: Instant, val systemPort: String, val event: String)

/** A system adapter that replays a trace and records what was sent to it. */
private class TraceAdapter(private val trace: List<Input> = emptyList()) : SystemAdapter {
    private var next = 0
    val sent = mutableListOf<Input>()

    override fun send(systemPort: String, message: Message, now: Instant) {
        sent.add(Input(now, systemPort, message.event))
    }

    override fun nextInput(): Instant? = trace.getOrNull(next)?.at

    override fun takeInputsDue(now: Instant): List<Pair<String, Message>> {
        val due = mutableListOf<Pair<String, Message>>()
        while (next < trace.size && trace[next].at <= now) {
            val input = trace[next]
            due.add(input.systemPort to message(input.event))
            next += 1
        }
        return due
    }
}

private fun message(event: String): Message = Message(event, Unit)

private fun logged(result: TestCaseResult, component: String, kind: LogKind): Boolean =
    result.log.any { it.component == component && it.kind == kind }

/** The ids each component type declares, from `(type, id)` pairs. */
private fun byType(vararg entries: Pair<String, String>): Map<String, List<String>> =
    entries.groupBy({ it.first }, { it.second })

/** An MTC and one PTC joined by one connection, the MTC's `p` to the PTC's `q`. */
private fun mtcAndEcho(): Configuration = Configuration(
    mtcType = "Tester",
    ptcTypes = mapOf("echo" to "Echo"),
    portsOfType = byType("Tester" to "p", "Echo" to "q"),
    timersOfType = byType("Tester" to "reply"),
    connections = listOf(Endpoint("mtc", "p") to Endpoint("echo", "q")),
)

/**
 * An MTC alone, of the type [componentType], whose port `p` is mapped to the
 * system port `sut`.
 */
private fun mapped(componentType: String): Configuration = Configuration(
    mtcType = componentType,
    portsOfType = byType(componentType to "p"),
    mappings = listOf(Endpoint("mtc", "p") to "sut"),
)

/** An MTC alone, of the type [componentType], with no port and no timer. */
private fun lone(componentType: String): Configuration = Configuration(mtcType = componentType)

/**
 * The MTC creates the echo, sends to it, and waits for the answer within a
 * second: a receive branch passes, a timeout branch fails.
 */
private fun tester(): Pair<Step, Step> = Pair(
    { context, _ ->
        context.create("echo")
        context.send("p", message("ping"))
        context.startTimer("reply", SECOND)
    },
    repliesInTime(),
)

private fun repliesInTime(): Step = { context, self ->
    if (context.snapshotPresent("p")) {
        assertEquals("pong", context.receive("p").event)
        context.stopTimer("reply")
        context.setverdict(Verdict.PASS, "")
        self.end()
    } else if (context.timedOut("reply")) {
        context.timeout("reply")
        context.setverdict(Verdict.FAIL, "no reply in time")
        self.end()
    }
}

private fun runCase(
    configuration: Configuration,
    machines: Machines,
    adapter: SystemAdapter,
    guard: Duration?,
): TestCaseResult = runCaseWith(configuration, machines, adapter, guard, ModuleParameters())

private fun runCaseWith(
    configuration: Configuration,
    machines: Machines,
    adapter: SystemAdapter,
    guard: Duration?,
    parameters: ModuleParameters,
): TestCaseResult = TestCaseRun(configuration, machines.factory(), adapter, guard, parameters).run()

/** A `uint32` module parameter's value, as a generated suite writes its default. */
private fun saved(value: UInt): ModuleParameterValue = SavedValues.of(value)

/** What a component read of a module parameter, if it read one. */
private class Slot {
    var value: UInt? = null
}

/**
 * §11.5, §11.8: a component reads the module parameters of its run into its
 * variables before it starts, as a generated machine reads each through the
 * reader of the type it declares.
 */
private fun readsJob(job: Slot): Pair<Step, Step> = Pair(
    { context, self ->
        val read = context.readModuleParameters { given ->
            job.value = SavedValues.uint32(given.of("job"), "job")
        }
        if (read) {
            context.setverdict(Verdict.PASS, "")
            self.end()
        }
    },
    idle(),
)

class TestVerdictTest {

    @Test
    fun `only ever becomes worse`() {
        assertEquals(Verdict.PASS, worse(Verdict.PASS, Verdict.NONE))
        assertEquals(Verdict.INCONCLUSIVE, worse(Verdict.PASS, Verdict.INCONCLUSIVE))
        assertEquals(Verdict.FAIL, worse(Verdict.FAIL, Verdict.PASS))
        assertEquals(Verdict.ERROR, worse(Verdict.FAIL, Verdict.ERROR))
    }
}

class TestPortTest {

    @Test
    fun `a snapshot sees only what was queued when it was taken`() {
        val port = Port()
        port.takeSnapshot()
        port.enqueue(message("late"))
        assertNull(port.snapshot(), "a message that arrives after the snapshot is not in it")
        port.takeSnapshot()
        assertNotNull(port.snapshot())
        port.enqueue(message("behind"))
        assertEquals("late", port.snapshot()?.event, "a later message goes behind the head")
        assertEquals("late", port.removeHead().event)
        assertEquals(1, port.size)
    }
}

class TestTimerTest {

    @Test
    fun `times out at its instant and restarts when started again`() {
        val timer = Timer()
        timer.start(0, 10)
        timer.update(9)
        assertEquals(TimerStatus.RUNNING, timer.status)
        timer.start(5, 10)
        assertEquals(15L, timer.expiry, "a running timer that is started again restarts")
        timer.update(15)
        assertEquals(TimerStatus.TIMEOUT, timer.status)
        timer.takeSnapshot()
        assertTrue(timer.snapshotTimedOut)
        timer.timeoutTaken()
        assertEquals(TimerStatus.IDLE, timer.status)
        timer.stop()
        assertEquals(TimerStatus.IDLE, timer.status, "stopping a timer that is not running does nothing")
    }
}

/**
 * The SUT's four messages, all at `NOW = 0`: a record of another type, another
 * event, another value, and the one the template matches.
 */
private class FourInputs : SystemAdapter {
    private var given = false

    override fun send(systemPort: String, message: Message, now: Instant) {}

    override fun nextInput(): Instant? = null

    override fun takeInputsDue(now: Instant): List<Pair<String, Message>> {
        if (now != 0L || given) {
            return emptyList()
        }
        given = true
        return listOf(
            "sut" to Message("number", "7"),
            "sut" to Message("other", 7),
            "sut" to Message("number", 6),
            "sut" to Message("number", 7),
        )
    }
}

class TestCaseRunTest {

    /**
     * P6, P9, §3.2: a message sent in a round is taken in a later round, and the
     * whole exchange happens at one instant, because a branch fired in each round.
     */
    @Test
    fun `an answered request passes without time moving`() {
        val machines = Machines().with("Tester", tester()).with(
            "Echo",
            Pair(idle()) { context, _ ->
                if (context.snapshotPresent("q")) {
                    assertEquals("ping", context.receive("q").event)
                    context.send("q", message("pong"))
                }
            },
        )
        val result = runCase(mtcAndEcho(), machines, TraceAdapter(), null)
        assertEquals(Verdict.PASS, result.verdict)
        assertEquals(0L, result.endedAt)
        assertTrue(logged(result, "echo", LogKind.KILL), "the MTC's end removes the PTC (P10)")
    }

    /** P7, §3.3: with nothing to receive, time advances to the timer's instant. */
    @Test
    fun `an unanswered request fails when its timer expires`() {
        val machines = Machines().with("Tester", tester()).with("Echo", Pair(idle(), idle()))
        val result = runCase(mtcAndEcho(), machines, TraceAdapter(), null)
        assertEquals(Verdict.FAIL, result.verdict)
        assertEquals(SECOND, result.endedAt)
    }

    /**
     * P13: no branch can fire and no instant is pending, so the test case can
     * never end; the runtime ends it with error.
     */
    @Test
    fun `a test case that can never end is an error`() {
        val machines = Machines().with("Waiter", Pair(idle(), idle()))
        val result = runCase(mapped("Waiter"), machines, TraceAdapter(), null)
        assertEquals(Verdict.ERROR, result.verdict)
        assertTrue(logged(result, "mtc", LogKind.ERROR))
    }

    /** P11: the guard instant ends a test case that has not ended, with error. */
    @Test
    fun `the guard ends a test case with error`() {
        val machines = Machines().with(
            "Waiter",
            Pair({ context, _ -> context.setverdict(Verdict.PASS, "") }, idle()),
        )
        val result = runCase(mapped("Waiter"), machines, TraceAdapter(), 5 * SECOND)
        assertEquals(Verdict.ERROR, result.verdict)
        assertEquals(5 * SECOND, result.endedAt)
    }

    /**
     * §2.2, §12.2: a mapped port's sends reach the adapter, and the trace's
     * inputs reach the port at their instants.
     */
    @Test
    fun `a mapped port talks to the system adapter on the logical clock`() {
        val configuration = mapped("Tester").copy(timersOfType = byType("Tester" to "reply"))
        val machines = Machines().with(
            "Tester",
            Pair(
                { context, _ ->
                    context.send("p", message("ping"))
                    context.startTimer("reply", SECOND)
                },
                repliesInTime(),
            ),
        )
        val adapter = TraceAdapter(listOf(Input(500, "sut", "pong")))
        val result = runCase(configuration, machines, adapter, null)
        assertEquals(Verdict.PASS, result.verdict)
        assertEquals(500L, result.endedAt)
        assertEquals(1, adapter.sent.size)
        assertEquals("sut", adapter.sent[0].systemPort)
        assertEquals("ping", adapter.sent[0].event)
    }

    /** P9, P14: a removed PTC's verdict still counts in the test case verdict. */
    @Test
    fun `a removed ptcs verdict still counts`() {
        val machines = Machines()
            .with(
                "Tester",
                Pair(
                    { context, _ ->
                        context.create("echo")
                        context.startTimer("reply", 10)
                    },
                    { context, self ->
                        if (context.timedOut("reply")) {
                            context.timeout("reply")
                            context.kill("echo")
                            context.setverdict(Verdict.PASS, "")
                            self.end()
                        }
                    },
                ),
            )
            .with("Echo", Pair({ context, _ -> context.setverdict(Verdict.FAIL, "") }, idle()))
        val result = runCase(mtcAndEcho(), machines, TraceAdapter(), null)
        assertEquals(Verdict.FAIL, result.verdict)
    }

    /**
     * §3.2 step 4: components select one at a time in creation order, so when the
     * MTC terminates in a round, a PTC that would have selected after it in that
     * round does not.
     */
    @Test
    fun `nothing selects after the mtc terminates in the round`() {
        val base = mtcAndEcho()
        val configuration = base.copy(timersOfType = base.timersOfType + ("Echo" to listOf("late")))
        val machines = Machines()
            .with(
                "Tester",
                Pair(
                    { context, _ ->
                        context.create("echo")
                        context.startTimer("reply", 10)
                    },
                    { context, self ->
                        if (context.timedOut("reply")) {
                            context.timeout("reply")
                            context.setverdict(Verdict.PASS, "")
                            self.end()
                        }
                    },
                ),
            )
            .with(
                "Echo",
                Pair(
                    { context, _ -> context.startTimer("late", 10) },
                    { context, _ ->
                        if (context.timedOut("late")) {
                            context.timeout("late")
                            context.setverdict(Verdict.FAIL, "")
                        }
                    },
                ),
            )
        val result = runCase(configuration, machines, TraceAdapter(), null)
        assertEquals(Verdict.PASS, result.verdict)
        assertFalse(logged(result, "echo", LogKind.TIMEOUT))
    }

    /**
     * P2, P10: a PTC that terminates is in DONE from the next snapshot on, and a
     * done branch removes nothing.
     */
    @Test
    fun `a done branch sees a ptc that terminated`() {
        val machines = Machines()
            .with(
                "Tester",
                Pair(
                    { context, _ -> context.create("echo") },
                    { context, self ->
                        if (context.done("echo")) {
                            context.doneTaken("echo")
                            context.kill("echo")
                            self.end()
                        }
                    },
                ),
            )
            .with(
                "Echo",
                Pair(
                    { context, self ->
                        context.setverdict(Verdict.PASS, "")
                        self.end()
                    },
                    idle(),
                ),
            )
        val result = runCase(mtcAndEcho(), machines, TraceAdapter(), null)
        assertEquals(Verdict.PASS, result.verdict)
        assertTrue(logged(result, "echo", LogKind.DONE))
    }

    /**
     * P2, §12.3: a receive branch with a template matches only a message of its
     * record, under its record's event, whose value its `match` holds; the guard
     * reads the snapshot and removes nothing.
     */
    @Test
    fun `a receive with a template matches only its records value`() {
        val seen = mutableListOf<String>()
        val isSeven: (Int) -> Boolean = { it == 7 }
        val machines = Machines().with(
            "Reader",
            Pair(idle()) { context, self ->
                if (receives<Int>(context, "p", "number", isSeven)) {
                    seen.add("seven")
                    assertTrue(
                        receives<Int>(context, "p", "number", isSeven),
                        "a guard that matched removed the message",
                    )
                    context.receive("p")
                    self.end()
                } else if (context.snapshotPresent("p")) {
                    val event = context.withSnapshot("p") { head -> head?.event }
                    if (event != null) {
                        seen.add(event)
                    }
                    context.receive("p")
                }
            },
        )
        val result = runCase(mapped("Reader"), machines, FourInputs(), null)
        assertEquals(
            listOf("number", "other", "number", "seven"),
            seen,
            "a value of another type, another event and another value do not match",
        )
        assertEquals(Verdict.NONE, result.verdict)
    }

    /** §11.8: the value the run is given is the one the component reads, and the component starts on it. */
    @Test
    fun `a component reads the module parameter its run is given`() {
        val job = Slot()
        val machines = Machines().with("Reader", readsJob(job))
        val parameters = ModuleParameters()
        parameters.set("job", saved(7u))
        val result = runCaseWith(lone("Reader"), machines, TraceAdapter(), null, parameters)
        assertEquals(7u, job.value)
        assertEquals(Verdict.PASS, result.verdict)
    }

    /**
     * P13: a module parameter the run has no value for is a dynamic error, named
     * in the log, and the component goes no further than the read.
     */
    @Test
    fun `a module parameter the run is not given is an error`() {
        val job = Slot()
        val machines = Machines().with("Reader", readsJob(job))
        val result = runCase(lone("Reader"), machines, TraceAdapter(), null)
        assertEquals(Verdict.ERROR, result.verdict)
        assertNull(job.value)
        val last = assertNotNull(result.log.lastOrNull(), "the run logged its error")
        assertEquals(LogKind.ERROR, last.kind)
        assertTrue(last.detail.contains("'job'"), last.detail)
    }

    /**
     * P13: a value that is not of the type the component declares is refused as a
     * restored value of another type is, and not converted.
     */
    @Test
    fun `a module parameter of another type is an error`() {
        val job = Slot()
        val machines = Machines().with("Reader", readsJob(job))
        val parameters = ModuleParameters()
        parameters.set("job", SavedValues.of("seven"))
        val result = runCaseWith(lone("Reader"), machines, TraceAdapter(), null, parameters)
        assertEquals(Verdict.ERROR, result.verdict)
        assertNull(job.value)
        assertTrue(logged(result, "mtc", LogKind.ERROR))
    }

    /**
     * P13: a component type the factory of machines makes no machine for ends the
     * run with error where a component of it is created.
     */
    @Test
    fun `a component type with no machine is an error`() {
        val none: MachineFactory = { _ -> null }
        val result = TestCaseRun(lone("Ghost"), none, TraceAdapter(), null, ModuleParameters()).run()
        assertEquals(Verdict.ERROR, result.verdict)
        assertTrue(logged(result, "mtc", LogKind.ERROR))
        assertFalse(logged(result, "mtc", LogKind.START), "no component of the type was created")
    }

    /** P6, P13: a send on a port whose peer is not present is a dynamic error. */
    @Test
    fun `a send to an absent peer is an error`() {
        val machines = Machines().with(
            "Tester",
            Pair({ context, _ -> context.send("p", message("ping")) }, idle()),
        )
        val result = runCase(mtcAndEcho(), machines, TraceAdapter(), null)
        assertEquals(Verdict.ERROR, result.verdict)
        assertEquals(0L, result.endedAt)
    }
}

/** A test case whose MTC, of type [componentType], sets [verdict] as it starts and ends there. */
private fun endsWith(name: String, componentType: String, verdict: Verdict): TestCase {
    val machines = Machines().with(
        componentType,
        Pair(
            { context, self ->
                context.setverdict(verdict, "")
                self.end()
            },
            idle(),
        ),
    )
    return TestCase(
        name = name,
        configuration = lone(componentType),
        machines = machines.factory(),
        moduleParameters = emptyList(),
    )
}

/** The adapters of a suite's run, each a fresh TraceAdapter, recording the test case each was built for. */
private fun tracing(built: MutableList<String>): (String) -> SystemAdapter? = { testCase ->
    built.add(testCase)
    TraceAdapter()
}

private val selectsX: Selection = { world -> enabled(world, "x") }

private val selectsY: Selection = { world -> enabled(world, "y") }

private fun world(vararg features: String): Features = features.toSet()

private fun execute(testCase: TestCase, guard: Duration?): Execute = Execute(testCase, guard, null)

class TestSuiteRunTest {

    /**
     * P11: the runner runs the test cases a world selects, in the suite's order,
     * and skips the others; a test case with no selection runs in every world.
     */
    @Test
    fun `runs the test cases a world selects in the suites order`() {
        val suite = Suite(
            name = "ordered",
            moduleParameters = ModuleParameters(),
            executes = listOf(
                execute(endsWith("first", "First", Verdict.PASS), null),
                execute(endsWith("second", "Second", Verdict.FAIL), null).copy(selection = selectsX),
                execute(endsWith("third", "Third", Verdict.INCONCLUSIVE), null).copy(selection = selectsY),
            ),
        )
        val built = mutableListOf<String>()
        val results = runSuite(suite, world("y"), tracing(built), ModuleParameters())
        assertEquals(2, results.size)
        assertEquals("first", results[0].testCase)
        assertEquals(Verdict.PASS, results[0].result.verdict)
        assertEquals("third", results[1].testCase)
        assertEquals(Verdict.INCONCLUSIVE, results[1].result.verdict)
        assertEquals(listOf("first", "third"), built, "an adapter is built for each case run")
    }

    /** P11: each test case is a fresh run from NOW = 0 under its own guard. */
    @Test
    fun `each test case is a fresh run under its own guard`() {
        val waiting = Machines().with("Waiter", Pair(idle(), idle()))
        val stuck = TestCase(
            name = "stuck",
            configuration = lone("Waiter"),
            machines = waiting.factory(),
            moduleParameters = emptyList(),
        )
        val suite = Suite(
            name = "guarded",
            moduleParameters = ModuleParameters(),
            executes = listOf(
                execute(stuck, 5 * SECOND),
                execute(endsWith("quick", "Quick", Verdict.PASS), null),
            ),
        )
        val built = mutableListOf<String>()
        val results = runSuite(suite, emptySet(), tracing(built), ModuleParameters())
        assertEquals(2, results.size)
        assertEquals(Verdict.ERROR, results[0].result.verdict)
        assertEquals(5 * SECOND, results[0].result.endedAt, "the guard ends the first")
        assertEquals(Verdict.PASS, results[1].result.verdict)
        assertEquals(0L, results[1].result.endedAt, "the second starts again at NOW = 0")
    }

    /**
     * -1 §8.2.1: a value the caller gives a module parameter replaces the suite's
     * default, and a parameter the suite does not declare cannot be given one.
     */
    @Test
    fun `an override replaces the default of a module parameter the suite declares`() {
        val job = Slot()
        val machines = Machines().with("Reader", readsJob(job))
        val defaults = ModuleParameters()
        defaults.set("job", saved(7u))
        val suite = Suite(
            name = "parameters",
            moduleParameters = defaults,
            executes = listOf(
                execute(
                    TestCase(
                        name = "reads",
                        configuration = lone("Reader"),
                        machines = machines.factory(),
                        moduleParameters = listOf("job"),
                    ),
                    null,
                ),
            ),
        )
        val built = mutableListOf<String>()
        val results = runSuite(suite, emptySet(), tracing(built), ModuleParameters())
        assertEquals(1, results.size)
        assertEquals(Verdict.PASS, results[0].result.verdict)
        assertEquals(7u, job.value, "the default")

        val overrides = ModuleParameters()
        overrides.set("job", saved(9u))
        val overridden = runSuite(suite, emptySet(), tracing(built), overrides)
        assertEquals(1, overridden.size)
        assertEquals(9u, job.value, "the override")

        val stray = ModuleParameters()
        stray.set("limit", saved(1u))
        val refused = assertFailsWith<OverrideRefusal>("an override of an undeclared parameter is refused") {
            runSuite(suite, emptySet(), tracing(built), stray)
        }
        assertTrue(refused.reason.contains("'limit'"), refused.reason)
    }

    /**
     * P13: a test case whose components read a module parameter the run has no
     * value for ends with error before anything of it runs, its adapter included.
     */
    @Test
    fun `a test case that reads a module parameter with no value is an error`() {
        val reads = endsWith("reads", "Reader", Verdict.PASS).copy(moduleParameters = listOf("job"))
        val suite = Suite(
            name = "undeclared",
            moduleParameters = ModuleParameters(),
            executes = listOf(execute(reads, null)),
        )
        val built = mutableListOf<String>()
        val results = runSuite(suite, emptySet(), tracing(built), ModuleParameters())
        assertEquals(1, results.size)
        assertEquals(Verdict.ERROR, results[0].result.verdict)
        assertEquals(1, results[0].result.log.size)
        assertTrue(results[0].result.log[0].detail.contains("'job'"))
        assertTrue(built.isEmpty(), "no adapter is built for a case that cannot start")
    }

    /** P13: a test case the caller gives no system adapter ends with error. */
    @Test
    fun `a test case with no system adapter is an error`() {
        val suite = Suite(
            name = "unadapted",
            moduleParameters = ModuleParameters(),
            executes = listOf(execute(endsWith("only", "Only", Verdict.PASS), null)),
        )
        val results = runSuite(suite, emptySet(), { _ -> null }, ModuleParameters())
        assertEquals(1, results.size)
        assertEquals(Verdict.ERROR, results[0].result.verdict)
    }
}

class TheSharedHandleTest {

    /**
     * A snapshot reader runs while the run's state is held, so a call it makes
     * into its context is re-entrant: it throws, naming the component, and does
     * not go on inside the state.
     */
    @Test
    fun `a call into the context from a snapshot reader panics`() {
        val machines = Machines().with(
            "Reader",
            Pair({ context, _ -> context.withSnapshot("p") { context.log("inside the reader") } }, idle()),
        )
        val failure = assertFailsWith<IllegalStateException> {
            runCase(mapped("Reader"), machines, TraceAdapter(), null)
        }
        assertEquals("re-entrant access to the test run from mtc", failure.message)
    }

    /**
     * P9, P13: a component whose machine is running is not removed under it. The
     * PTC's start kills the MTC, whose own start is still running; that is a
     * dynamic error, and the MTC stays.
     */
    @Test
    fun `a kill of a component whose machine runs is an error`() {
        val machines = Machines()
            .with(
                "Tester",
                Pair(
                    { context, _ ->
                        context.create("echo")
                        context.setverdict(Verdict.PASS, "")
                    },
                    idle(),
                ),
            )
            .with("Echo", Pair({ context, _ -> context.kill("mtc") }, idle()))
        val result = runCase(mtcAndEcho(), machines, TraceAdapter(), null)
        assertEquals(Verdict.ERROR, result.verdict)
        assertTrue(logged(result, "echo", LogKind.ERROR))
        assertFalse(logged(result, "mtc", LogKind.KILL), "the MTC was not removed")
        assertTrue(logged(result, "mtc", LogKind.VERDICT), "the MTC's start ran on after the refused kill")
    }

    /**
     * A context names one creation of its component: once the PTC is removed, a
     * call through a context it handed out throws instead of reaching another
     * component.
     */
    @Test
    fun `a context does not outlive its component`() {
        var kept: ComponentContext? = null
        val machines = Machines()
            .with(
                "Tester",
                Pair(
                    { context, _ ->
                        context.create("echo")
                        context.kill("echo")
                        val echo = checkNotNull(kept) { "the echo kept its context" }
                        echo.setverdict(Verdict.FAIL, "after its removal")
                    },
                    idle(),
                ),
            )
            .with("Echo", Pair({ context, _ -> kept = context }, idle()))
        val failure = assertFailsWith<IllegalStateException> {
            runCase(mtcAndEcho(), machines, TraceAdapter(), null)
        }
        assertEquals("the component echo was removed, and its context outlived it", failure.message)
    }
}

class TheRunIsRunOnceTest {

    /**
     * A run has read its adapter and holds its components' end, so running it a
     * second time is refused rather than run on that state.
     */
    @Test
    fun `a run that has run is not run again`() {
        val machines = Machines().with(
            "Tester",
            Pair(
                { context, self ->
                    context.setverdict(Verdict.PASS, "")
                    self.end()
                },
                idle(),
            ),
        )
        val run = TestCaseRun(lone("Tester"), machines.factory(), TraceAdapter(), null, ModuleParameters())
        assertEquals(Verdict.PASS, run.run().verdict)
        val failure = assertFailsWith<IllegalStateException> { run.run() }
        assertEquals("a test case run is run once; build another to run the test case again", failure.message)
    }
}

/** The test-port processor, of the type `StateMachineEngine.registerEventProcessor` takes a handler of. */
private typealias SendProcessor =
    (StateMachineEngine.HostSendRequest) -> List<StateMachineEngine.HostSendResponse>

/** The test-component invoker, of the type `StateMachineEngine.registerInvoker` takes a handler of. */
private typealias Invoker = (StateMachineEngine.HostInvokeEvent) -> StateMachineEngine.HostInvokeResponse?

/** The Event I/O Processor whose addresses are ports (docs/SCE_TEST_PROFILE.md §11.1). */
private const val TEST_PORT = "http://sce.dev/ext/test-port"

/** The invocation of a PTC (docs/SCE_TEST_PROFILE.md §11.1). */
private const val TEST_COMPONENT = "http://sce.dev/ext/test-component"

/**
 * A send (P6): the request carries the template's value, which the run appends
 * to the connected port, or hands the system adapter.
 */
private fun testPortProcessor(context: ComponentContext): SendProcessor = { request ->
    val value = checkNotNull(request.value) { "a test-port send carries its template's value" }
    context.send(request.target, Message(request.eventName, value))
    emptyList()
}

/** A PTC (P9): the start creates it, the cancel removes it. */
private fun testComponentInvoker(context: ComponentContext): Invoker = { event ->
    val start = event.start
    val cancel = event.cancel
    if (start != null) {
        context.create(start.invokeId)
    }
    if (cancel != null) {
        context.kill(cancel.invokeId)
    }
    null
}

/** The record a template of the port type reads. */
private class Ping(val n: Int)

/**
 * An MTC wired as a generated one is: its start registers the two processors,
 * each holding its context, and it reaches the doors of the run only through
 * them. Entering its state invokes `echo` and sends it a `Ping`; a timeout leaves
 * the state, which cancels the invoke.
 */
private class DoorsTester : ComponentMachine {
    private var invoker: Invoker? = null
    private var context: ComponentContext? = null
    private var ended = false

    override fun start(context: ComponentContext) {
        val sends = testPortProcessor(context)
        val invokes = testComponentInvoker(context)
        invokes(
            StateMachineEngine.HostInvokeEvent(
                start = StateMachineEngine.HostInvokeRequest(processorType = TEST_COMPONENT, invokeId = "echo"),
            ),
        )
        sends(
            StateMachineEngine.HostSendRequest(
                processorType = TEST_PORT,
                eventName = "ping",
                target = "p",
                value = Ping(3),
            ),
        )
        context.startTimer("reply", 10)
        this.invoker = invokes
        this.context = context
    }

    override fun deliverSnapshot() {
        val started = checkNotNull(this.context) { "a snapshot is delivered only to a started machine" }
        if (!started.timedOut("reply")) {
            return
        }
        started.timeout("reply")
        val invokes = checkNotNull(invoker) { "the start registered the invoker" }
        invokes(
            StateMachineEngine.HostInvokeEvent(
                cancel = StateMachineEngine.HostInvokeCancel(processorType = TEST_COMPONENT, invokeId = "echo"),
            ),
        )
        started.setverdict(Verdict.PASS, "")
        ended = true
    }

    override fun terminated(): Boolean = ended
}

private fun position(result: TestCaseResult, component: String, kind: LogKind): Int {
    val at = result.log.indexOfFirst { it.component == component && it.kind == kind }
    check(at >= 0) { "no $kind of $component in ${result.log}" }
    return at
}

class TheDoorsTest {

    /**
     * §12.3: the PTC the invoker's start creates receives the value the
     * processor's send carried, by its type, and the invoker's cancel removes it
     * at the timeout, before the MTC's verdict.
     */
    @Test
    fun `a send and a ptc go through the host processors`() {
        val echo: Step = { context, _ ->
            if (receives<Ping>(context, "q", "ping") { ping -> ping.n == 3 }) {
                context.receive("q")
                context.setverdict(Verdict.PASS, "the value arrived")
            } else if (context.snapshotPresent("q")) {
                context.receive("q")
                context.setverdict(Verdict.FAIL, "another value arrived")
            }
        }
        val factory: MachineFactory = { componentType ->
            when (componentType) {
                "Tester" -> DoorsTester()
                "Echo" -> Scripted(idle(), echo)
                else -> null
            }
        }
        val result = TestCaseRun(mtcAndEcho(), factory, TraceAdapter(), null, ModuleParameters()).run()
        assertEquals(Verdict.PASS, result.verdict, result.log.toString())
        val created = position(result, "echo", LogKind.CREATE)
        val sent = position(result, "mtc", LogKind.SEND)
        val received = position(result, "echo", LogKind.RECEIVE)
        val killed = position(result, "echo", LogKind.KILL)
        val verdict = position(result, "mtc", LogKind.VERDICT)
        assertTrue(created < sent && sent < received, result.log.toString())
        assertEquals("p ping", result.log[sent].detail)
        assertEquals("q ping", result.log[received].detail)
        assertEquals(10L, result.log[killed].at, "the cancel came at the timeout")
        assertTrue(killed < verdict, "the cancel removed the PTC, not the end")
        assertTrue(logged(result, "echo", LogKind.VERDICT))
    }
}
