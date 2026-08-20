// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4382370ad28e3e273e1d105876d814053809a7d5b704c5d43426b4c872443a55
// generated-at: 0
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
