// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test159.scxml:1
package com.sce.w3c

import com.sce.generated.test159.Test159Event
import com.sce.generated.test159.Test159State
import com.sce.generated.test159.Test159StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.9: If the processing of an element of executable content causes an error to be raised, the processor MUST NOT process the remaining elements of the block.
@DisplayName("Test 159 -- W3C SCXML 4.9")
class Test159 : W3CTestBase<Test159State, Test159Event>() {
    override fun createStateMachine() = Test159StateMachine(createEngine())
    override val expectedPassState: Test159State = Test159State.Pass
}
