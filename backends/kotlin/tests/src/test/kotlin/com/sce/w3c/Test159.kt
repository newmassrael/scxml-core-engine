// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test159.scxml:1
package com.sce.w3c

import com.sce.generated.test159.Test159Event
import com.sce.generated.test159.Test159State
import com.sce.generated.test159.Test159StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.9: If the processing of an element of executable content causes an error to be raised, the processor MUST NOT process the remaining elements of the block.
@DisplayName("Test 159 -- W3C SCXML 4.9")
class Test159 : W3CTestBase<Test159State, Test159Event>() {
    override fun createStateMachine() = Test159StateMachine(createEngine())
    override val expectedPassState: Test159State = Test159State.Pass
}
