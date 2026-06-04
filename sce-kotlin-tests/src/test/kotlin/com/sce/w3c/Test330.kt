// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
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
