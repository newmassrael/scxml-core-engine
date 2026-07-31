// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test330.scxml:1
package com.sce.w3c

import com.sce.generated.test330.Test330Event
import com.sce.generated.test330.Test330State
import com.sce.generated.test330.Test330StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST insure that the following fields (name, type, sendid, origin, origintype, invokeid, data) are present in all events (_event variable), whether internal or external.
@DisplayName("Test 330 -- W3C SCXML 5.10")
class Test330 : W3CTestBase<Test330State, Test330Event>() {
    override fun createStateMachine() = Test330StateMachine()
    override val expectedPassState: Test330State = Test330State.Pass
}
