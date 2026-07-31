// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test224.scxml:1
package com.sce.w3c

import com.sce.generated.test224.Test224Event
import com.sce.generated.test224.Test224State
import com.sce.generated.test224.Test224StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the platform generates an identifier for 'idlocation', the identifier MUST have the form stateid.platformid, where stateid is the id of the state containing this element and platformid is automatically generated.
@DisplayName("Test 224 -- W3C SCXML 6.4")
class Test224 : W3CTestBase<Test224State, Test224Event>() {
    override fun createStateMachine() = Test224StateMachine(createEngine())
    override val expectedPassState: Test224State = Test224State.Pass
    override val timeoutMs: Long = 5000L
}
