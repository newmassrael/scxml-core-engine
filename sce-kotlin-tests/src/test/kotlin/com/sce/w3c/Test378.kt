// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 3acf03cd1e197da0d6a3e7ecc2541747678939372fbe1d99b37c7415a38be32a
// generated-at: 1780830703
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test378.scxml:1
package com.sce.w3c

import com.sce.generated.test378.Test378Event
import com.sce.generated.test378.Test378State
import com.sce.generated.test378.Test378StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.9: The SCXML processor MUST treat each [onexit] handler as a separate block of executable content.
@DisplayName("Test 378 -- W3C SCXML 3.9")
class Test378 : W3CTestBase<Test378State, Test378Event>() {
    override fun createStateMachine() = Test378StateMachine(createEngine())
    override val expectedPassState: Test378State = Test378State.Pass
}
