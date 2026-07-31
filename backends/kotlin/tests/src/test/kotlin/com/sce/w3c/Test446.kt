// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test446.scxml:1
package com.sce.w3c

import com.sce.generated.test446.Test446Event
import com.sce.generated.test446.Test446State
import com.sce.generated.test446.Test446StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if either the 'src' attribute or in-line content is provided in the data elemenet, then if the content (whether fetched or provided in-line) is JSON and the processor supports JSON, the SCXML Processor MUST create the corresponding ECMAScript structure and assign it as the value of the data element.
@DisplayName("Test 446 -- W3C SCXML B.2")
class Test446 : W3CTestBase<Test446State, Test446Event>() {
    override fun createStateMachine() = Test446StateMachine(createEngine())
    override val expectedPassState: Test446State = Test446State.Pass
}
