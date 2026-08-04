// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c9e08658681ef21dd3bd5428d9da1979a690ea0bbf7340f9b10920cbe666e5c5
// generated-at: 0
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
