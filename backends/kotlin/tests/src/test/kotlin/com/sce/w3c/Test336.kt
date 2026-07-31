// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test336.scxml:1
package com.sce.w3c

import com.sce.generated.test336.Test336Event
import com.sce.generated.test336.Test336State
import com.sce.generated.test336.Test336StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: For external events, the SCXML Processor SHOULD set the origintype field to a value which, in combination with the 'origin' field, will allow the receiver of the event to send a response back to the originating entity.
@DisplayName("Test 336 -- W3C SCXML 5.10")
class Test336 : W3CTestBase<Test336State, Test336Event>() {
    override fun createStateMachine() = Test336StateMachine(createEngine())
    override val expectedPassState: Test336State = Test336State.Pass
}
