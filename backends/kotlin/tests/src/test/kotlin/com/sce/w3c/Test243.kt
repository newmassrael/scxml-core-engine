// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: dbfa9cca1428438cf4178bb8fcf463f9b9d0c7c649f4bf0e0f3de90abcfd2a47
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test243.scxml:1
package com.sce.w3c

import com.sce.generated.test243.Test243Event
import com.sce.generated.test243.Test243State
import com.sce.generated.test243.Test243StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked process is of type http://www.w3.org/TR/scxml/ and 'name' of a param element in the invoke matches the 'id' of a data element in the top-level data declarations of the invoked session, the SCXML Processor MUST use the value of the param element as the initial value of the corresponding data element.
@DisplayName("Test 243 -- W3C SCXML 6.4")
class Test243 : W3CTestBase<Test243State, Test243Event>() {
    override fun createStateMachine() = Test243StateMachine(createEngine())
    override val expectedPassState: Test243State = Test243State.Pass
    override val timeoutMs: Long = 5000L
}
