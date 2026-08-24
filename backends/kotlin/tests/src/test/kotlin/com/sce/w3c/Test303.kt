// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test303.scxml:1
package com.sce.w3c

import com.sce.generated.test303.Test303Event
import com.sce.generated.test303.Test303State
import com.sce.generated.test303.Test303StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: The SCXML Processor MUST evaluate all script elements not children of scxml as part of normal executable content evaluation. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 303 -- W3C SCXML 5.8")
class Test303 : W3CTestBase<Test303State, Test303Event>() {
    override fun createStateMachine() = Test303StateMachine(createEngine())
    override val expectedPassState: Test303State = Test303State.Pass
}
