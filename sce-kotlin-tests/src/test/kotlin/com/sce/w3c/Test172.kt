// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d2f0bcf4d5c727ad2446a904193402929b9b2d65dfec5e5c07ad3bc881483b09
// generated-at: 1780358475
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test172.scxml:1
package com.sce.w3c

import com.sce.generated.test172.Test172Event
import com.sce.generated.test172.Test172State
import com.sce.generated.test172.Test172StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'eventexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'event'.
@DisplayName("Test 172 -- W3C SCXML 6.2")
class Test172 : W3CTestBase<Test172State, Test172Event>() {
    override fun createStateMachine() = Test172StateMachine(createEngine())
    override val expectedPassState: Test172State = Test172State.Pass
}
