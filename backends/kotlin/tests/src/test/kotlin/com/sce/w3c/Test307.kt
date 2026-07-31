// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test307.scxml:1
package com.sce.w3c

import com.sce.generated.test307.Test307Event
import com.sce.generated.test307.Test307State
import com.sce.generated.test307.Test307StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: When "late" data binding is used, accessing data substructure in expressions before the corresponding data element is loaded MUST yield the same execution-time behavior as accessing non-existent data substructure in a loaded data instance.
@DisplayName("Test 307 -- W3C SCXML 5.9")
class Test307 : W3CTestBase<Test307State, Test307Event>() {
    override fun createStateMachine() = Test307StateMachine(createEngine())
    override val expectedPassState: Test307State = Test307State.Final
}
