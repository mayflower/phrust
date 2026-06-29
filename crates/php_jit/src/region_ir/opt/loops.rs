//! Conservative natural-loop detection for region IR.

use std::collections::{BTreeMap, BTreeSet};

use crate::region_ir::NodeId;

use super::{cfg::RegionCfg, dominators::DominatorTree};

/// One natural loop discovered from a backedge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionLoop {
    /// Loop header.
    pub header: NodeId,
    /// Backedge tail.
    pub backedge: NodeId,
    /// Nodes in the conservative loop body.
    pub body: BTreeSet<NodeId>,
}

/// Loop nesting data.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoopInfo {
    /// Natural loops in deterministic order.
    pub loops: Vec<RegionLoop>,
    loop_depths: BTreeMap<NodeId, usize>,
}

impl LoopInfo {
    /// Returns the loop nesting depth for a control node.
    #[must_use]
    pub fn depth(&self, node: NodeId) -> usize {
        self.loop_depths.get(&node).copied().unwrap_or(0)
    }
}

/// Detects backedge-derived natural loops.
#[must_use]
pub fn detect_loops(cfg: &RegionCfg, _dominators: &DominatorTree) -> LoopInfo {
    let mut loops = Vec::new();
    let mut loop_depths: BTreeMap<NodeId, usize> = BTreeMap::new();

    for tail in &cfg.control_nodes {
        for header in cfg.successors(*tail) {
            if header.raw() > tail.raw() {
                continue;
            }

            let mut body = BTreeSet::from([*header, *tail]);
            let mut stack = vec![*tail];
            while let Some(node) = stack.pop() {
                for predecessor in cfg.predecessors(node) {
                    if body.insert(*predecessor) {
                        stack.push(*predecessor);
                    }
                }
            }

            for node in &body {
                *loop_depths.entry(*node).or_default() += 1;
            }

            loops.push(RegionLoop {
                header: *header,
                backedge: *tail,
                body,
            });
        }
    }

    LoopInfo { loops, loop_depths }
}
