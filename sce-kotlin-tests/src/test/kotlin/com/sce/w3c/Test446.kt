// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test446.scxml:1
package com.sce.w3c

import com.sce.generated.test446.Test446Event
import com.sce.generated.test446.Test446State
import com.sce.generated.test446.Test446StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if either the 'src' attribute or in-line content is provided in the data elemenet, then if the content (whether fetched or provided in-line) is JSON and the processor supports JSON, the SCXML Processor MUST create the corresponding ECMAScript structure and assign it as the value of the data element.
@DisplayName("Test 446 -- W3C SCXML B.2")
class Test446 : W3CTestBase<Test446State, Test446Event>() {
    override fun createStateMachine() = Test446StateMachine(createEngine())
    override val expectedPassState: Test446State = Test446State.Pass
}
