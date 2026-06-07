// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 07a1057b89512b0ade7260ce662ea4e6ef3c2abde2d5bd32fb4fe82bd263d4bc
// generated-at: 1780802714
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test190.scxml:1
package com.sce.w3c

import com.sce.generated.test190.Test190Event
import com.sce.generated.test190.Test190State
import com.sce.generated.test190.Test190StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If the target is the special term '#_scxml_sessionid', where sessionid is the id of an SCXML session that is accessible to the Processor, the Processor MUST add the event to the external queue of that session.
@DisplayName("Test 190 -- W3C SCXML C.1")
class Test190 : W3CTestBase<Test190State, Test190Event>() {
    override fun createStateMachine() = Test190StateMachine(createEngine())
    override val expectedPassState: Test190State = Test190State.Pass
}
