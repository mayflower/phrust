//! Validation-only interpreter for the safe scalar region IR subset.

use std::collections::{BTreeMap, BTreeSet};

use super::opt::{RegionCfg, build_cfg};
use super::{
    NodeId, OptimizerRegionGraph, RegionCompareOp, RegionConst, RegionNodeKind, RegionValueType,
    SnapshotEntry, SnapshotId, VmSlotId, verify_region_graph,
};

/// Scalar values supported by the validation-only region interpreter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegionInterpretValue {
    /// Boolean scalar.
    Bool(bool),
    /// Signed 64-bit integer scalar.
    I64(i64),
}

impl RegionInterpretValue {
    fn value_type(&self) -> RegionValueType {
        match self {
            Self::Bool(_) => RegionValueType::Bool,
            Self::I64(_) => RegionValueType::I64,
        }
    }
}

/// Parameter values for one interpretation run.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegionInterpretInputs {
    params: BTreeMap<VmSlotId, RegionInterpretValue>,
}

impl RegionInterpretInputs {
    /// Creates an empty input set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an i64 parameter value.
    #[must_use]
    pub fn with_i64(mut self, slot: VmSlotId, value: i64) -> Self {
        self.params.insert(slot, RegionInterpretValue::I64(value));
        self
    }

    /// Adds a boolean parameter value.
    #[must_use]
    pub fn with_bool(mut self, slot: VmSlotId, value: bool) -> Self {
        self.params.insert(slot, RegionInterpretValue::Bool(value));
        self
    }

    fn get(&self, slot: VmSlotId) -> Option<&RegionInterpretValue> {
        self.params.get(&slot)
    }
}

/// Terminal status for one validation interpretation run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionInterpretStatus {
    /// A region `Return` node produced a scalar value.
    Returned,
    /// A guard failed and produced side-exit metadata.
    SideExit,
    /// The graph used a node or semantic case outside the safe subset.
    Unsupported,
}

/// Explicit unsupported reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionInterpretUnsupportedReason {
    /// Machine-readable code.
    pub code: &'static str,
    /// Human-readable detail.
    pub detail: String,
    /// Node where interpretation stopped, when node-local.
    pub node: Option<NodeId>,
}

impl RegionInterpretUnsupportedReason {
    fn new(code: &'static str, detail: impl Into<String>, node: Option<NodeId>) -> Self {
        Self {
            code,
            detail: detail.into(),
            node,
        }
    }
}

/// Metadata returned when a guard fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionGuardExitReason {
    /// Guard node that failed.
    pub guard_node: NodeId,
    /// Snapshot captured by the guard.
    pub snapshot: SnapshotId,
    /// Live snapshot entries available to a future resume path.
    pub entries: Vec<SnapshotEntry>,
    /// Control node that would resume in the interpreter path.
    pub resume_control: Option<NodeId>,
}

/// Structured result from the validation-only region interpreter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionInterpretResult {
    /// Terminal status.
    pub status: RegionInterpretStatus,
    /// Returned scalar value, when `status == Returned`.
    pub returned_value: Option<RegionInterpretValue>,
    /// Number of control and data nodes evaluated.
    pub executed_nodes: u64,
    /// Explicit unsupported reason, when `status == Unsupported`.
    pub unsupported_reason: Option<RegionInterpretUnsupportedReason>,
    /// Guard side-exit metadata, when `status == SideExit`.
    pub guard_exit_reason: Option<RegionGuardExitReason>,
}

impl RegionInterpretResult {
    fn returned(value: RegionInterpretValue, executed_nodes: u64) -> Self {
        Self {
            status: RegionInterpretStatus::Returned,
            returned_value: Some(value),
            executed_nodes,
            unsupported_reason: None,
            guard_exit_reason: None,
        }
    }

    fn side_exit(reason: RegionGuardExitReason, executed_nodes: u64) -> Self {
        Self {
            status: RegionInterpretStatus::SideExit,
            returned_value: None,
            executed_nodes,
            unsupported_reason: None,
            guard_exit_reason: Some(reason),
        }
    }

    fn unsupported(reason: RegionInterpretUnsupportedReason, executed_nodes: u64) -> Self {
        Self {
            status: RegionInterpretStatus::Unsupported,
            returned_value: None,
            executed_nodes,
            unsupported_reason: Some(reason),
            guard_exit_reason: None,
        }
    }
}

/// Interprets one region graph with scalar parameter inputs.
#[must_use]
pub fn interpret_region(
    graph: &OptimizerRegionGraph,
    inputs: &RegionInterpretInputs,
) -> RegionInterpretResult {
    RegionInterpreter::new(graph, inputs).run()
}

struct RegionInterpreter<'a> {
    graph: &'a OptimizerRegionGraph,
    inputs: &'a RegionInterpretInputs,
    cfg: RegionCfg,
    executed_nodes: u64,
    active_phi_input: usize,
}

impl<'a> RegionInterpreter<'a> {
    fn new(graph: &'a OptimizerRegionGraph, inputs: &'a RegionInterpretInputs) -> Self {
        Self {
            graph,
            inputs,
            cfg: build_cfg(graph),
            executed_nodes: 0,
            active_phi_input: 0,
        }
    }

    fn run(&mut self) -> RegionInterpretResult {
        if let Err(errors) = verify_region_graph(self.graph) {
            let detail = errors
                .iter()
                .map(|error| error.code)
                .collect::<Vec<_>>()
                .join(",");
            return self.unsupported("invalid_graph", detail, None);
        }

        let Some(mut current) = self.cfg.entry else {
            return self.unsupported("missing_entry", "region graph has no entry control", None);
        };
        let mut previous = None;
        let max_control_steps = self.graph.nodes().len().saturating_mul(16).max(64);

        for _ in 0..max_control_steps {
            self.active_phi_input = self.predecessor_index(current, previous);
            self.record_node(current);
            let Some(node) = self.graph.node(current) else {
                return self.unsupported(
                    "missing_control_node",
                    format!("control node n{} is missing", current.raw()),
                    Some(current),
                );
            };

            match &node.kind {
                RegionNodeKind::Start
                | RegionNodeKind::Entry(_)
                | RegionNodeKind::Begin
                | RegionNodeKind::Merge
                | RegionNodeKind::LoopBegin
                | RegionNodeKind::LoopEnd
                | RegionNodeKind::IfTrue
                | RegionNodeKind::IfFalse => {
                    let Some(next) = self.first_successor(current) else {
                        return self.unsupported(
                            "control_without_successor",
                            format!("control node n{} has no successor", current.raw()),
                            Some(current),
                        );
                    };
                    previous = Some(current);
                    current = next;
                }
                RegionNodeKind::If => {
                    let Ok(condition) = self.eval_bool_input(current, 0) else {
                        return self.unsupported(
                            "non_bool_if_condition",
                            format!("If node n{} condition is not bool", current.raw()),
                            Some(current),
                        );
                    };
                    let Some(next) = self.branch_successor(current, condition) else {
                        return self.unsupported(
                            "branch_without_successor",
                            format!("If node n{} has no matching successor", current.raw()),
                            Some(current),
                        );
                    };
                    previous = Some(current);
                    current = next;
                }
                RegionNodeKind::Guard { snapshot } => {
                    let Ok(condition) = self.eval_bool_input(current, 0) else {
                        return self.unsupported(
                            "non_bool_guard_condition",
                            format!("Guard node n{} condition is not bool", current.raw()),
                            Some(current),
                        );
                    };
                    if !condition {
                        let entries = self
                            .graph
                            .snapshot(*snapshot)
                            .map(|snapshot| snapshot.entries.clone())
                            .unwrap_or_default();
                        return RegionInterpretResult::side_exit(
                            RegionGuardExitReason {
                                guard_node: current,
                                snapshot: *snapshot,
                                entries,
                                resume_control: node.control,
                            },
                            self.executed_nodes,
                        );
                    }
                    let Some(next) = self.first_successor(current) else {
                        return self.unsupported(
                            "guard_without_successor",
                            format!("Guard node n{} has no successor", current.raw()),
                            Some(current),
                        );
                    };
                    previous = Some(current);
                    current = next;
                }
                RegionNodeKind::Return => match self.eval_input(current, 0) {
                    Ok(value) => {
                        return RegionInterpretResult::returned(value, self.executed_nodes);
                    }
                    Err(reason) => {
                        return RegionInterpretResult::unsupported(reason, self.executed_nodes);
                    }
                },
                _ => {
                    return self.unsupported(
                        "unsupported_control_node",
                        format!("{} is not executable control", node.kind.name()),
                        Some(current),
                    );
                }
            }
        }

        self.unsupported(
            "control_step_limit",
            format!("region exceeded {max_control_steps} control steps"),
            Some(current),
        )
    }

    fn eval_input(
        &mut self,
        node: NodeId,
        input_index: usize,
    ) -> Result<RegionInterpretValue, RegionInterpretUnsupportedReason> {
        let input = self
            .graph
            .node(node)
            .and_then(|node| node.inputs.get(input_index))
            .copied()
            .ok_or_else(|| {
                RegionInterpretUnsupportedReason::new(
                    "missing_input",
                    format!("node n{} missing input {}", node.raw(), input_index),
                    Some(node),
                )
            })?;
        self.eval_node(input, &mut BTreeSet::new())
    }

    fn eval_bool_input(
        &mut self,
        node: NodeId,
        input_index: usize,
    ) -> Result<bool, RegionInterpretUnsupportedReason> {
        match self.eval_input(node, input_index)? {
            RegionInterpretValue::Bool(value) => Ok(value),
            other => Err(RegionInterpretUnsupportedReason::new(
                "type_mismatch",
                format!("expected bool, found {}", value_label(&other)),
                Some(node),
            )),
        }
    }

    fn eval_node(
        &mut self,
        id: NodeId,
        stack: &mut BTreeSet<NodeId>,
    ) -> Result<RegionInterpretValue, RegionInterpretUnsupportedReason> {
        if !stack.insert(id) {
            return Err(RegionInterpretUnsupportedReason::new(
                "cyclic_data_dependency",
                format!(
                    "node n{} participates in a cyclic data dependency",
                    id.raw()
                ),
                Some(id),
            ));
        }
        self.record_node(id);

        let Some(node) = self.graph.node(id) else {
            stack.remove(&id);
            return Err(RegionInterpretUnsupportedReason::new(
                "missing_node",
                format!("node n{} is missing", id.raw()),
                Some(id),
            ));
        };

        let result = match &node.kind {
            RegionNodeKind::Const(constant) => match self.graph.constant(*constant) {
                Some(RegionConst::Bool(value)) => Ok(RegionInterpretValue::Bool(*value)),
                Some(RegionConst::I64(value)) => Ok(RegionInterpretValue::I64(*value)),
                Some(constant) => Err(RegionInterpretUnsupportedReason::new(
                    "unsupported_constant",
                    format!("{constant:?} is outside the scalar subset"),
                    Some(id),
                )),
                None => Err(RegionInterpretUnsupportedReason::new(
                    "missing_constant",
                    format!("constant c{} is missing", constant.raw()),
                    Some(id),
                )),
            },
            RegionNodeKind::Param { slot } => {
                let value = self.inputs.get(*slot).cloned().ok_or_else(|| {
                    RegionInterpretUnsupportedReason::new(
                        "missing_param",
                        format!("parameter slot v{} is missing", slot.raw()),
                        Some(id),
                    )
                })?;
                if value.value_type() == node.value_type {
                    Ok(value)
                } else {
                    Err(RegionInterpretUnsupportedReason::new(
                        "param_type_mismatch",
                        format!(
                            "parameter slot v{} expected {}, found {}",
                            slot.raw(),
                            node.value_type.as_str(),
                            value_label(&value)
                        ),
                        Some(id),
                    ))
                }
            }
            RegionNodeKind::Copy => self.eval_data_input(id, 0, stack),
            RegionNodeKind::Phi => {
                let input_index = self
                    .active_phi_input
                    .min(node.inputs.len().saturating_sub(1));
                self.eval_data_input(id, input_index, stack)
            }
            RegionNodeKind::Add | RegionNodeKind::Sub | RegionNodeKind::Mul => {
                let left = self.eval_i64_data_input(id, 0, stack)?;
                let right = self.eval_i64_data_input(id, 1, stack)?;
                let value = match node.kind {
                    RegionNodeKind::Add => left.checked_add(right),
                    RegionNodeKind::Sub => left.checked_sub(right),
                    RegionNodeKind::Mul => left.checked_mul(right),
                    _ => None,
                };
                value.map(RegionInterpretValue::I64).ok_or_else(|| {
                    RegionInterpretUnsupportedReason::new(
                        "i64_overflow",
                        format!("{} overflowed checked i64 arithmetic", node.kind.name()),
                        Some(id),
                    )
                })
            }
            RegionNodeKind::AndBool | RegionNodeKind::OrBool => {
                let left = self.eval_bool_data_input(id, 0, stack)?;
                let right = self.eval_bool_data_input(id, 1, stack)?;
                Ok(RegionInterpretValue::Bool(match node.kind {
                    RegionNodeKind::AndBool => left && right,
                    RegionNodeKind::OrBool => left || right,
                    _ => unreachable!(),
                }))
            }
            RegionNodeKind::Compare(op) => {
                let left = self.eval_i64_data_input(id, 0, stack)?;
                let right = self.eval_i64_data_input(id, 1, stack)?;
                Ok(RegionInterpretValue::Bool(compare_i64(*op, left, right)))
            }
            RegionNodeKind::Select => {
                let condition = self.eval_bool_data_input(id, 0, stack)?;
                let chosen = if condition { 1 } else { 2 };
                self.eval_data_input(id, chosen, stack)
            }
            _ if !node.effects.is_pure() => Err(RegionInterpretUnsupportedReason::new(
                "effectful_node",
                format!("{} has PHP-visible effects", node.kind.name()),
                Some(id),
            )),
            _ => Err(RegionInterpretUnsupportedReason::new(
                "unsupported_node",
                format!("{} is outside the interpreter subset", node.kind.name()),
                Some(id),
            )),
        };

        stack.remove(&id);
        result
    }

    fn eval_data_input(
        &mut self,
        node: NodeId,
        input_index: usize,
        stack: &mut BTreeSet<NodeId>,
    ) -> Result<RegionInterpretValue, RegionInterpretUnsupportedReason> {
        let input = self
            .graph
            .node(node)
            .and_then(|node| node.inputs.get(input_index))
            .copied()
            .ok_or_else(|| {
                RegionInterpretUnsupportedReason::new(
                    "missing_input",
                    format!("node n{} missing input {}", node.raw(), input_index),
                    Some(node),
                )
            })?;
        self.eval_node(input, stack)
    }

    fn eval_i64_data_input(
        &mut self,
        node: NodeId,
        input_index: usize,
        stack: &mut BTreeSet<NodeId>,
    ) -> Result<i64, RegionInterpretUnsupportedReason> {
        match self.eval_data_input(node, input_index, stack)? {
            RegionInterpretValue::I64(value) => Ok(value),
            other => Err(RegionInterpretUnsupportedReason::new(
                "type_mismatch",
                format!("expected i64, found {}", value_label(&other)),
                Some(node),
            )),
        }
    }

    fn eval_bool_data_input(
        &mut self,
        node: NodeId,
        input_index: usize,
        stack: &mut BTreeSet<NodeId>,
    ) -> Result<bool, RegionInterpretUnsupportedReason> {
        match self.eval_data_input(node, input_index, stack)? {
            RegionInterpretValue::Bool(value) => Ok(value),
            other => Err(RegionInterpretUnsupportedReason::new(
                "type_mismatch",
                format!("expected bool, found {}", value_label(&other)),
                Some(node),
            )),
        }
    }

    fn first_successor(&self, node: NodeId) -> Option<NodeId> {
        self.cfg.successors(node).first().copied()
    }

    fn branch_successor(&self, node: NodeId, condition: bool) -> Option<NodeId> {
        let successors = self.cfg.successors(node);
        if successors.len() == 1 {
            return successors.first().copied();
        }

        let wanted = if condition {
            RegionNodeKind::IfTrue
        } else {
            RegionNodeKind::IfFalse
        };
        successors
            .iter()
            .copied()
            .find(|successor| {
                self.graph
                    .node(*successor)
                    .is_some_and(|node| node.kind == wanted)
            })
            .or_else(|| successors.get(usize::from(!condition)).copied())
    }

    fn predecessor_index(&self, current: NodeId, previous: Option<NodeId>) -> usize {
        let Some(previous) = previous else {
            return 0;
        };
        self.cfg
            .predecessors(current)
            .iter()
            .position(|candidate| *candidate == previous)
            .unwrap_or(0)
    }

    fn record_node(&mut self, _node: NodeId) {
        self.executed_nodes += 1;
    }

    fn unsupported(
        &self,
        code: &'static str,
        detail: impl Into<String>,
        node: Option<NodeId>,
    ) -> RegionInterpretResult {
        RegionInterpretResult::unsupported(
            RegionInterpretUnsupportedReason::new(code, detail, node),
            self.executed_nodes,
        )
    }
}

fn compare_i64(op: RegionCompareOp, left: i64, right: i64) -> bool {
    match op {
        RegionCompareOp::Eq => left == right,
        RegionCompareOp::NotEq => left != right,
        RegionCompareOp::Lt => left < right,
        RegionCompareOp::Lte => left <= right,
        RegionCompareOp::Gt => left > right,
        RegionCompareOp::Gte => left >= right,
    }
}

fn value_label(value: &RegionInterpretValue) -> &'static str {
    match value {
        RegionInterpretValue::Bool(_) => "bool",
        RegionInterpretValue::I64(_) => "i64",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RegionInterpretInputs, RegionInterpretStatus, RegionInterpretValue, interpret_region,
    };
    use crate::region_ir::opt::analyze_region_graph;
    use crate::region_ir::{
        NodeId, OptimizerRegionGraph, RegionBuilder, RegionCompareOp, RegionConst, RegionEffects,
        RegionId, RegionNode, RegionNodeKind, RegionPlacement, RegionValueType, SnapshotEntry,
        VmSlotId,
    };

    #[test]
    fn region_ir_interpreter_returns_expected_scalar_result() {
        let mut builder = RegionBuilder::new(RegionId::new(420), "interpret-scalar");
        let start = builder.start();
        let param = builder.param_i64(VmSlotId::new(0));
        let two = builder.const_i64(2);
        let three = builder.const_i64(3);
        let sum = builder.emit_add_i64(param, two);
        let product = builder.emit_mul_i64(sum, three);
        builder.emit_return(start, product);
        let graph = builder.finish();

        let result = interpret_region(
            &graph,
            &RegionInterpretInputs::new().with_i64(VmSlotId::new(0), 5),
        );

        assert_eq!(result.status, RegionInterpretStatus::Returned);
        assert_eq!(result.returned_value, Some(RegionInterpretValue::I64(21)));
        assert!(result.executed_nodes >= 6);
    }

    #[test]
    fn folding_before_interpretation_does_not_change_result() {
        let mut raw = RegionBuilder::new(RegionId::new(421), "raw-add-zero");
        let raw_start = raw.start();
        let raw_param = raw.param_i64(VmSlotId::new(0));
        let raw_zero = raw.const_i64(0);
        let raw_added = raw.emit_add_i64(raw_param, raw_zero);
        raw.emit_return(raw_start, raw_added);
        let raw_graph = raw.finish();

        let mut folded = RegionBuilder::new(RegionId::new(422), "folded-add-zero");
        let folded_start = folded.start();
        let folded_param = folded.param_i64(VmSlotId::new(0));
        let folded_zero = folded.const_i64(0);
        let folded_added = folded.fold_add_i64(folded_param, folded_zero);
        folded.emit_return(folded_start, folded_added);
        let folded_graph = folded.finish();

        let inputs = RegionInterpretInputs::new().with_i64(VmSlotId::new(0), 41);
        assert_eq!(
            interpret_region(&raw_graph, &inputs).returned_value,
            interpret_region(&folded_graph, &inputs).returned_value
        );
    }

    #[test]
    fn sccp_and_gcm_analysis_before_interpretation_does_not_change_result() {
        let mut builder = RegionBuilder::new(RegionId::new(423), "analysis-before-interpret");
        let start = builder.start();
        let lhs = builder.const_i64(4);
        let rhs = builder.const_i64(2);
        let sum = builder.emit_add_i64(lhs, rhs);
        let limit = builder.const_i64(10);
        let condition = builder.emit_compare_i64(RegionCompareOp::Lt, sum, limit);
        let branch = builder.emit_if(start, condition);
        builder.emit_return(branch, sum);
        let graph = builder.finish();

        let before = interpret_region(&graph, &RegionInterpretInputs::new());
        let analysis = analyze_region_graph(&graph);
        let after = interpret_region(&graph, &RegionInterpretInputs::new());

        assert_eq!(before.status, RegionInterpretStatus::Returned);
        assert_eq!(before.returned_value, Some(RegionInterpretValue::I64(6)));
        assert_eq!(before.returned_value, after.returned_value);
        assert!(
            analysis
                .sccp
                .values
                .iter()
                .any(|value| { value.label() == "const i64 6" })
        );
        assert!(analysis.gcm.counters.nodes_considered > 0);
    }

    #[test]
    fn guard_fail_returns_snapshot_exit_metadata() {
        let mut builder = RegionBuilder::new(RegionId::new(424), "guard-fail");
        let start = builder.start();
        let param = builder.param_i64(VmSlotId::new(0));
        let zero = builder.const_i64(0);
        let positive = builder.emit_compare_i64(RegionCompareOp::Gt, param, zero);
        let snapshot = builder.add_snapshot(vec![SnapshotEntry {
            slot: VmSlotId::new(0),
            value_type: RegionValueType::I64,
        }]);
        let guard = builder.emit_guard(snapshot, start, positive);
        builder.emit_return(guard, param);
        let graph = builder.finish();

        let result = interpret_region(
            &graph,
            &RegionInterpretInputs::new().with_i64(VmSlotId::new(0), -1),
        );

        assert_eq!(result.status, RegionInterpretStatus::SideExit);
        let exit = result.guard_exit_reason.expect("guard exit metadata");
        assert_eq!(exit.snapshot, snapshot);
        assert_eq!(exit.resume_control, Some(start));
        assert_eq!(exit.entries.len(), 1);
    }

    #[test]
    fn unsupported_nodes_fail_explicitly() {
        let mut graph = OptimizerRegionGraph::new(RegionId::new(425), "unsupported-call");
        let start = control_node(&mut graph, RegionNodeKind::Start, None);
        let call = graph.add_node(RegionNode::new(
            RegionNodeKind::Call,
            Vec::new(),
            Some(start),
            RegionValueType::MixedValue,
            RegionPlacement::Pinned,
            RegionEffects {
                may_call: true,
                ..RegionEffects::PURE
            },
        ));
        graph.add_node(RegionNode::new(
            RegionNodeKind::Return,
            vec![call],
            Some(start),
            RegionValueType::Control,
            RegionPlacement::ControlOnly,
            RegionEffects::PURE,
        ));

        let result = interpret_region(&graph, &RegionInterpretInputs::new());

        assert_eq!(result.status, RegionInterpretStatus::Unsupported);
        assert_eq!(
            result.unsupported_reason.as_ref().map(|reason| reason.code),
            Some("effectful_node")
        );
    }

    #[test]
    fn select_and_phi_are_supported_for_simple_scalar_tests() {
        let mut graph = OptimizerRegionGraph::new(RegionId::new(426), "select-phi");
        let start = control_node(&mut graph, RegionNodeKind::Start, None);
        let c_true = graph.add_constant(RegionConst::Bool(true));
        let condition = graph.add_node(RegionNode::new(
            RegionNodeKind::Const(c_true),
            Vec::new(),
            None,
            RegionValueType::Bool,
            RegionPlacement::Floating,
            RegionEffects::PURE,
        ));
        let one = const_i64(&mut graph, 1);
        let two = const_i64(&mut graph, 2);
        let selected = graph.add_node(RegionNode::new(
            RegionNodeKind::Select,
            vec![condition, one, two],
            None,
            RegionValueType::I64,
            RegionPlacement::Floating,
            RegionEffects::PURE,
        ));
        let phi = graph.add_node(RegionNode::new(
            RegionNodeKind::Phi,
            vec![selected, selected],
            None,
            RegionValueType::I64,
            RegionPlacement::Floating,
            RegionEffects::PURE,
        ));
        graph.add_node(RegionNode::new(
            RegionNodeKind::Return,
            vec![phi],
            Some(start),
            RegionValueType::Control,
            RegionPlacement::ControlOnly,
            RegionEffects::PURE,
        ));

        let result = interpret_region(&graph, &RegionInterpretInputs::new());

        assert_eq!(result.status, RegionInterpretStatus::Returned);
        assert_eq!(result.returned_value, Some(RegionInterpretValue::I64(1)));
    }

    fn control_node(
        graph: &mut OptimizerRegionGraph,
        kind: RegionNodeKind,
        control: Option<NodeId>,
    ) -> NodeId {
        graph.add_node(RegionNode::new(
            kind,
            Vec::new(),
            control,
            RegionValueType::Control,
            RegionPlacement::ControlOnly,
            RegionEffects::PURE,
        ))
    }

    fn const_i64(graph: &mut OptimizerRegionGraph, value: i64) -> NodeId {
        let constant = graph.add_constant(RegionConst::I64(value));
        graph.add_node(RegionNode::new(
            RegionNodeKind::Const(constant),
            Vec::new(),
            None,
            RegionValueType::I64,
            RegionPlacement::Floating,
            RegionEffects::PURE,
        ))
    }
}
