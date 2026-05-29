// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5af0768adc0cd444b401fc40536c0de87cadf9b1f8be7299536f4fc9ed22e337
// generated-at: 1780020098
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test520.scxml:1
package com.sce.w3c

import com.sce.generated.test520.Test520Event
import com.sce.generated.test520.Test520State
import com.sce.generated.test520.Test520StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If a content child is present, the SCXML Processor MUST use its value as the body of the message.
@DisplayName("Test 520 -- W3C SCXML C.2")
class Test520 : W3CHttpTestBase<Test520State, Test520Event>() {
    override fun createStateMachine() = Test520StateMachine()
    override val expectedPassState: Test520State = Test520State.Pass
}
