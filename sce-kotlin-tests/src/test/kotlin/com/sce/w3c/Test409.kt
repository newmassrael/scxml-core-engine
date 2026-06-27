// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 4a741a2915b4fc1d6292d4cc68ddf4af4e269ea63531bfee3c7b94ccd4e9b0bc
// generated-at: 1782562648
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test409.scxml:1
package com.sce.w3c

import com.sce.generated.test409.Test409Event
import com.sce.generated.test409.Test409State
import com.sce.generated.test409.Test409StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Finally [after the onexits and canceling the invocations], the Processor MUST remove the state from the active state's list.
@DisplayName("Test 409 -- W3C SCXML 3.13")
class Test409 : W3CTestBase<Test409State, Test409Event>() {
    override fun createStateMachine() = Test409StateMachine()
    override val expectedPassState: Test409State = Test409State.Pass
    override val timeoutMs: Long = 5000L
}
