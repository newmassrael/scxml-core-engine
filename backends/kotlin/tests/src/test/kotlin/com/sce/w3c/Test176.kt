// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test176.scxml:1
package com.sce.w3c

import com.sce.generated.test176.Test176Event
import com.sce.generated.test176.Test176State
import com.sce.generated.test176.Test176StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The SCXML Processor MUST evaluate param when the parent send element is evaluated and pass the resulting data unmodified to the external service when the message is delivered
@DisplayName("Test 176 -- W3C SCXML 6.2")
class Test176 : W3CTestBase<Test176State, Test176Event>() {
    override fun createStateMachine() = Test176StateMachine(createEngine())
    override val expectedPassState: Test176State = Test176State.Pass
}
