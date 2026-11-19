// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C++ machine is built from the values its deployment supplies (docs/adr/0109, D12;
// docs/SCE_ACCEPTED_SUBSET.md §2.15, "A value the deployment supplies"), run with no script
// engine, as the Rust machine is (backends/rust/tests/tests/a_machine_reads_its_deployments_values.rs).
// The machine is generated from sce-build/tests/fixtures/static_datamodel/static_deployment.scxml
// where the test target is built (tests/CMakeLists.txt).
//
// The machine reads a record and a list, from the configuration its deployment writes under
// it (`deployment/deploy.yaml`, which the build checks and writes as
// `deployment/static_deployment.config.json`, held to that text by
// sce-build/tests/a_machine_reads_the_values_its_deployment_supplies.rs) and from a file a
// host names (`deployment/from_file.json`). A configuration that is no value of the types is
// refused by the reader with the lift's code, pointer and bound, and no machine is built.

#include <fstream>
#include <optional>
#include <sstream>
#include <string>

#include <gtest/gtest.h>

#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "static_deployment_sm.h"

namespace SCE::Tests {
namespace {

namespace P = ::SCE::Generated::static_deployment;
using Machine = P::static_deployment;
using Deployment = P::static_deploymentDeployment;

/// The text of `name` under the fixture's `deployment/` directory.
std::string configuration(const std::string &name) {
    std::ifstream in(std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/static_datamodel/deployment/" + name);
    EXPECT_TRUE(in.good()) << name << " is read";
    std::stringstream text;
    text << in.rdbuf();
    return text.str();
}

/// The values `text` holds, read as a host reads them at start.
Deployment readValues(const std::string &text) {
    Deployment deployment;
    const ::SCE::Common::EventPayloadRefusal refusal = Deployment::read(text, deployment);
    EXPECT_TRUE(refusal.empty()) << "the configuration reads: " << refusal.reason();
    return deployment;
}

void takeRow(Machine &machine) {
    machine.raiseExternal("row.taken", "");
    StaticScenario::settleRounds(machine);
}

// The values the deployment writes are what the machine starts on: an initial value reads
// each, the entry reads the record's text and the list's first element, and a guard reads
// the count it started from.
TEST(AMachineReadsItsDeploymentsValuesTest, AMachineStartsOnTheValuesItsDeploymentWrites) {
    Machine machine(readValues(configuration("static_deployment.config.json")));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    EXPECT_EQ(machine.remaining(), 2u) << "the record's `max_rows`";
    EXPECT_EQ(machine.host_count(), 2u) << "the list's length";
    EXPECT_EQ(machine.label(), "main") << "the record's `label`";
    EXPECT_EQ(machine.first_host(), "a.example") << "the list's first element";
    EXPECT_EQ(machine.refused(), 0u);
    for (int n = 0; n < 3; ++n) {
        takeRow(machine);
    }
    EXPECT_EQ(machine.remaining(), 0u) << "two rows were taken";
    EXPECT_EQ(machine.refused(), 1u) << "the third found none left";
}

// A file a host reads is read by the same reader: its members in an order of their own,
// and one the machine declares no value of stepped over.
TEST(AMachineReadsItsDeploymentsValuesTest, AMachineStartsOnTheValuesAFileHolds) {
    Machine machine(readValues(configuration("from_file.json")));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    EXPECT_EQ(machine.remaining(), 5u);
    EXPECT_EQ(machine.host_count(), 1u);
    EXPECT_EQ(machine.label(), "file");
    EXPECT_EQ(machine.first_host(), "c.example");
}

// A configuration that does not read refuses the start: the host is told the code, the
// place and the bound, and the values it would have built a machine from are left as they
// were.
TEST(AMachineReadsItsDeploymentsValuesTest, AConfigurationThatDoesNotReadRefusesTheStart) {
    Deployment untouched;
    ::SCE::Common::EventPayloadRefusal refusal = Deployment::read(configuration("too_many_hosts.json"), untouched);
    ASSERT_FALSE(refusal.empty()) << "five hosts are past the four the list holds";
    EXPECT_EQ(refusal.code(), ::SCE::Common::RefusalCode::TooMany);
    EXPECT_EQ(refusal.pointer(), "/hosts");
    EXPECT_EQ(refusal.bound(), std::optional<uint32_t>(4u));
    EXPECT_EQ(untouched.v_hosts.size(), 0u) << "nothing of a refused configuration is kept";

    refusal = Deployment::read(configuration("missing_hosts.json"), untouched);
    ASSERT_FALSE(refusal.empty()) << "a value the machine declares is missing";
    EXPECT_EQ(refusal.code(), ::SCE::Common::RefusalCode::Missing);
    EXPECT_EQ(refusal.pointer(), "/hosts");

    refusal = Deployment::read("", untouched);
    EXPECT_EQ(refusal.code(), ::SCE::Common::RefusalCode::NotJson);
    refusal = Deployment::read("[]", untouched);
    EXPECT_EQ(refusal.code(), ::SCE::Common::RefusalCode::NotObject);
}

}  // namespace
}  // namespace SCE::Tests
