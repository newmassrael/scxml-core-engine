// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test449.scxml:1
package com.sce.w3c

import com.sce.generated.test449.Test449Event
import com.sce.generated.test449.Test449State
import com.sce.generated.test449.Test449StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must convert ECMAScript expressions used in conditional expressions into their effective boolean value using the ToBoolean operator as described in Section 9.2 of [ECMASCRIPT-262].
@DisplayName("Test 449 -- W3C SCXML B.2")
class Test449 : W3CTestBase<Test449State, Test449Event>() {
    override fun createStateMachine() = Test449StateMachine(createEngine())
    override val expectedPassState: Test449State = Test449State.Pass
}
