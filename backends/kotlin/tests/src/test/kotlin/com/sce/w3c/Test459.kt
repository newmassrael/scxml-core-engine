// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test459.scxml:1
package com.sce.w3c

import com.sce.generated.test459.Test459Event
import com.sce.generated.test459.Test459State
import com.sce.generated.test459.Test459StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, the iteration order for the foreach element is the order of the underlying ECMAScript array, and goes from an index of 0 by increments of one to an index of array_name.length - 1.
@DisplayName("Test 459 -- W3C SCXML B.2")
class Test459 : W3CTestBase<Test459State, Test459Event>() {
    override fun createStateMachine() = Test459StateMachine(createEngine())
    override val expectedPassState: Test459State = Test459State.Pass
}
