// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c96808b03e7b119d29792dbf258f9125c91be8c72d4823c8f9b56e0e05a3240b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test277.scxml:1
package com.sce.w3c

import com.sce.generated.test277.Test277Event
import com.sce.generated.test277.Test277State
import com.sce.generated.test277.Test277StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: If the value specified for a data element (by 'src', children, or the environment) is not a legal data value, the SCXML Processor MUST raise place error.execution in the internal event queue and MUST create an empty data element in the data model with the specified id.
@DisplayName("Test 277 -- W3C SCXML 5.3")
class Test277 : W3CTestBase<Test277State, Test277Event>() {
    override fun createStateMachine() = Test277StateMachine(createEngine())
    override val expectedPassState: Test277State = Test277State.Pass
}
