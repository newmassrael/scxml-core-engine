// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test579.scxml:1
package com.sce.w3c

import com.sce.generated.test579.Test579Event
import com.sce.generated.test579.Test579State
import com.sce.generated.test579.Test579StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: Before the parent state has been visited for the first time, if a transition is executed that takes the history state as its target,
@DisplayName("Test 579 -- W3C SCXML 3.10")
class Test579 : W3CTestBase<Test579State, Test579Event>() {
    override fun createStateMachine() = Test579StateMachine(createEngine())
    override val expectedPassState: Test579State = Test579State.Pass
    override val timeoutMs: Long = 5000L
}
