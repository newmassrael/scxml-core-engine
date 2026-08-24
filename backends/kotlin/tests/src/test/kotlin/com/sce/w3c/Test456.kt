// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 672592645a46a971e7d9a638044244b01d838f9ebaf5e6860dc88538368c4548
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test456.scxml:1
package com.sce.w3c

import com.sce.generated.test456.Test456Event
import com.sce.generated.test456.Test456State
import com.sce.generated.test456.Test456StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: the SCXML Processor must accept any ECMAScript program as defined in Section 14 of [ECMASCRIPT-262] as the content of a script element.
@DisplayName("Test 456 -- W3C SCXML B.2")
class Test456 : W3CTestBase<Test456State, Test456Event>() {
    override fun createStateMachine() = Test456StateMachine(createEngine())
    override val expectedPassState: Test456State = Test456State.Pass
}
