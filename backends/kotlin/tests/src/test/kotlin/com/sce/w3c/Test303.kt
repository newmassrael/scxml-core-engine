// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test303.scxml:1
package com.sce.w3c

import com.sce.generated.test303.Test303Event
import com.sce.generated.test303.Test303State
import com.sce.generated.test303.Test303StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: The SCXML Processor MUST evaluate all script elements not children of scxml as part of normal executable content evaluation. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 303 -- W3C SCXML 5.8")
class Test303 : W3CTestBase<Test303State, Test303Event>() {
    override fun createStateMachine() = Test303StateMachine(createEngine())
    override val expectedPassState: Test303State = Test303State.Pass
}
