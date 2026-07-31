// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test294.scxml:1
package com.sce.w3c

import com.sce.generated.test294.Test294Event
import com.sce.generated.test294.Test294State
import com.sce.generated.test294.Test294StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.5: In cases where the SCXML Processor generates a 'done' event upon entry into the final state, it MUST evaluate the donedata elements param or content children and place the resulting data in the _event.data field. The exact format of that data will be determined by the datamodel
@DisplayName("Test 294 -- W3C SCXML 5.5")
class Test294 : W3CTestBase<Test294State, Test294Event>() {
    override fun createStateMachine() = Test294StateMachine(createEngine())
    override val expectedPassState: Test294State = Test294State.Pass
}
