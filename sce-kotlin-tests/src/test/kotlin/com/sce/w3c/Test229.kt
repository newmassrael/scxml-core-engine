// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
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
