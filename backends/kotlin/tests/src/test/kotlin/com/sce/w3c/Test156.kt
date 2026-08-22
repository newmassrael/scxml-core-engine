// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 35c8283af859855fefb53b36dbcc38c1c549511d8a5bf7a3250f4215fef24b75
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test156.scxml:1
package com.sce.w3c

import com.sce.generated.test156.Test156Event
import com.sce.generated.test156.Test156State
import com.sce.generated.test156.Test156StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: If the evaluation of any child element of foreach causes an error, the processor MUST cease execution of the foreach element and the block that contains it.
@DisplayName("Test 156 -- W3C SCXML 4.6")
class Test156 : W3CTestBase<Test156State, Test156Event>() {
    override fun createStateMachine() = Test156StateMachine(createEngine())
    override val expectedPassState: Test156State = Test156State.Pass
}
