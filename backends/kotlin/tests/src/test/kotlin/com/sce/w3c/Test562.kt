// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test562.scxml:1
package com.sce.w3c

import com.sce.generated.test562.Test562Event
import com.sce.generated.test562.Test562State
import com.sce.generated.test562.Test562StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if the content provided to populate _event.data is neither key-value pairs nor JSON nor a valid XML document, the Processor MUST treat the content treat the content as a space-normalized string literal and assign it as the value of _event.data.
@DisplayName("Test 562 -- W3C SCXML B.2")
class Test562 : W3CTestBase<Test562State, Test562Event>() {
    override fun createStateMachine() = Test562StateMachine(createEngine())
    override val expectedPassState: Test562State = Test562State.Pass
}
