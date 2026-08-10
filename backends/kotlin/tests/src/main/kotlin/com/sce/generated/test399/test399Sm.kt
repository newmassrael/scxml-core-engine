// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c328b7a85ff2f465624a51fc9ec80940f3b78fbf4df26d1c6eaabfe6afd320f8
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/399/test399.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test399.scxml:6 :: _machine

package com.sce.generated.test399

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test399State : State {
    data object Fail : Test399State
    data object Pass : Test399State
    data object S0 : Test399State
    data object S01 : Test399State
    data object S02 : Test399State
    data object S03 : Test399State
    data object S04 : Test399State
    data object S05 : Test399State
    data object S06 : Test399State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test399Event : Event {
    data object Bar : Test399Event
    sealed interface Error : Test399Event {
        data object Execution : Error
    }
    sealed interface Foo : Test399Event {
        data object Self : Foo
        data object Zoo : Foo
    }
    data object Foos : Test399Event
    data object Timeout : Test399Event
}
// --- State Machine (W3C SCXML) ---

class Test399StateMachine(
) : StateMachineEngine<Test399State, Test399Event>() {

    override val initialState: Test399State = Test399State.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test399State): Test399State? = when (state) {
        is Test399State.S01 -> Test399State.S0
        is Test399State.S02 -> Test399State.S0
        is Test399State.S03 -> Test399State.S0
        is Test399State.S04 -> Test399State.S0
        is Test399State.S05 -> Test399State.S0
        is Test399State.S06 -> Test399State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test399State): Test399State = when (state) {
        is Test399State.S0 -> Test399State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test399State? = when (stateId) {
        "fail" -> Test399State.Fail
        "pass" -> Test399State.Pass
        "s0" -> Test399State.S0
        "s01" -> Test399State.S01
        "s02" -> Test399State.S02
        "s03" -> Test399State.S03
        "s04" -> Test399State.S04
        "s05" -> Test399State.S05
        "s06" -> Test399State.S06
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test399State): String = when (state) {
        is Test399State.Fail -> "fail"
        is Test399State.Pass -> "pass"
        is Test399State.S0 -> "s0"
        is Test399State.S01 -> "s01"
        is Test399State.S02 -> "s02"
        is Test399State.S03 -> "s03"
        is Test399State.S04 -> "s04"
        is Test399State.S05 -> "s05"
        is Test399State.S06 -> "s06"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test399State): Boolean = when (state) {
        is Test399State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test399State): Int = when (state) {
        is Test399State.Fail -> 8
        is Test399State.Pass -> 7
        is Test399State.S0 -> 0
        is Test399State.S01 -> 1
        is Test399State.S02 -> 2
        is Test399State.S03 -> 3
        is Test399State.S04 -> 4
        is Test399State.S05 -> 5
        is Test399State.S06 -> 6
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test399State,
        event: Test399Event
    ): TransitionResult<Test399State> = when (state) {
        is Test399State.S0 -> processS0(event)
        is Test399State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S04 -> {
            val result = processS04(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S05 -> {
            val result = processS05(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S06 -> {
            val result = processS06(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        event is Test399Event.Timeout -> TransitionResult.External(Test399State.Fail, Test399State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo bar"
        (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.S02, Test399State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo bar"
        (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.S03, Test399State.S02)

        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo bar"
        (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.S04, Test399State.S03)

        else -> TransitionResult.Ignored
    }

    private fun processS04(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo"
        (event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.Fail, Test399State.S04)

        event is Test399Event.Foos -> TransitionResult.External(Test399State.S05, Test399State.S04)

        else -> TransitionResult.Ignored
    }

    private fun processS05(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        event is Test399Event.Foo.Zoo -> TransitionResult.External(Test399State.S06, Test399State.S05)

        else -> TransitionResult.Ignored
    }

    private fun processS06(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test399State.Pass, Test399State.S06)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test399.scxml:6 :: _machine
    override fun onEntry(state: Test399State) {
        when (state) {
            is Test399State.Fail -> {
                // SCE-MAP: test399.scxml:69 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test399State.Pass -> {
                // SCE-MAP: test399.scxml:68 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test399State.S0 -> {
                // SCE-MAP: test399.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test399Event.Timeout)
            }
            is Test399State.S01 -> {
                // SCE-MAP: test399.scxml:17 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return

            raiseInternal(Test399Event.Foo.Self)
            }
            is Test399State.S02 -> {
                // SCE-MAP: test399.scxml:25 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return

            raiseInternal(Test399Event.Bar)
            }
            is Test399State.S03 -> {
                // SCE-MAP: test399.scxml:33 :: s03 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return

            raiseInternal(Test399Event.Foo.Zoo)
            }
            is Test399State.S04 -> {
                // SCE-MAP: test399.scxml:41 :: s04 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s04")) return

            raiseInternal(Test399Event.Foos)
            }
            is Test399State.S05 -> {
                // SCE-MAP: test399.scxml:50 :: s05 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s05")) return

            raiseInternal(Test399Event.Foo.Zoo)
            }
            is Test399State.S06 -> {
                // SCE-MAP: test399.scxml:58 :: s06 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s06")) return

            raiseInternal(Test399Event.Foo.Self)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test399.scxml:6 :: _machine
    override fun onExit(state: Test399State) {
        when (state) {
            is Test399State.Fail -> {
                // SCE-MAP: test399.scxml:69 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test399State.Pass -> {
                // SCE-MAP: test399.scxml:68 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test399State.S0 -> {
                // SCE-MAP: test399.scxml:9 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test399State.S01 -> {
                // SCE-MAP: test399.scxml:17 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test399State.S02 -> {
                // SCE-MAP: test399.scxml:25 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
            is Test399State.S03 -> {
                // SCE-MAP: test399.scxml:33 :: s03 :: _state_body
                activeStateIds.remove("s03")
            }
            is Test399State.S04 -> {
                // SCE-MAP: test399.scxml:41 :: s04 :: _state_body
                activeStateIds.remove("s04")
            }
            is Test399State.S05 -> {
                // SCE-MAP: test399.scxml:50 :: s05 :: _state_body
                activeStateIds.remove("s05")
            }
            is Test399State.S06 -> {
                // SCE-MAP: test399.scxml:58 :: s06 :: _state_body
                activeStateIds.remove("s06")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test399.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test399State,
        event: Test399Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
