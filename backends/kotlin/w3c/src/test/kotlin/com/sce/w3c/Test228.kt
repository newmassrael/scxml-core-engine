// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test228.scxml:1
package com.sce.w3c

import com.sce.generated.test228.Test228Event
import com.sce.generated.test228.Test228State
import com.sce.generated.test228.Test228StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: the Processor MUST keep track of the unique invokeid and insure that it is included in all events that the invoked service returns to the invoking session.
@DisplayName("Test 228 -- W3C SCXML 6.4")
class Test228 : W3CTestBase<Test228State, Test228Event>() {
    override fun createStateMachine() = Test228StateMachine(createEngine())
    override val expectedPassState: Test228State = Test228State.Pass
    override val timeoutMs: Long = 5000L
}
