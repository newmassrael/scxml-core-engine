// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test552.scxml:1
package com.sce.w3c

import com.sce.generated.test552.Test552Event
import com.sce.generated.test552.Test552State
import com.sce.generated.test552.Test552StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: If the 'src' attribute is present, the Platform MUST fetch the specified object at the time specified by the 'binding' attribute of scxml and MUST assign it as the value of the data element.
@DisplayName("Test 552 -- W3C SCXML 5.3")
class Test552 : W3CTestBase<Test552State, Test552Event>() {
    override fun createStateMachine() = Test552StateMachine(createEngine())
    override val expectedPassState: Test552State = Test552State.Pass
}
