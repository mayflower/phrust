//! Region IR optimization report model.

use crate::region_ir::NodeId;

/// Aggregate counters reported by the no-exec region optimizer prototype.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegionOptReport {
    /// Pure floating nodes inspected by GCM.
    pub nodes_considered: u64,
    /// Nodes assigned an early placement.
    pub nodes_scheduled_early: u64,
    /// Nodes assigned a late placement.
    pub nodes_scheduled_late: u64,
    /// Nodes kept at their existing pinned/control placement.
    pub nodes_kept_pinned: u64,
    /// Nodes rejected because effect or value semantics forbid movement.
    pub nodes_rejected_by_effects: u64,
    /// Natural loops detected in the conservative CFG.
    pub loops_detected: u64,
}

/// Stable per-node placement decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionScheduleDecision {
    /// Node being classified.
    pub node: NodeId,
    /// Machine-readable placement label.
    pub label: &'static str,
    /// Optional control anchor selected by GCM.
    pub anchor: Option<NodeId>,
    /// Short reason for pinned/rejected nodes.
    pub reason: &'static str,
}

impl RegionScheduleDecision {
    /// Creates a stable placement decision.
    #[must_use]
    pub const fn new(
        node: NodeId,
        label: &'static str,
        anchor: Option<NodeId>,
        reason: &'static str,
    ) -> Self {
        Self {
            node,
            label,
            anchor,
            reason,
        }
    }
}
