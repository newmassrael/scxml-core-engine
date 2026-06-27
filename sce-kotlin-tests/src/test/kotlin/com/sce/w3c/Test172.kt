// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712
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
