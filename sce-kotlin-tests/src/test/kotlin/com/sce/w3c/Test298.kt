// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 3acf03cd1e197da0d6a3e7ecc2541747678939372fbe1d99b37c7415a38be32a
// generated-at: 1780830703
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test298.scxml:1
package com.sce.w3c

import com.sce.generated.test298.Test298Event
import com.sce.generated.test298.Test298State
import com.sce.generated.test298.Test298StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: If the 'location' attribute on a param element does not refer to a valid location in the data model, the processor MUST place the error error.execution on the internal event queue.
@DisplayName("Test 298 -- W3C SCXML 5.7")
class Test298 : W3CTestBase<Test298State, Test298Event>() {
    override fun createStateMachine() = Test298StateMachine(createEngine())
    override val expectedPassState: Test298State = Test298State.Pass
}
