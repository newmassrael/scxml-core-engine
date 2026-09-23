// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
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
