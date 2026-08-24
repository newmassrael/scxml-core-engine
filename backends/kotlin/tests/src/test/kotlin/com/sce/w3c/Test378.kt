// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 672592645a46a971e7d9a638044244b01d838f9ebaf5e6860dc88538368c4548
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test378.scxml:1
package com.sce.w3c

import com.sce.generated.test378.Test378Event
import com.sce.generated.test378.Test378State
import com.sce.generated.test378.Test378StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.9: The SCXML processor MUST treat each [onexit] handler as a separate block of executable content.
@DisplayName("Test 378 -- W3C SCXML 3.9")
class Test378 : W3CTestBase<Test378State, Test378Event>() {
    override fun createStateMachine() = Test378StateMachine(createEngine())
    override val expectedPassState: Test378State = Test378State.Pass
}
