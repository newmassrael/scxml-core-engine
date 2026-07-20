// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test232.scxml:1
package com.sce.w3c

import com.sce.generated.test232.Test232Event
import com.sce.generated.test232.Test232State
import com.sce.generated.test232.Test232StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: he invoked external service MAY return multiple events while it is processing
@DisplayName("Test 232 -- W3C SCXML 6.4")
class Test232 : W3CTestBase<Test232State, Test232Event>() {
    override fun createStateMachine() = Test232StateMachine()
    override val expectedPassState: Test232State = Test232State.Pass
}
