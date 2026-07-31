// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test205.scxml:1
package com.sce.w3c

import com.sce.generated.test205.Test205Event
import com.sce.generated.test205.Test205State
import com.sce.generated.test205.Test205StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The sending SCXML Interpreter MUST not alter the content of the send
@DisplayName("Test 205 -- W3C SCXML 6.2")
class Test205 : W3CTestBase<Test205State, Test205Event>() {
    override fun createStateMachine() = Test205StateMachine(createEngine())
    override val expectedPassState: Test205State = Test205State.Pass
}
