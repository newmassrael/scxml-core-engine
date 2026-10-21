// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
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
    override fun createStateMachine() = Test449StateMachine()
    override val expectedPassState: Test449State = Test449State.Pass
}
