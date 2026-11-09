//! Explicit opt-in OCI backend identity for bounded Protocol 0.2 experiments.
//! Does not replace or impersonate the frozen Conda Phase 1 identity.

use crate::backend::{BackendError, MooseProcessRunner};

pub const APPROVED_CANDIDATE_IMAGE: &str =
    "idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialOciProfile {
    image: String,
}

impl OfficialOciProfile {
    /// This experimental profile is pinned to a digest that passed independent
    /// real-solver Thermal smoke and V&V CI. Arbitrary images are rejected.
    pub fn candidate(image: &str) -> Result<Self, String> {
        if image != APPROVED_CANDIDATE_IMAGE {
            return Err(format!(
                "unapproved MOOSE OCI identity: expected {APPROVED_CANDIDATE_IMAGE}"
            ));
        }
        Ok(Self {
            image: image.to_owned(),
        })
    }

    pub fn image(&self) -> &str {
        &self.image
    }

    /// Runtime probe is independent of Conda metadata and fails closed.
    /// The caller must supply an executable that invokes this exact digest;
    /// the official OCI CI wrapper enforces that external binding.
    pub fn probe(&self, runner: &MooseProcessRunner) -> Result<(), BackendError> {
        let version = runner.run(&["--version"])?;
        if !version.success() || version.stdout_text().trim().is_empty() {
            return Err(BackendError::Probe(
                "official OCI MOOSE --version failed or produced no output".to_owned(),
            ));
        }
        let help = runner.run(&["--help"])?;
        let text = format!("{}{}", help.stdout_text(), help.stderr_text());
        if !help.success() || !text.contains("--check-input") {
            return Err(BackendError::Probe(
                "official OCI MOOSE --help does not support --check-input".to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{OfficialOciProfile, APPROVED_CANDIDATE_IMAGE};

    #[test]
    fn only_approved_immutable_digest_is_accepted() {
        assert!(OfficialOciProfile::candidate(APPROVED_CANDIDATE_IMAGE).is_ok());
        for candidate in [
            "idaholab/moose:latest",
            "idaholab/moose@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c",
            "",
        ] {
            assert!(OfficialOciProfile::candidate(candidate).is_err());
        }
    }
}
