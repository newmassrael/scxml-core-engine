// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5e63a3ecc19b397697c3e24d727bc3c78cb748941f07d7f7c9d76cdea58d15a4
// generated-at: 1780032748
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
