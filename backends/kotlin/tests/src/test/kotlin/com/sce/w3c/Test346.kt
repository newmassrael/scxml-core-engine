// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 06c80ca1f364d1bd47dcf4355438c9eb8afe054b2712ace900a9053d7a3870aa
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test346.scxml:1
package com.sce.w3c

import com.sce.generated.test346.Test346Event
import com.sce.generated.test346.Test346State
import com.sce.generated.test346.Test346StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST place the error error.execution on the internal event queue when any attempt to change the value of a system variable is made.
@DisplayName("Test 346 -- W3C SCXML 5.10")
class Test346 : W3CTestBase<Test346State, Test346Event>() {
    override fun createStateMachine() = Test346StateMachine(createEngine())
    override val expectedPassState: Test346State = Test346State.Pass
}
