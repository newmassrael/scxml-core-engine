// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
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
