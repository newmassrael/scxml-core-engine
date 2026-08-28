// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2999a09c910b968e408271dc62f423daf659e11e3dbdea0cdf9857029573f331
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test236.scxml:1
package com.sce.w3c

import com.sce.generated.test236.Test236Event
import com.sce.generated.test236.Test236State
import com.sce.generated.test236.Test236StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: The external service MUST NOT generate any other events after the invoke.done.invokeid event.
@DisplayName("Test 236 -- W3C SCXML 6.4")
class Test236 : W3CTestBase<Test236State, Test236Event>() {
    override fun createStateMachine() = Test236StateMachine()
    override val expectedPassState: Test236State = Test236State.Pass
    override val timeoutMs: Long = 5000L
}
