//! Benchmark-only crate for performance Rust hot paths.
//!
//! Runtime crates must not depend on Criterion. This package keeps benchmark
//! dependencies isolated from the engine crates while still exercising their
//! public APIs.
