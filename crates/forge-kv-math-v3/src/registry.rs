//! Semantic-Codepoint-Kernel Bridge — SEMANTIC-CODEPOINT-KERNEL-BRIDGE-001/002.
//!
//! Maps each semantic primitive (identified by a PUA Unicode codepoint in the
//! U+E100 block, Private Use Area) to its integer domain, CPU symbol, WGSL kernel
//! source (compile-time validated via `include_str!`), and proof corpus.
//!
//! `determinism_proof` consumes this registry for claim labels and kernel sources
//! instead of hard-coding either, bridging the semantic authority layer to the
//! deterministic GPU proof layer.

use std::fmt;

// ── Stable primitive identifiers ─────────────────────────────────────────────

/// Stable identifier for a semantic primitive.
///
/// The discriminant IS the PUA Unicode codepoint (U+E100 block). Using the
/// codepoint as the discriminant means the enum variant and the codepoint are
/// always in sync — you cannot have one without the other. Prefer these IDs over
/// array indexes when referencing registry entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SemanticPrimitive {
    /// Integer hash over two u32 inputs. Invention #7.
    PrismaticHashU32       = 0xE101,
    /// Signed 64-bit Permyriad `pos * ratio / 10000`. Invention #156.
    PermyriadMulDivI64     = 0xE102,
    /// FNV-1a semantic-key codepoints through Permyriad GPU arithmetic.
    StatCodepointPermyriad = 0xE103,
}

impl SemanticPrimitive {
    /// Stable name string. Used as the claim label in `determinism_proof`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PrismaticHashU32       => "prismatic_hash_u32",
            Self::PermyriadMulDivI64     => "permyriad_mul_div_i64",
            Self::StatCodepointPermyriad => "stat_codepoint_permyriad",
        }
    }
}

impl fmt::Display for SemanticPrimitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "U+{:04X}", *self as u32)
    }
}

// ── Integer domain ───────────────────────────────────────────────────────────

/// Integer domain a semantic primitive operates in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerDomain {
    /// Unsigned 32-bit — wrapping arithmetic, fully defined in WGSL.
    U32,
    /// Signed 64-bit — Permyriad `pos * ratio / 10000` pattern.
    I64,
}

impl fmt::Display for IntegerDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::U32 => f.write_str("U32"),
            Self::I64 => f.write_str("I64"),
        }
    }
}

// ── Kernel sources ───────────────────────────────────────────────────────────

/// WGSL kernel source(s) for a registry entry.
///
/// Sources are embedded at compile time via `include_str!`. If a referenced file
/// is missing the crate fails to compile, giving structural validation of every
/// kernel path declared in the registry.
pub enum KernelSrc {
    /// One kernel covers this primitive entirely.
    Single {
        /// Kernel file name for display.
        name: &'static str,
        /// Embedded WGSL source.
        src:  &'static str,
    },
    /// Two paths: native (SHADER_INT64-gated) and emulated (vec2<u32>, portable).
    NativeEmulated {
        /// Native kernel file name for display.
        native_name:   &'static str,
        /// Embedded native (SHADER_INT64) WGSL source.
        native_src:    &'static str,
        /// Emulated kernel file name for display.
        emulated_name: &'static str,
        /// Embedded emulated (vec2<u32>) WGSL source.
        emulated_src:  &'static str,
    },
}

impl KernelSrc {
    /// Display name for the kernel file(s). Used in proof output.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Single { name, .. }              => name,
            Self::NativeEmulated { emulated_name, .. } => emulated_name,
        }
    }

    /// Source for a Single primitive. Panics if NativeEmulated.
    pub fn as_single_src(&self) -> &'static str {
        match self {
            Self::Single { src, .. }       => src,
            Self::NativeEmulated { .. }    => panic!("kernel is NativeEmulated, not Single"),
        }
    }

    /// Native (SHADER_INT64) source. Panics if Single.
    pub fn as_native_src(&self) -> &'static str {
        match self {
            Self::NativeEmulated { native_src, .. } => native_src,
            Self::Single { .. }                     => panic!("kernel is Single, not NativeEmulated"),
        }
    }

    /// Emulated (vec2<u32>) source. Panics if Single.
    pub fn as_emulated_src(&self) -> &'static str {
        match self {
            Self::NativeEmulated { emulated_src, .. } => emulated_src,
            Self::Single { .. }                       => panic!("kernel is Single, not NativeEmulated"),
        }
    }

    /// Portable fallback: emulated if NativeEmulated, src if Single.
    pub fn portable_src(&self) -> &'static str {
        match self {
            Self::Single { src, .. }                  => src,
            Self::NativeEmulated { emulated_src, .. } => emulated_src,
        }
    }
}

impl fmt::Debug for KernelSrc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single { name, .. } => write!(f, "Single({name})"),
            Self::NativeEmulated { native_name, emulated_name, .. } => {
                write!(f, "NativeEmulated({native_name} / {emulated_name})")
            }
        }
    }
}

// ── Registry entry ───────────────────────────────────────────────────────────

/// One entry in the semantic-codepoint-kernel bridge.
pub struct KernelEntry {
    /// Stable identifier. The discriminant IS the PUA Unicode codepoint.
    pub id: SemanticPrimitive,
    /// Stable semantic name. The claim label in `determinism_proof`.
    pub name: &'static str,
    /// 13forge invention registry number, where applicable.
    pub invention: Option<u32>,
    /// Integer domain.
    pub domain: IntegerDomain,
    /// Canonical CPU-side function symbol.
    pub cpu_symbol: &'static str,
    /// WGSL entry-point function name.
    pub wgsl_symbol: &'static str,
    /// WGSL kernel file name(s) for display.
    pub wgsl_kernel: &'static str,
    /// Compile-time-validated WGSL source(s). Build fails if any file is missing.
    pub kernel_src: KernelSrc,
    /// Short corpus identifier printed in proof output.
    pub corpus_name: &'static str,
    /// Human-readable corpus description.
    pub proof_corpus: &'static str,
}

// ── Registry ─────────────────────────────────────────────────────────────────

/// Semantic-Codepoint-Kernel bridge registry.
///
/// `kernel_src` fields are embedded via `include_str!` — the build fails if any
/// referenced WGSL file is missing (compile-time file-existence validation).
/// Order is stable; do not reorder, append only.
pub static REGISTRY: [KernelEntry; 3] = [
    KernelEntry {
        id:          SemanticPrimitive::PrismaticHashU32,
        name:        "prismatic_hash_u32",
        invention:   Some(7),
        domain:      IntegerDomain::U32,
        cpu_symbol:  "prismatic_hash",
        wgsl_symbol: "main_cs",
        wgsl_kernel: "kernel.wgsl",
        kernel_src:  KernelSrc::Single {
            name: "kernel.wgsl",
            src:  include_str!("../proof/kernel.wgsl"),
        },
        corpus_name:  "u32-golden-ratio-4k",
        proof_corpus: "4096 u32 pairs: index i XOR 0xABCD1234, \
                       wrapping multiply chain through golden-ratio constants",
    },
    KernelEntry {
        id:          SemanticPrimitive::PermyriadMulDivI64,
        name:        "permyriad_mul_div_i64",
        invention:   Some(156),
        domain:      IntegerDomain::I64,
        cpu_symbol:  "cpu_i64",
        wgsl_symbol: "main_cs",
        wgsl_kernel: "kernel_i64_native.wgsl / kernel_i64_emu.wgsl",
        kernel_src:  KernelSrc::NativeEmulated {
            native_name:   "kernel_i64_native.wgsl",
            native_src:    include_str!("../proof/kernel_i64_native.wgsl"),
            emulated_name: "kernel_i64_emu.wgsl",
            emulated_src:  include_str!("../proof/kernel_i64_emu.wgsl"),
        },
        corpus_name:  "i64-adversarial-256",
        proof_corpus: "256 adversarial i64 pairs: negatives, >i32::MAX, \
                       wrapping-mul overflow, truncate-toward-zero divide; \
                       proven on both native SHADER_INT64 and vec2<u32> emulated paths",
    },
    KernelEntry {
        id:          SemanticPrimitive::StatCodepointPermyriad,
        name:        "stat_codepoint_permyriad",
        invention:   None,
        domain:      IntegerDomain::I64,
        cpu_symbol:  "fnv1a",
        wgsl_symbol: "main_cs",
        wgsl_kernel: "kernel_i64_emu.wgsl",
        kernel_src:  KernelSrc::Single {
            name: "kernel_i64_emu.wgsl",
            src:  include_str!("../proof/kernel_i64_emu.wgsl"),
        },
        corpus_name:  "i64-fnv1a-codepoints-256",
        proof_corpus: "256 i64 pairs derived from FNV-1a codepoints of 16 canonical \
                       semantic key names (high-bit u64 values span the negative i64 range); \
                       ratio = 10000 - (i % 100); proves semantic-layer codepoints are \
                       safe operands for GPU Permyriad arithmetic",
    },
];

/// Look up a registry entry by stable SemanticPrimitive ID.
///
/// Panics if the variant is somehow absent, which cannot happen as long as
/// every SemanticPrimitive variant appears in REGISTRY.
pub fn entry(id: SemanticPrimitive) -> &'static KernelEntry {
    REGISTRY
        .iter()
        .find(|e| e.id == id)
        .expect("all SemanticPrimitive variants must be present in REGISTRY")
}
