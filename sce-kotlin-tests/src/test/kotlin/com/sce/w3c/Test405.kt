// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
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
