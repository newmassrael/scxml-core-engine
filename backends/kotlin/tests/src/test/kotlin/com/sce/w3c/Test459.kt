// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: de19bbe38b591cc01cdfdd92e882d57ccdf3a1af11ac8c67d61cfaa6882bea1d
// generated-at: 0
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
