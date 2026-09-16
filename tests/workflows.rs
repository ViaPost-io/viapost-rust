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

#[test]
fn ci_and_release_validate_a_fresh_msrv_consumer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .expect("CI workflow should be readable");
    let release = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .expect("release workflow should be readable");
    let script = root.join("scripts/check-msrv-consumer.sh");

    assert!(script.exists());
    assert!(ci.contains("scripts/check-msrv-consumer.sh"));
    assert!(release.contains("scripts/check-msrv-consumer.sh"));
    assert!(release.contains("verify-fresh-msrv-consumer:"));
    assert!(release.contains("needs: verify-fresh-msrv-consumer"));
    assert!(release.contains("source_sha: ${{ steps.source.outputs.sha }}"));
    assert!(release.contains("ref: ${{ needs.verify-fresh-msrv-consumer.outputs.source_sha }}"));

    let producer = release
        .split("  verify-and-build:")
        .nth(1)
        .and_then(|section| section.split("  attest-build-provenance:").next())
        .expect("release artifact producer should exist");
    assert!(!producer.contains("scripts/check-msrv-consumer.sh"));
}
