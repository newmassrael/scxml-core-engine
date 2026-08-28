// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2999a09c910b968e408271dc62f423daf659e11e3dbdea0cdf9857029573f331
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test329.scxml:1
package com.sce.w3c

import com.sce.generated.test329.Test329Event
import com.sce.generated.test329.Test329State
import com.sce.generated.test329.Test329StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST cause any attempt to change the value of a system variable to fail.
@DisplayName("Test 329 -- W3C SCXML 5.10")
class Test329 : W3CTestBase<Test329State, Test329Event>() {
    override fun createStateMachine() = Test329StateMachine(createEngine())
    override val expectedPassState: Test329State = Test329State.Pass
}
