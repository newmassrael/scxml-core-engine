// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.12 and docs/adr/0109, step 4 of its order of work: an
// enum field of a host-run `<invoke>`'s request record. Rust compile+run gate;
// the C++ twin is `AStandardHostScenario.ATypedKeyGenerationReadsItsUseByItsName`.
//
// The committed machine under `src/integration/static_datamodel/` is generated
// from `sce-build/tests/fixtures/static_datamodel/static_host_cose.scxml`
// (regen: `scripts/regen_static_datamodel_rust.sh`). Its key generation's
// request holds `key_use`, a value of the library's enum `key_use`: the machine
// takes it from the event that asks for the key, the value crosses to the host
// as the name the enum declares for it, and the generated adapter reads it back
// into the machine's own enum by that name, so the host is handed the variant
// and never a text it would have to judge.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::Engine;
use sce_rust_tests::integration::static_datamodel::static_host_cose_sm::{
    StaticHostCoseHostInvokers as _, StaticHostCoseKeyGenerateRequest as GenerateRequest,
    StaticHostCoseKeyGenerateResult as GenerateResult, StaticHostCoseKeyUseEnum as KeyUse,
    StaticHostCosePolicy as Policy, StaticHostCoseSceHostKeyGenerateInvoker as KeyGenerateInvoker,
};

/// A host of key generation through the generated typed interface: records
/// each request it is handed and answers it with a key id at once.
struct RecordingHost(Arc<Mutex<Vec<GenerateRequest>>>);

impl KeyGenerateInvoker for RecordingHost {
    fn start_key_generate(
        &mut self,
        request: GenerateRequest,
        _token: u64,
    ) -> Option<GenerateResult> {
        self.0.lock().expect("generation requests").push(request);
        Some(GenerateResult {
            key_id: "generated:k".into(),
            ..GenerateResult::default()
        })
    }

    fn cancel_key_generate(&mut self, _token: u64) {}
}

#[test]
fn each_use_reaches_the_host_as_the_variant_its_name_declares() {
    let requests: Arc<Mutex<Vec<GenerateRequest>>> = Arc::default();
    let mut engine = Engine::new(Policy::new());
    engine.register_sce_host_key_generate_invoker(RecordingHost(Arc::clone(&requests)));
    let Ok(()) = engine.initialize();
    engine.step();
    let uses = [
        ("sign", KeyUse::Sign),
        ("seal", KeyUse::Seal),
        ("mac", KeyUse::Mac),
        ("derive", KeyUse::Derive),
    ];
    for (name, _) in &uses {
        engine.raise_external_by_name(
            "host.key.generate.request",
            &format!(r#"{{"key_use":"{name}","label":"device"}}"#),
        );
        engine.step();
        engine.step();
    }
    let requests = requests.lock().expect("generation requests");
    let handed: Vec<KeyUse> = requests.iter().map(|request| request.key_use).collect();
    let wanted: Vec<KeyUse> = uses.iter().map(|(_, variant)| *variant).collect();
    assert_eq!(handed, wanted, "the uses the host was handed, in order");
    assert!(
        requests.iter().all(|request| request.label == "device"),
        "the text beside the enum reached the host as written: {requests:?}"
    );
}
