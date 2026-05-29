// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5e63a3ecc19b397697c3e24d727bc3c78cb748941f07d7f7c9d76cdea58d15a4
// generated-at: 1780032748
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test580.scxml:1
package com.sce.w3c

import com.sce.generated.test580.Test580Event
import com.sce.generated.test580.Test580State
import com.sce.generated.test580.Test580StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: It follows from the semantics of history states that they never end up in the state configuration
@DisplayName("Test 580 -- W3C SCXML 3.10")
class Test580 : W3CTestBase<Test580State, Test580Event>() {
    override fun createStateMachine() = Test580StateMachine(createEngine())
    override val expectedPassState: Test580State = Test580State.Pass
    override val timeoutMs: Long = 5000L
}
