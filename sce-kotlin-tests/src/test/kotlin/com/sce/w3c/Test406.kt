// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 4a741a2915b4fc1d6292d4cc68ddf4af4e269ea63531bfee3c7b94ccd4e9b0bc
// generated-at: 1782562648
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test406.scxml:1
package com.sce.w3c

import com.sce.generated.test406.Test406Event
import com.sce.generated.test406.Test406State
import com.sce.generated.test406.Test406StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: [the SCXML Processor executing a set of transitions] MUST then [after the exits and the transitions] enter the states in the transitions' entry set in entry order.
@DisplayName("Test 406 -- W3C SCXML 3.13")
class Test406 : W3CTestBase<Test406State, Test406Event>() {
    override fun createStateMachine() = Test406StateMachine()
    override val expectedPassState: Test406State = Test406State.Pass
}
