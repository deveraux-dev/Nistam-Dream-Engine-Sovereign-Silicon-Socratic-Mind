//! ForgeVisionMap trait and implementation for WorldGenState.
//!
//! Feature-gated behind the `wright_stats` feature flag. Provides engine-wide
//! variable inspection of world generation state.

use crate::state::WorldGenState;

/// A value returned by `wright_read`.
#[derive(Debug, Clone, PartialEq)]
pub enum WrightValue {
    I64(i64),
    F64(f64),
    Bool(bool),
}

/// Schema entry describing a readable variable.
#[derive(Debug, Clone)]
pub struct WrightVariable {
    pub name: String,
    pub description: String,
}

/// Trait for exposing subsystem state to the Wright validation runner.
pub trait ForgeVisionMap {
    /// Return the schema of all readable variables.
    fn wright_variables(&self) -> Vec<WrightVariable>;

    /// Read a variable by name. Returns `Err` for unknown keys.
    fn wright_read(&self, key: &str) -> Result<WrightValue, String>;
}

impl ForgeVisionMap for WorldGenState {
    fn wright_variables(&self) -> Vec<WrightVariable> {
        vec![
            WrightVariable {
                name: "worldgen.chunk_count".to_string(),
                description: "Number of chunks generated".to_string(),
            },
            WrightVariable {
                name: "worldgen.void_ratio".to_string(),
                description: "Ratio of void voxels to total voxels".to_string(),
            },
            WrightVariable {
                name: "worldgen.avg_density".to_string(),
                description: "Weighted average biome density across all matter".to_string(),
            },
            WrightVariable {
                name: "worldgen.determinism_ok".to_string(),
                description: "Whether determinism verification passed".to_string(),
            },
            WrightVariable {
                name: "worldgen.biome_barren".to_string(),
                description: "Count of barren biome voxels".to_string(),
            },
            WrightVariable {
                name: "worldgen.biome_sparse".to_string(),
                description: "Count of sparse biome voxels".to_string(),
            },
            WrightVariable {
                name: "worldgen.biome_moderate".to_string(),
                description: "Count of moderate biome voxels".to_string(),
            },
            WrightVariable {
                name: "worldgen.biome_dense".to_string(),
                description: "Count of dense biome voxels".to_string(),
            },
            WrightVariable {
                name: "worldgen.biome_lush".to_string(),
                description: "Count of lush biome voxels".to_string(),
            },
        ]
    }

    fn wright_read(&self, key: &str) -> Result<WrightValue, String> {
        match key {
            "worldgen.chunk_count" => Ok(WrightValue::I64(self.chunks_generated as i64)),
            "worldgen.void_ratio" => {
                #[cfg(feature = "wright_stats")]
                {
                    let total = self.total_voids + self.total_matter;
                    let ratio = if total == 0 {
                        0.0
                    } else {
                        self.total_voids as f64 / total as f64
                    };
                    Ok(WrightValue::F64(ratio))
                }
                #[cfg(not(feature = "wright_stats"))]
                {
                    Err("wright_stats feature not enabled".to_string())
                }
            }
            "worldgen.avg_density" => {
                #[cfg(feature = "wright_stats")]
                {
                    if self.total_matter == 0 {
                        Ok(WrightValue::F64(0.0))
                    } else {
                        let weights: [f64; 5] = [0.5, 2.0, 3.0, 4.0, 5.0];
                        let weighted_sum: f64 = self
                            .biome_histogram
                            .iter()
                            .enumerate()
                            .map(|(i, &count)| weights[i] * count as f64)
                            .sum();
                        let avg = weighted_sum / self.total_matter as f64;
                        Ok(WrightValue::F64(avg))
                    }
                }
                #[cfg(not(feature = "wright_stats"))]
                {
                    Err("wright_stats feature not enabled".to_string())
                }
            }
            "worldgen.determinism_ok" => Ok(WrightValue::Bool(self.determinism_verified)),
            "worldgen.biome_barren" => Ok(WrightValue::I64(self.biome_histogram[0] as i64)),
            "worldgen.biome_sparse" => Ok(WrightValue::I64(self.biome_histogram[1] as i64)),
            "worldgen.biome_moderate" => Ok(WrightValue::I64(self.biome_histogram[2] as i64)),
            "worldgen.biome_dense" => Ok(WrightValue::I64(self.biome_histogram[3] as i64)),
            "worldgen.biome_lush" => Ok(WrightValue::I64(self.biome_histogram[4] as i64)),
            _ => Err(format!("unknown wright variable: {}", key)),
        }
    }
}
