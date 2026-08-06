// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
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
