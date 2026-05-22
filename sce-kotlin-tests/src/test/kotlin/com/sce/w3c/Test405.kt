// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 168f4a554705bdfb42cc51a9fbd01e4e5fc028c49c4d6f47071af9577599e075
// generated-at: 1779449862
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test405.scxml:1
package com.sce.w3c

import com.sce.generated.test405.Test405Event
import com.sce.generated.test405.Test405State
import com.sce.generated.test405.Test405StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: [the SCXML Processor executing a set of transitions] MUST then [after the onexits] execute the executable content contained in the transitions in document order.
@DisplayName("Test 405 -- W3C SCXML 3.13")
class Test405 : W3CTestBase<Test405State, Test405Event>() {
    override fun createStateMachine() = Test405StateMachine()
    override val expectedPassState: Test405State = Test405State.Pass
}
