// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test304.scxml:1
package com.sce.w3c

import com.sce.generated.test304.Test304Event
import com.sce.generated.test304.Test304State
import com.sce.generated.test304.Test304StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: In a conformant SCXML document, the name of any script variable MAY be used as a location expression. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 304 -- W3C SCXML 5.8")
class Test304 : W3CTestBase<Test304State, Test304Event>() {
    override fun createStateMachine() = Test304StateMachine(createEngine())
    override val expectedPassState: Test304State = Test304State.Pass
}
