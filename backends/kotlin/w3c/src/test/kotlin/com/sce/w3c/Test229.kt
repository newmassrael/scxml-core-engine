// SCE-GENERATED — DO NOT EDIT
// source-hash: f3158878ef169c4cfdbf46e8cf020833d4c4ed95fcdbe085cbf23ada07986427
// source: test229.scxml
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test229.scxml:1
package com.sce.w3c

import com.sce.generated.test229.Test229Event
import com.sce.generated.test229.Test229State
import com.sce.generated.test229.Test229StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the 'autoforward' attribute is set to true, the SCXML Processor MUST send an exact copy of every external event it receives to the invoked process.
@DisplayName("Test 229 -- W3C SCXML 6.4")
class Test229 : W3CTestBase<Test229State, Test229Event>() {
    override fun createStateMachine() = Test229StateMachine()
    override val expectedPassState: Test229State = Test229State.Pass
    override val timeoutMs: Long = 5000L
}
