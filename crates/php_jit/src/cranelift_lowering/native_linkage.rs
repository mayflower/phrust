//! Process-stable native function identities and indirection cells.

use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};

/// Complete symbolic identity for one published PHP native function version.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NativeFunctionKey {
    pub deployment_unit: String,
    pub function_id: u32,
    pub signature_hash: u64,
    pub compiler_tier: String,
    pub version: String,
    pub invalidation_generation: u64,
}

/// Target slot selected by the compiler tier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeFunctionTier {
    Baseline,
    Optimized,
}

/// Publication state for diagnostics and safe invalidation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NativeIndirectionState {
    Unpublished = 0,
    Published = 1,
    Retired = 2,
}

impl NativeIndirectionState {
    fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Published,
            2 => Self::Retired,
            _ => Self::Unpublished,
        }
    }
}

/// Generation- and signature-checked live indirection for one PHP function.
///
/// Persistent artifacts contain only [`NativeFunctionKey`] data. Process
/// addresses are published here after validation and never serialized.
#[derive(Debug)]
pub struct NativeIndirectionCell {
    key: NativeFunctionKey,
    generation: AtomicU64,
    baseline_target: AtomicUsize,
    optimized_target: AtomicUsize,
    state: AtomicU8,
}

impl NativeIndirectionCell {
    #[must_use]
    pub fn new(key: NativeFunctionKey) -> Self {
        Self {
            generation: AtomicU64::new(key.invalidation_generation),
            key,
            baseline_target: AtomicUsize::new(0),
            optimized_target: AtomicUsize::new(0),
            state: AtomicU8::new(NativeIndirectionState::Unpublished as u8),
        }
    }

    #[must_use]
    pub fn key(&self) -> &NativeFunctionKey {
        &self.key
    }

    pub fn publish(&self, tier: NativeFunctionTier, generation: u64, address: usize) {
        match tier {
            NativeFunctionTier::Baseline => self.baseline_target.store(address, Ordering::Release),
            NativeFunctionTier::Optimized => {
                self.optimized_target.store(address, Ordering::Release);
            }
        }
        self.generation.store(generation, Ordering::Release);
        self.state
            .store(NativeIndirectionState::Published as u8, Ordering::Release);
    }

    /// Resolves the best target only for an exact ABI and deployment generation.
    #[must_use]
    pub fn resolve(&self, signature_hash: u64, generation: u64) -> Option<usize> {
        if signature_hash != self.key.signature_hash
            || self.generation.load(Ordering::Acquire) != generation
            || self.state() != NativeIndirectionState::Published
        {
            return None;
        }
        let optimized = self.optimized_target.load(Ordering::Acquire);
        (optimized != 0).then_some(optimized).or_else(|| {
            let baseline = self.baseline_target.load(Ordering::Acquire);
            (baseline != 0).then_some(baseline)
        })
    }

    /// Clears live addresses before the owning code generation can retire.
    pub fn retire(&self) {
        self.state
            .store(NativeIndirectionState::Retired as u8, Ordering::Release);
        self.optimized_target.store(0, Ordering::Release);
        self.baseline_target.store(0, Ordering::Release);
    }

    #[must_use]
    pub fn state(&self) -> NativeIndirectionState {
        NativeIndirectionState::from_raw(self.state.load(Ordering::Acquire))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> NativeFunctionKey {
        NativeFunctionKey {
            deployment_unit: "unit-a".to_owned(),
            function_id: 7,
            signature_hash: 11,
            compiler_tier: "baseline".to_owned(),
            version: "v1".to_owned(),
            invalidation_generation: 3,
        }
    }

    #[test]
    fn cell_validates_signature_and_generation_and_retires() {
        let cell = NativeIndirectionCell::new(key());
        assert_eq!(cell.resolve(11, 3), None);
        cell.publish(NativeFunctionTier::Baseline, 3, 0x1234);
        assert_eq!(cell.resolve(11, 3), Some(0x1234));
        assert_eq!(cell.resolve(12, 3), None);
        assert_eq!(cell.resolve(11, 4), None);
        cell.retire();
        assert_eq!(cell.resolve(11, 3), None);
    }
}
