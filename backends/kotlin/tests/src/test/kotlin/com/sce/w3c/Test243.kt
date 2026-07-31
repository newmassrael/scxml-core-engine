// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test243.scxml:1
package com.sce.w3c

import com.sce.generated.test243.Test243Event
import com.sce.generated.test243.Test243State
import com.sce.generated.test243.Test243StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked process is of type http://www.w3.org/TR/scxml/ and 'name' of a param element in the invoke matches the 'id' of a data element in the top-level data declarations of the invoked session, the SCXML Processor MUST use the value of the param element as the initial value of the corresponding data element.
@DisplayName("Test 243 -- W3C SCXML 6.4")
class Test243 : W3CTestBase<Test243State, Test243Event>() {
    override fun createStateMachine() = Test243StateMachine(createEngine())
    override val expectedPassState: Test243State = Test243State.Pass
    override val timeoutMs: Long = 5000L
}
