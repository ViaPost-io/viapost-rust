use std::{fs, path::Path};

#[test]
fn release_is_created_only_after_verification_and_attestation() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml");
    if !path.exists() {
        return;
    }

    let workflow = fs::read_to_string(path).expect("release workflow should be readable");

    assert!(workflow.contains("tags: ['v*']"));
    assert!(workflow.contains("needs: [verify-and-build, attest-build-provenance]"));
    assert!(workflow.contains("subject-path: dist/*"));
    assert!(workflow.contains("gh release create \"$RELEASE_TAG\""));
    assert!(!workflow.contains("types: [published]"));
    assert!(!workflow.contains("--clobber"));
    assert!(workflow.contains("github.event_name == 'push' && github.sha || inputs.tag"));
    assert!(workflow.contains("EVENT_SHA: ${{ github.event_name == 'push' && github.sha || '' }}"));
    assert!(workflow.contains("gh release download \"$RELEASE_TAG\""));
    assert!(workflow.contains("gh release view \"$RELEASE_TAG\" --json isDraft --jq .isDraft"));
    assert!(workflow.contains("Reproduce the attested registry package"));
    assert!(workflow.contains("cargo package --locked"));
    assert!(workflow.contains("cmp --silent \"$attested_package\" \"$reproduced_package\""));
    assert!(workflow.contains("cargo publish --locked --no-verify"));
    assert!(!workflow.contains("--token \"$CARGO_REGISTRY_TOKEN\""));
}

#[test]
fn contract_drift_uses_the_published_public_contract() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/contract-drift.yml");
    if !path.exists() {
        return;
    }

    let workflow = fs::read_to_string(path).expect("contract workflow should be readable");

    assert!(workflow.contains("https://docs.viapost.io/openapi/public.yaml"));
    assert!(!workflow.contains("raw.githubusercontent.com/ViaPost-io/base-code"));
}
