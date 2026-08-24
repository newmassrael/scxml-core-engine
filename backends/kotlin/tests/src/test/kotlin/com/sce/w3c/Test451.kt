// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 672592645a46a971e7d9a638044244b01d838f9ebaf5e6860dc88538368c4548
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test451.scxml:1
package com.sce.w3c

import com.sce.generated.test451.Test451Event
import com.sce.generated.test451.Test451State
import com.sce.generated.test451.Test451StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must add an ECMAScript
@DisplayName("Test 451 -- W3C SCXML B.2")
class Test451 : W3CTestBase<Test451State, Test451Event>() {
    override fun createStateMachine() = Test451StateMachine()
    override val expectedPassState: Test451State = Test451State.Pass
}
