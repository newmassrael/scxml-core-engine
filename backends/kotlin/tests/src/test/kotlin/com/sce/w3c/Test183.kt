// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4382370ad28e3e273e1d105876d814053809a7d5b704c5d43426b4c872443a55
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test183.scxml:1
package com.sce.w3c

import com.sce.generated.test183.Test183Event
import com.sce.generated.test183.Test183State
import com.sce.generated.test183.Test183StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'idlocation' is present, the SCXML Processor MUST generate an id when the parent send element is evaluated and store it in this location
@DisplayName("Test 183 -- W3C SCXML 6.2")
class Test183 : W3CTestBase<Test183State, Test183Event>() {
    override fun createStateMachine() = Test183StateMachine(createEngine())
    override val expectedPassState: Test183State = Test183State.Pass
}
