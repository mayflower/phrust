//! Deliberately cheap streaming Generic compilation policy.
//!
//! The Generic backend shares semantic instruction emitters with the
//! optimizing backend, but it does not share the optimizing value-placement
//! contract. Values crossing real CFG edges are materialized in the native
//! fragment frame so Cranelift never has to construct function-wide SSA live
//! ranges for PHP locals and virtual registers.

use super::CraneliftLoweringError;
use super::executable_region::DefinedRegionFunction;
use crate::region_ir::NativeCompilerTier;

type FragmentEmitter<'a> =
    dyn FnMut(NativeCompilationMode) -> Result<DefinedRegionFunction, CraneliftLoweringError> + 'a;

/// Stable compiler mode included in diagnostics and persistent identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeCompilationMode {
    StreamingGeneric,
    SsaOptimizing,
}

impl NativeCompilationMode {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::StreamingGeneric => "streaming-generic",
            Self::SsaOptimizing => "ssa-optimizing",
        }
    }

    pub(super) const fn specialization(self) -> &'static str {
        match self {
            Self::StreamingGeneric => super::native_linkage::GENERIC_FUNCTION_SPECIALIZATION,
            Self::SsaOptimizing => "ssa-optimizing-v1",
        }
    }

    pub(super) const fn streams_cfg_state_through_slots(self) -> bool {
        matches!(self, Self::StreamingGeneric)
    }
}

/// Common fragment compiler contract. The optimizing implementation remains
/// an interface on this branch; the parallel hot-native work owns its policy.
pub(super) trait NativeFragmentCompiler {
    fn mode(&self) -> NativeCompilationMode;

    fn compile_fragment(
        &self,
        emit: &mut FragmentEmitter<'_>,
    ) -> Result<DefinedRegionFunction, CraneliftLoweringError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct StreamingGenericCompiler;

impl NativeFragmentCompiler for StreamingGenericCompiler {
    fn mode(&self) -> NativeCompilationMode {
        NativeCompilationMode::StreamingGeneric
    }

    fn compile_fragment(
        &self,
        emit: &mut FragmentEmitter<'_>,
    ) -> Result<DefinedRegionFunction, CraneliftLoweringError> {
        emit(NativeCompilationMode::StreamingGeneric)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SsaOptimizingCompiler;

impl NativeFragmentCompiler for SsaOptimizingCompiler {
    fn mode(&self) -> NativeCompilationMode {
        NativeCompilationMode::SsaOptimizing
    }

    fn compile_fragment(
        &self,
        emit: &mut FragmentEmitter<'_>,
    ) -> Result<DefinedRegionFunction, CraneliftLoweringError> {
        emit(NativeCompilationMode::SsaOptimizing)
    }
}

static STREAMING_GENERIC_COMPILER: StreamingGenericCompiler = StreamingGenericCompiler;
static SSA_OPTIMIZING_COMPILER: SsaOptimizingCompiler = SsaOptimizingCompiler;

pub(super) fn compiler_for_tier(tier: NativeCompilerTier) -> &'static dyn NativeFragmentCompiler {
    match tier {
        NativeCompilerTier::Generic => &STREAMING_GENERIC_COMPILER,
        NativeCompilerTier::Optimizing => &SSA_OPTIMIZING_COMPILER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_select_distinct_stable_compilation_modes() {
        let generic = compiler_for_tier(NativeCompilerTier::Generic);
        let optimizing = compiler_for_tier(NativeCompilerTier::Optimizing);
        assert_eq!(generic.mode().as_str(), "streaming-generic");
        assert_eq!(optimizing.mode().as_str(), "ssa-optimizing");
        assert_ne!(
            generic.mode().specialization(),
            optimizing.mode().specialization()
        );
    }
}
