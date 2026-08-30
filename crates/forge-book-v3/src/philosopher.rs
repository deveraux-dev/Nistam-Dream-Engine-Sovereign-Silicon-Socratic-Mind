//! Sovereign Philosopher: Jungian-Stoic narrative synthesis via local Gemma model.

use crate::gemma_client::{GemmaClient, GemmaError};
use serde::{Deserialize, Serialize};

/// Request struct for synthesis generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisRequest {
    /// Schema version for compatibility tracking.
    pub request_schema_version: String,
    /// Local entity's DNA/context.
    pub local_dna: serde_json::Value,
    /// Peer entity's DNA/context.
    pub peer_dna: serde_json::Value,
    /// Resonance score 0–100 used in prompt context.
    pub resonance_score: u32,
}

/// Response struct for synthesis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisResponse {
    /// Schema version for compatibility tracking.
    pub response_schema_version: String,
    /// Synthesized narrative text.
    pub text: String,
    /// Archetype bond label.
    pub archetype_bond: String,
    /// Model ID that generated this response.
    pub model_id: String,
}

/// Fallback response when synthesis cannot be generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackArtifact {
    /// Schema version for compatibility tracking.
    pub response_schema_version: String,
    /// Error code identifying the failure mode.
    pub error_code: String,
    /// Human-readable error message.
    pub message: String,
}

/// The Sovereign Philosopher generates Jungian-Stoic narrative synthesis.
pub struct SovereignPhilosopher {
    client: GemmaClient,
}

impl SovereignPhilosopher {
    /// Create a new philosopher with the default localhost Gemma sidecar.
    pub fn new() -> Self {
        Self {
            client: GemmaClient::localhost_13017(),
        }
    }

    /// Create a new philosopher with a custom Gemma client.
    pub fn with_client(client: GemmaClient) -> Self {
        Self { client }
    }

    fn build_prompt(&self, request: &SynthesisRequest) -> String {
        format!(
            "TASK: Synthesize a narrative for two entities based on their Astrological DNA.\n\
             RESONANCE: {}%\n\
             STYLE: Jungian-Stoic, dense, poetic, Modern Mystic.\n\
             FORMAT: JSON with 'archetype_bond' (short title) and 'text' (narrative paragraph).",
            request.resonance_score
        )
    }

    /// Generate a synthesis response synchronously by calling the Gemma sidecar.
    ///
    /// Returns a `SynthesisResponse` on success or a `FallbackArtifact` on any error.
    pub fn generate_synthesis(&self, request: &SynthesisRequest) -> Result<SynthesisResponse, FallbackArtifact> {
        let prompt = self.build_prompt(request);

        match self.client.infer(&prompt) {
            Ok(raw_text) => {
                match serde_json::from_str::<serde_json::Value>(&raw_text) {
                    Ok(parsed) => {
                        let text = parsed
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Signal fragmented.")
                            .to_string();
                        let archetype_bond = parsed
                            .get("archetype_bond")
                            .and_then(|v| v.as_str())
                            .unwrap_or("UNKNOWN_BOND")
                            .to_string();

                        Ok(SynthesisResponse {
                            response_schema_version: "1.0".to_string(),
                            text,
                            archetype_bond,
                            model_id: "gemma-local".to_string(),
                        })
                    }
                    Err(_) => Ok(SynthesisResponse {
                        response_schema_version: "1.0".to_string(),
                        text: raw_text,
                        archetype_bond: "UNSTRUCTURED_SIGNAL".to_string(),
                        model_id: "gemma-local".to_string(),
                    }),
                }
            }
            Err(e) => Err(FallbackArtifact {
                response_schema_version: "1.0".to_string(),
                error_code: match &e {
                    GemmaError::SidecarUnreachable(_) => "SIDECAR_UNREACHABLE".to_string(),
                    GemmaError::FrameWrite(_) => "FRAME_WRITE_ERROR".to_string(),
                    GemmaError::FrameRead(_) => "FRAME_READ_ERROR".to_string(),
                    GemmaError::InvalidUtf8(_) => "RESPONSE_ENCODING_ERROR".to_string(),
                    GemmaError::InvalidResponse(_) => "SIDECAR_ERROR".to_string(),
                },
                message: e.to_string(),
            }),
        }
    }
}

impl Default for SovereignPhilosopher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn philosopher_creates_default() {
        let _p = SovereignPhilosopher::new();
    }

    #[test]
    fn synthesis_request_builds_prompt() {
        let p = SovereignPhilosopher::new();
        let req = SynthesisRequest {
            request_schema_version: "1.0".to_string(),
            local_dna: serde_json::json!({}),
            peer_dna: serde_json::json!({}),
            resonance_score: 85,
        };
        let prompt = p.build_prompt(&req);
        assert!(prompt.contains("85%"));
        assert!(prompt.contains("Jungian-Stoic"));
    }

    #[test]
    fn synthesis_response_fallback_on_sidecar_down() {
        let p = SovereignPhilosopher::new();
        let req = SynthesisRequest {
            request_schema_version: "1.0".to_string(),
            local_dna: serde_json::json!({}),
            peer_dna: serde_json::json!({}),
            resonance_score: 50,
        };

        let result = p.generate_synthesis(&req);
        assert!(result.is_err());
        if let Err(fallback) = result {
            assert!(!fallback.error_code.is_empty());
            assert!(!fallback.message.is_empty());
        }
    }
}
