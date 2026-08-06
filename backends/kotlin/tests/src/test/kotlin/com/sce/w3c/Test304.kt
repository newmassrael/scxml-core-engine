// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 419df244c5f8e83941772fe0e162c3decc43983c72d904462cbbb6425fb07338
// generated-at: 0
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
