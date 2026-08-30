//! AstraKey Sieve — deterministic content generation via prime numbers.
//!
//! Layer 1: Sieve of Eratosthenes + HMAC-SHA256 seed derivation.
//! All values are integers. No floats in the core path.
//!
//! Layer 2 (ASP constraint validation) stays offline in Python/Clingo.
//! The runtime consumes pre-validated SeedPacks or derives on the fly.

pub mod sieve;
pub mod derivation;
pub mod types;
