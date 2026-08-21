// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2cf4917c7dff79eaf746b52e649909e9c7318e80b65f49555ba6a2bcd0d8eaca
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
