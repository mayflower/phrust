//! Function-scoped native compile planning.
//!
//! A production compile group contains exactly one PHP function.  The plan is
//! built before Cranelift lowering so compile breadth and structural cost are
//! explicit and testable instead of being inferred from a module afterwards.

use crate::region_ir::{RegionGraph, baseline_instruction_lowering, build_executable_ssa};
use php_ir::{BlockId, FunctionId, LocalId};
use std::collections::BTreeSet;

pub const BASELINE_FRAGMENT_MAX_PHP_BLOCKS: usize = 256;
// Generated top-level declaration blocks are semantically atomic in the
// current Region CFG and can contain tens of thousands of cheap declaration
// instructions. Keep a finite absolute ceiling for such a single block while
// the independent CLIF ceiling prevents helper/exception-heavy code from
// entering Cranelift without a structural bound. Ordinary multi-block PHP
// functions are still cut at the much smaller 256-block boundary.
pub const BASELINE_FRAGMENT_MAX_IR_INSTRUCTIONS: usize = 32_768;
pub const BASELINE_FRAGMENT_MAX_ESTIMATED_CLIF_BLOCKS: usize = 8_192;
pub const OPTIMIZING_REGION_MAX_PHP_BLOCKS: usize = 256;
pub const OPTIMIZING_REGION_MAX_IR_INSTRUCTIONS: usize = 1_500;
pub const OPTIMIZING_REGION_MAX_VIRTUAL_VALUES: usize = 768;

/// One bounded internal native fragment of a single PHP function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeFragmentPlan {
    pub id: u32,
    pub blocks: Vec<BlockId>,
    pub ir_instructions: usize,
    pub estimated_clif_blocks: usize,
}

impl NativeFragmentPlan {
    #[must_use]
    pub fn is_within_budget(&self) -> bool {
        self.blocks.len() <= BASELINE_FRAGMENT_MAX_PHP_BLOCKS
            && self.ir_instructions <= BASELINE_FRAGMENT_MAX_IR_INSTRUCTIONS
            && self.estimated_clif_blocks <= BASELINE_FRAGMENT_MAX_ESTIMATED_CLIF_BLOCKS
    }
}

/// Pre-Cranelift structural estimate for one PHP function compile group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCompilePlan {
    /// The only PHP function body admitted to this compile group.
    pub function: FunctionId,
    pub ir_instructions: usize,
    pub php_cfg_blocks: usize,
    pub estimated_clif_blocks: usize,
    pub virtual_values: usize,
    pub maximum_estimated_live_set: usize,
    pub safepoint_count: usize,
    pub safepoint_live_set_sum: usize,
    pub phi_count: usize,
    pub exception_regions: usize,
    pub suspension_points: usize,
    pub call_sites: usize,
    pub estimated_helper_branches: usize,
    pub fragments: Vec<NativeFragmentPlan>,
}

impl NativeCompilePlan {
    /// Returns whether whole-region SSA is structurally bounded.
    #[must_use]
    pub fn permits_whole_region_optimization(&self) -> bool {
        self.fragments.len() == 1
            && self.php_cfg_blocks <= OPTIMIZING_REGION_MAX_PHP_BLOCKS
            && self.ir_instructions <= OPTIMIZING_REGION_MAX_IR_INSTRUCTIONS
            && self.virtual_values <= OPTIMIZING_REGION_MAX_VIRTUAL_VALUES
    }

    /// Builds the mandatory plan for one already verified Region graph.
    #[must_use]
    pub fn for_region(region: &RegionGraph) -> Self {
        let instructions = region
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .collect::<Vec<_>>();
        let safepoints = instructions
            .iter()
            .copied()
            .filter(|instruction| {
                baseline_instruction_lowering(&instruction.source_kind).requires_safepoint
            })
            .collect::<Vec<_>>();
        let safepoint_live_set_sum = safepoints
            .iter()
            .map(|instruction| instruction.live_locals.len())
            .sum();
        let maximum_estimated_live_set = instructions
            .iter()
            .map(|instruction| {
                instruction
                    .live_locals
                    .len()
                    .saturating_add(instruction.register_uses().len())
            })
            .max()
            .unwrap_or(0)
            .max(region.params.len());
        let eligible_locals = (0..region.local_count)
            .map(LocalId::new)
            .collect::<BTreeSet<_>>();
        let phi_count = build_executable_ssa(region, &eligible_locals).phi_count();
        let suspension_points = instructions
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.kind,
                    crate::region_ir::RegionInstructionKind::NativeSuspend(_)
                )
            })
            .count();
        let call_sites = instructions
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.kind,
                    crate::region_ir::RegionInstructionKind::NativeCall(_)
                )
            })
            .count();
        let estimated_helper_branches = safepoints.len();
        let native_transition_points = instructions
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.kind,
                    crate::region_ir::RegionInstructionKind::Binary { .. }
                )
            })
            .count();
        let handler_resume_points = region
            .exception_regions
            .iter()
            .flat_map(|handler| [handler.catch, handler.finally])
            .flatten()
            .collect::<BTreeSet<_>>()
            .len();
        let osr_entries = region.osr_entries().len();
        let resume_dispatch_points = handler_resume_points
            .saturating_add(suspension_points)
            .saturating_add(native_transition_points)
            .saturating_add(osr_entries);
        // Ordinary instructions and terminators remain in their real PHP CFG
        // blocks. Extra blocks are reserved for fallible-helper continuations,
        // actual native resume entries, and the native entry dispatcher.
        let estimated_clif_blocks = region
            .blocks
            .len()
            .saturating_add(1)
            .saturating_add(native_transition_points)
            .saturating_add(suspension_points)
            .saturating_add(resume_dispatch_points.saturating_mul(2))
            .saturating_add(estimated_helper_branches.saturating_mul(2))
            .saturating_add(4);
        let mut fragments = Vec::<NativeFragmentPlan>::new();
        let mut current_blocks = Vec::new();
        let mut current_instructions = 0_usize;
        let mut current_clif_blocks = 1_usize;
        for block in &region.blocks {
            let block_instructions = block.instructions.len();
            let block_safepoints = block
                .instructions
                .iter()
                .filter(|instruction| {
                    baseline_instruction_lowering(&instruction.source_kind).requires_safepoint
                })
                .count();
            let block_transitions = block
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(
                        instruction.kind,
                        crate::region_ir::RegionInstructionKind::Binary { .. }
                    )
                })
                .count();
            let block_suspensions = block
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(
                        instruction.kind,
                        crate::region_ir::RegionInstructionKind::NativeSuspend(_)
                    )
                })
                .count();
            let block_clif_blocks = 1_usize
                .saturating_add(block_safepoints)
                .saturating_add(block_transitions.saturating_mul(3))
                .saturating_add(block_suspensions.saturating_mul(3));
            let exceeds_budget = !current_blocks.is_empty()
                && (current_blocks.len().saturating_add(1) > BASELINE_FRAGMENT_MAX_PHP_BLOCKS
                    || current_instructions.saturating_add(block_instructions)
                        > BASELINE_FRAGMENT_MAX_IR_INSTRUCTIONS
                    || current_clif_blocks.saturating_add(block_clif_blocks)
                        > BASELINE_FRAGMENT_MAX_ESTIMATED_CLIF_BLOCKS);
            if exceeds_budget {
                fragments.push(NativeFragmentPlan {
                    id: u32::try_from(fragments.len()).unwrap_or(u32::MAX),
                    blocks: std::mem::take(&mut current_blocks),
                    ir_instructions: current_instructions,
                    estimated_clif_blocks: current_clif_blocks,
                });
                current_instructions = 0;
                current_clif_blocks = 1;
            }
            current_blocks.push(block.id);
            current_instructions = current_instructions.saturating_add(block_instructions);
            current_clif_blocks = current_clif_blocks.saturating_add(block_clif_blocks);
        }
        if !current_blocks.is_empty() {
            fragments.push(NativeFragmentPlan {
                id: u32::try_from(fragments.len()).unwrap_or(u32::MAX),
                blocks: current_blocks,
                ir_instructions: current_instructions,
                estimated_clif_blocks: current_clif_blocks,
            });
        }

        Self {
            function: region.function,
            ir_instructions: instructions.len(),
            php_cfg_blocks: region.blocks.len(),
            estimated_clif_blocks,
            virtual_values: region.register_count as usize,
            maximum_estimated_live_set,
            safepoint_count: safepoints.len(),
            safepoint_live_set_sum,
            phi_count,
            exception_regions: region.exception_regions.len(),
            suspension_points,
            call_sites,
            estimated_helper_branches,
            fragments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region_ir::{BaselineRegionBuilder, CompileMetadata, NativeCompilerTier};
    use php_ir::{FunctionFlags, IrBuilder, IrSpan, UnitId};

    #[test]
    fn compile_plan_contains_exactly_the_requested_function() {
        let mut builder = IrBuilder::new(UnitId::new(1));
        let file = builder.add_file("layout.php");
        let span = IrSpan::new(file, 0, 1);
        for name in ["first", "second", "third"] {
            let function = builder.start_function(name, FunctionFlags::default(), span);
            let block = builder.append_block(function);
            builder.terminate_return(function, block, None, span);
        }
        let unit = builder.finish();
        let region = BaselineRegionBuilder::build(
            &unit,
            FunctionId::new(1),
            &CompileMetadata {
                ir_fingerprint: "plan-test".to_owned(),
                tier: NativeCompilerTier::Baseline,
                helper_abi_hash: 0,
                target_cpu: "test".to_owned(),
                semantic_config_hash: 0,
                dependency_identity: "test".to_owned(),
            },
        )
        .expect("region");
        let plan = NativeCompilePlan::for_region(&region);

        assert_eq!(plan.function, FunctionId::new(1));
        assert_eq!(plan.php_cfg_blocks, 1);
        assert_eq!(plan.ir_instructions, 0);
        assert_eq!(plan.fragments.len(), 1);
        assert_eq!(plan.fragments[0].blocks, vec![BlockId::new(0)]);
        assert!(plan.fragments[0].is_within_budget());
        assert!(plan.permits_whole_region_optimization());
    }
}
