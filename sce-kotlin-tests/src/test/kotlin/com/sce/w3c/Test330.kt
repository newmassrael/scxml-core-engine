// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test330.scxml:1
package com.sce.w3c

import com.sce.generated.test330.Test330Event
import com.sce.generated.test330.Test330State
import com.sce.generated.test330.Test330StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST insure that the following fields (name, type, sendid, origin, origintype, invokeid, data) are present in all events (_event variable), whether internal or external.
@DisplayName("Test 330 -- W3C SCXML 5.10")
class Test330 : W3CTestBase<Test330State, Test330Event>() {
    override fun createStateMachine() = Test330StateMachine()
    override val expectedPassState: Test330State = Test330State.Pass
}
