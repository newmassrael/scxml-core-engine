// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 419df244c5f8e83941772fe0e162c3decc43983c72d904462cbbb6425fb07338
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
