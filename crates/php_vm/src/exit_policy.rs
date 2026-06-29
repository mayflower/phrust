//! Request-local exit counter policy for adaptive specialization.

use std::collections::BTreeMap;

use crate::{GuardKind, GuardedTier, InlineCacheKind};

/// Location for one optimized or optimizable site.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExitSiteLocation {
    BytecodeOffset(u32),
    RegionId(String),
}

impl ExitSiteLocation {
    #[must_use]
    pub fn to_json(&self) -> String {
        match self {
            Self::BytecodeOffset(offset) => {
                format!("{{\"kind\":\"bytecode_offset\",\"value\":{offset}}}")
            }
            Self::RegionId(region) => {
                format!(
                    "{{\"kind\":\"region_id\",\"value\":\"{}\"}}",
                    escape_json(region)
                )
            }
        }
    }
}

/// Stable key for side-exit and guard-failure counters.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExitCounterKey {
    pub function_id: u32,
    pub location: ExitSiteLocation,
    pub tier: GuardedTier,
    pub exit_reason: String,
    pub guard_kind: Option<GuardKind>,
}

impl ExitCounterKey {
    #[must_use]
    pub fn bytecode(
        function_id: u32,
        bytecode_offset: u32,
        tier: GuardedTier,
        exit_reason: impl Into<String>,
        guard_kind: Option<GuardKind>,
    ) -> Self {
        Self {
            function_id,
            location: ExitSiteLocation::BytecodeOffset(bytecode_offset),
            tier,
            exit_reason: exit_reason.into(),
            guard_kind,
        }
    }

    #[must_use]
    pub fn region(
        function_id: u32,
        region_id: impl Into<String>,
        tier: GuardedTier,
        exit_reason: impl Into<String>,
        guard_kind: Option<GuardKind>,
    ) -> Self {
        Self {
            function_id,
            location: ExitSiteLocation::RegionId(region_id.into()),
            tier,
            exit_reason: exit_reason.into(),
            guard_kind,
        }
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let guard_kind = self.guard_kind.map(GuardKind::as_str).unwrap_or("");
        format!(
            concat!(
                "{{\"function_id\":{},",
                "\"location\":{},",
                "\"tier\":\"{}\",",
                "\"exit_reason\":\"{}\",",
                "\"guard_kind\":\"{}\"}}"
            ),
            self.function_id,
            self.location.to_json(),
            self.tier.as_str(),
            escape_json(&self.exit_reason),
            guard_kind
        )
    }
}

/// Thresholds used by the request-local exit policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExitPolicyThresholds {
    pub guard_failure_threshold: u64,
    pub side_exit_threshold: u64,
    pub megamorphic_threshold: u64,
    pub blacklist_threshold: u64,
    pub recompile_candidate_threshold: u64,
}

impl Default for ExitPolicyThresholds {
    fn default() -> Self {
        Self {
            guard_failure_threshold: 2,
            side_exit_threshold: 2,
            megamorphic_threshold: 1,
            blacklist_threshold: 3,
            recompile_candidate_threshold: 4,
        }
    }
}

/// Decision made for one exit-counter site.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitPolicyState {
    KeepOptimized,
    Dequicken,
    BlacklistForRequest,
    BlacklistPersistentlyCandidate,
    RecompileNarrowerCandidate,
    RecompileWiderCandidate,
    Unsupported,
}

impl ExitPolicyState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeepOptimized => "keep_optimized",
            Self::Dequicken => "dequicken",
            Self::BlacklistForRequest => "blacklist_for_request",
            Self::BlacklistPersistentlyCandidate => "blacklist_persistently_candidate",
            Self::RecompileNarrowerCandidate => "recompile_narrower_candidate",
            Self::RecompileWiderCandidate => "recompile_wider_candidate",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Aggregated counters for one key.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExitCounterSite {
    pub guard_failures: u64,
    pub side_exits: u64,
    pub megamorphic_transitions: u64,
    pub generic_fallbacks: u64,
    pub stable_hits: u64,
}

/// Policy decision attached to a site in JSON reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExitPolicyDecision {
    pub key: ExitCounterKey,
    pub state: ExitPolicyState,
    pub reason: String,
}

impl ExitPolicyDecision {
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            "{{\"key\":{},\"state\":\"{}\",\"reason\":\"{}\"}}",
            self.key.to_json(),
            self.state.as_str(),
            escape_json(&self.reason)
        )
    }
}

/// Unified request-local counter table for adaptive exits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExitCounterTable {
    thresholds: ExitPolicyThresholds,
    sites: BTreeMap<ExitCounterKey, ExitCounterSite>,
}

impl ExitCounterTable {
    #[must_use]
    pub fn new(thresholds: ExitPolicyThresholds) -> Self {
        Self {
            thresholds,
            sites: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn thresholds(&self) -> ExitPolicyThresholds {
        self.thresholds
    }

    #[must_use]
    pub fn sites(&self) -> &BTreeMap<ExitCounterKey, ExitCounterSite> {
        &self.sites
    }

    pub fn record_stable_hit(&mut self, key: ExitCounterKey) -> ExitPolicyState {
        let site = self.sites.entry(key.clone()).or_default();
        site.stable_hits = site.stable_hits.saturating_add(1);
        self.decision_for(&key)
    }

    pub fn record_guard_failure(&mut self, key: ExitCounterKey) -> ExitPolicyState {
        let site = self.sites.entry(key.clone()).or_default();
        site.guard_failures = site.guard_failures.saturating_add(1);
        site.generic_fallbacks = site.generic_fallbacks.saturating_add(1);
        self.decision_for(&key)
    }

    pub fn record_side_exit(&mut self, key: ExitCounterKey) -> ExitPolicyState {
        let site = self.sites.entry(key.clone()).or_default();
        site.side_exits = site.side_exits.saturating_add(1);
        site.generic_fallbacks = site.generic_fallbacks.saturating_add(1);
        self.decision_for(&key)
    }

    pub fn record_megamorphic(&mut self, key: ExitCounterKey) -> ExitPolicyState {
        let site = self.sites.entry(key.clone()).or_default();
        site.megamorphic_transitions = site.megamorphic_transitions.saturating_add(1);
        site.generic_fallbacks = site.generic_fallbacks.saturating_add(1);
        self.decision_for(&key)
    }

    #[must_use]
    pub fn decision_for(&self, key: &ExitCounterKey) -> ExitPolicyState {
        let Some(site) = self.sites.get(key) else {
            return ExitPolicyState::KeepOptimized;
        };
        if reason_is_unsupported(&key.exit_reason)
            || site.megamorphic_transitions >= self.thresholds.megamorphic_threshold
        {
            return ExitPolicyState::Unsupported;
        }
        if site.guard_failures >= self.thresholds.blacklist_threshold
            && should_request_blacklist(key)
        {
            return ExitPolicyState::BlacklistForRequest;
        }
        if site.guard_failures >= self.thresholds.blacklist_threshold.saturating_mul(2) {
            return ExitPolicyState::BlacklistPersistentlyCandidate;
        }
        if site.side_exits >= self.thresholds.side_exit_threshold && should_stay_generic(key) {
            return ExitPolicyState::Unsupported;
        }
        if site.guard_failures >= self.thresholds.recompile_candidate_threshold
            && key.guard_kind == Some(GuardKind::BuiltinCall)
        {
            return ExitPolicyState::RecompileNarrowerCandidate;
        }
        if site.side_exits >= self.thresholds.recompile_candidate_threshold {
            return ExitPolicyState::RecompileWiderCandidate;
        }
        if site.guard_failures >= self.thresholds.guard_failure_threshold {
            return ExitPolicyState::Dequicken;
        }
        ExitPolicyState::KeepOptimized
    }

    #[must_use]
    pub fn decisions(&self) -> Vec<ExitPolicyDecision> {
        self.sites
            .keys()
            .map(|key| ExitPolicyDecision {
                key: key.clone(),
                state: self.decision_for(key),
                reason: decision_reason(self, key).to_owned(),
            })
            .collect()
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\"sites\":[");
        for (index, (key, site)) in self.sites.iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                concat!(
                    "{{\"key\":{},",
                    "\"guard_failures\":{},",
                    "\"side_exits\":{},",
                    "\"megamorphic_transitions\":{},",
                    "\"generic_fallbacks\":{},",
                    "\"stable_hits\":{}}}"
                ),
                key.to_json(),
                site.guard_failures,
                site.side_exits,
                site.megamorphic_transitions,
                site.generic_fallbacks,
                site.stable_hits
            ));
        }
        json.push_str("],\"decisions\":[");
        for (index, decision) in self.decisions().iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            json.push_str(&decision.to_json());
        }
        json.push_str("]}");
        json
    }
}

impl Default for ExitCounterTable {
    fn default() -> Self {
        Self::new(ExitPolicyThresholds::default())
    }
}

pub(crate) fn inline_cache_guard_kind(kind: Option<InlineCacheKind>) -> GuardKind {
    match kind {
        Some(InlineCacheKind::PropertyFetch | InlineCacheKind::PropertyAssign) => {
            GuardKind::PropertyShape
        }
        Some(
            InlineCacheKind::FunctionCall
            | InlineCacheKind::MethodCall
            | InlineCacheKind::ClassConstantStaticProperty
            | InlineCacheKind::ClassRelation
            | InlineCacheKind::IncludePath
            | InlineCacheKind::AutoloadClassLookup
            | InlineCacheKind::DimFetch,
        )
        | None => GuardKind::InlineCacheShape,
    }
}

fn reason_is_unsupported(reason: &str) -> bool {
    reason.contains("unsupported")
        || reason.contains("mixed")
        || reason.contains("megamorphic")
        || reason.contains("generic")
}

fn should_request_blacklist(key: &ExitCounterKey) -> bool {
    matches!(
        key.guard_kind,
        Some(GuardKind::PropertyShape | GuardKind::InlineCacheShape)
    ) && (key.exit_reason.contains("wrong_class")
        || key.exit_reason.contains("shape")
        || key.exit_reason.contains("class"))
}

fn should_stay_generic(key: &ExitCounterKey) -> bool {
    matches!(
        key.guard_kind,
        Some(GuardKind::PackedArray | GuardKind::InlineCacheShape)
    ) || key.exit_reason.contains("layout")
        || key.exit_reason.contains("packed")
}

fn decision_reason(table: &ExitCounterTable, key: &ExitCounterKey) -> &'static str {
    let Some(site) = table.sites.get(key) else {
        return "no_exit_feedback";
    };
    if reason_is_unsupported(&key.exit_reason)
        || site.megamorphic_transitions >= table.thresholds.megamorphic_threshold
    {
        return "stay_generic";
    }
    if site.guard_failures >= table.thresholds.blacklist_threshold && should_request_blacklist(key)
    {
        return "request_local_blacklist";
    }
    if site.guard_failures >= table.thresholds.blacklist_threshold.saturating_mul(2) {
        return "persistent_blacklist_candidate";
    }
    if site.side_exits >= table.thresholds.side_exit_threshold && should_stay_generic(key) {
        return "generic_fallback";
    }
    if site.guard_failures >= table.thresholds.recompile_candidate_threshold
        && key.guard_kind == Some(GuardKind::BuiltinCall)
    {
        return "narrower_recompile_candidate";
    }
    if site.side_exits >= table.thresholds.recompile_candidate_threshold {
        return "wider_recompile_candidate";
    }
    if site.guard_failures >= table.thresholds.guard_failure_threshold {
        return "dequicken_after_guard_failures";
    }
    "stable_or_below_threshold"
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{
        ExitCounterKey, ExitCounterTable, ExitPolicyState, ExitPolicyThresholds, GuardKind,
        GuardedTier,
    };

    fn thresholds() -> ExitPolicyThresholds {
        ExitPolicyThresholds {
            guard_failure_threshold: 2,
            side_exit_threshold: 2,
            megamorphic_threshold: 1,
            blacklist_threshold: 3,
            recompile_candidate_threshold: 4,
        }
    }

    #[test]
    fn stable_optimized_site_keeps_policy() {
        let mut table = ExitCounterTable::new(thresholds());
        let key = ExitCounterKey::bytecode(
            7,
            11,
            GuardedTier::Quickening,
            "add_int_int",
            Some(GuardKind::QuickeningType),
        );

        assert_eq!(
            table.record_stable_hit(key.clone()),
            ExitPolicyState::KeepOptimized
        );
        assert_eq!(table.decision_for(&key), ExitPolicyState::KeepOptimized);
    }

    #[test]
    fn repeated_type_flip_dequickens() {
        let mut table = ExitCounterTable::new(thresholds());
        let key = ExitCounterKey::bytecode(
            1,
            2,
            GuardedTier::Quickening,
            "type_flip",
            Some(GuardKind::QuickeningType),
        );

        assert_eq!(
            table.record_guard_failure(key.clone()),
            ExitPolicyState::KeepOptimized
        );
        assert_eq!(
            table.record_guard_failure(key.clone()),
            ExitPolicyState::Dequicken
        );
    }

    #[test]
    fn repeated_wrong_class_blacklists_request_locally() {
        let mut table = ExitCounterTable::new(thresholds());
        let key = ExitCounterKey::bytecode(
            1,
            9,
            GuardedTier::InlineCache,
            "wrong_class_shape",
            Some(GuardKind::PropertyShape),
        );

        table.record_guard_failure(key.clone());
        table.record_guard_failure(key.clone());
        assert_eq!(
            table.record_guard_failure(key.clone()),
            ExitPolicyState::BlacklistForRequest
        );
    }

    #[test]
    fn packed_mixed_array_instability_stays_generic() {
        let mut table = ExitCounterTable::new(thresholds());
        let key = ExitCounterKey::bytecode(
            3,
            4,
            GuardedTier::Cranelift,
            "packed_to_mixed_layout",
            Some(GuardKind::PackedArray),
        );

        table.record_side_exit(key.clone());
        assert_eq!(
            table.record_side_exit(key.clone()),
            ExitPolicyState::Unsupported
        );
    }

    #[test]
    fn megamorphic_method_site_stays_generic() {
        let mut table = ExitCounterTable::new(thresholds());
        let key = ExitCounterKey::bytecode(
            5,
            8,
            GuardedTier::InlineCache,
            "method_megamorphic",
            Some(GuardKind::InlineCacheShape),
        );

        assert_eq!(
            table.record_megamorphic(key.clone()),
            ExitPolicyState::Unsupported
        );
    }

    #[test]
    fn report_contains_sites_and_decisions() {
        let mut table = ExitCounterTable::new(thresholds());
        let key = ExitCounterKey::region(
            5,
            "region-1",
            GuardedTier::RegionIr,
            "side_exit",
            Some(GuardKind::RegionAssumption),
        );
        table.record_side_exit(key);

        let json = table.to_json();
        assert!(json.contains("\"sites\""));
        assert!(json.contains("\"decisions\""));
        assert!(json.contains("\"region_id\""));
    }
}
