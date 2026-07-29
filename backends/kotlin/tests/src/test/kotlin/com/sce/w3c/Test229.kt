// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c22d767976ad0f3af27597215acac4daa969b18394744727f9f1e4af8f5db2d7
// generated-at: 1785338317
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
