// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test244.scxml:1
package com.sce.w3c

import com.sce.generated.test244.Test244Event
import com.sce.generated.test244.Test244State
import com.sce.generated.test244.Test244StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked process is of type http://www.w3.org/TR/scxml/ and the key of namelist item in the invoke matches the 'id' of a data element in the top-level data declarations of the invoked session, the SCXML Processor MUST use the corresponding value as the initial value of the corresponding data element.
@DisplayName("Test 244 -- W3C SCXML 6.4")
class Test244 : W3CTestBase<Test244State, Test244Event>() {
    override fun createStateMachine() = Test244StateMachine(createEngine())
    override val expectedPassState: Test244State = Test244State.Pass
}
