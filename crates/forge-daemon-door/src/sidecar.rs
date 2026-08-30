//! Local Gemma 9B sidecar client for perceptual evaluation.
//! Queries :13017 with structured prompts, parses RON responses deterministically.

use std::io::{Write, BufRead, BufReader};
use std::net::TcpStream;

/// Gemma 9B perceptual evaluation result.
#[derive(Debug, Clone)]
pub enum Evaluation {
    /// Palette meets the spec: reduced alarm, distinct, readable.
    Pass,
    /// Palette out of spec with directional feedback for agent iteration.
    Fail {
        /// Reason from Gemma: e.g., "Red L value too high".
        reason: String,
    },
}

/// Query Gemma 9B sidecar for palette perception judgment.
/// Returns binary gate + optional directional feedback.
pub fn evaluate_palette_perception(contract: &str, samples: &str) -> Result<Evaluation, String> {
    let prompt = format!(
        "Evaluate terminal colour palette perceptually.\n\n\
        Contract spec:\n{contract}\n\n\
        Actual samples:\n{samples}\n\n\
        Respond strictly in single-line RON format:\n\
        (status: Pass)\n\
        or\n\
        (status: Fail, reason: \"...\")"
    );

    let reply = sidecar_infer(&prompt)?;
    parse_evaluation_ron(&reply)
}

fn sidecar_infer(prompt: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect("127.0.0.1:13017")
        .map_err(|e| format!("sidecar unreachable (:13017): {e}"))?;

    let payload = format!("INFER {}", prompt);
    stream.write_all(payload.as_bytes())
        .map_err(|e| format!("send to sidecar: {e}"))?;

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        if let Ok(l) = line {
            if !l.is_empty() && !l.starts_with('[') {
                return Ok(l);
            }
        }
    }

    Err("sidecar returned no response".into())
}

fn parse_evaluation_ron(reply: &str) -> Result<Evaluation, String> {
    let trimmed = reply.trim();

    if trimmed.contains("status: Pass") || trimmed.to_uppercase().contains("PASS") {
        return Ok(Evaluation::Pass);
    }

    if trimmed.contains("status: Fail") || trimmed.to_uppercase().contains("FAIL") {
        // Extract reason if present
        let reason = if let Some(start) = trimmed.find("reason:") {
            let after_reason = &trimmed[start + 7..];
            if let Some(quote_start) = after_reason.find('"') {
                if let Some(quote_end) = after_reason[quote_start + 1..].find('"') {
                    after_reason[quote_start + 1..quote_start + 1 + quote_end].to_string()
                } else {
                    "No reason provided".to_string()
                }
            } else {
                "No reason provided".to_string()
            }
        } else {
            "Gemma judged the colours out of spec".to_string()
        };

        return Ok(Evaluation::Fail { reason });
    }

    Err(format!("Could not parse Gemma response: {}", trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pass_response() {
        assert!(matches!(
            parse_evaluation_ron("(status: Pass)"),
            Ok(Evaluation::Pass)
        ));
    }

    #[test]
    fn parse_fail_with_reason() {
        match parse_evaluation_ron("(status: Fail, reason: \"Red L value too high\")") {
            Ok(Evaluation::Fail { reason }) => {
                assert_eq!(reason, "Red L value too high");
            }
            _ => panic!("Expected Fail variant"),
        }
    }
}
