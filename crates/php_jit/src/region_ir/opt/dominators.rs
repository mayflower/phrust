//! Dominator computation for the conservative region CFG.

use std::collections::{BTreeMap, BTreeSet};

use crate::region_ir::NodeId;

use super::cfg::RegionCfg;

/// Dominator sets keyed by control node.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DominatorTree {
    dominators: BTreeMap<NodeId, BTreeSet<NodeId>>,
}

impl DominatorTree {
    /// Returns true when `dominator` dominates `node`.
    #[must_use]
    pub fn dominates(&self, dominator: NodeId, node: NodeId) -> bool {
        self.dominators
            .get(&node)
            .is_some_and(|nodes| nodes.contains(&dominator))
    }

    /// Returns dominators for one node.
    #[must_use]
    pub fn dominators(&self, node: NodeId) -> Option<&BTreeSet<NodeId>> {
        self.dominators.get(&node)
    }

    /// Finds a deterministic common dominator, preferring the deepest available
    /// node by dominator-set size and then table order.
    #[must_use]
    pub fn common_dominator(&self, nodes: &[NodeId]) -> Option<NodeId> {
        let first = *nodes.first()?;
        let mut common = self.dominators.get(&first)?.clone();
        for node in &nodes[1..] {
            let dominators = self.dominators.get(node)?;
            common.retain(|candidate| dominators.contains(candidate));
        }

        common.into_iter().max_by_key(|candidate| {
            (
                self.dominators.get(candidate).map_or(0, BTreeSet::len),
                candidate.raw(),
            )
        })
    }
}

/// Computes fixed-point dominator sets for the reconstructed CFG.
#[must_use]
pub fn compute_dominators(cfg: &RegionCfg) -> DominatorTree {
    let all: BTreeSet<NodeId> = cfg.control_nodes.iter().copied().collect();
    let mut dominators = BTreeMap::new();

    for node in &cfg.control_nodes {
        let set = if Some(*node) == cfg.entry {
            BTreeSet::from([*node])
        } else {
            all.clone()
        };
        dominators.insert(*node, set);
    }

    let mut changed = true;
    while changed {
        changed = false;
        for node in &cfg.control_nodes {
            if Some(*node) == cfg.entry {
                continue;
            }
            let predecessors = cfg.predecessors(*node);
            if predecessors.is_empty() {
                continue;
            }

            let mut next = all.clone();
            for predecessor in predecessors {
                if let Some(pred_dominators) = dominators.get(predecessor) {
                    next.retain(|candidate| pred_dominators.contains(candidate));
                }
            }
            next.insert(*node);

            if dominators.get(node) != Some(&next) {
                dominators.insert(*node, next);
                changed = true;
            }
        }
    }

    DominatorTree { dominators }
}
