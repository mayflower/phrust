use php_vm::api::{FunctionCallSiteSnapshot, QuickeningSiteSnapshot};
use std::{collections::BTreeMap, sync::Mutex};

// NOTE: feedback templates, not an OPcache-style metadata layer.
//
// This store only carries quickening and callsite feedback templates between requests.
// It does not persist immutable class/function/property metadata, include
// graphs, or any PHP-visible state; describing it as OPcache-like would be
// dishonest. PHP-visible request state is rebuilt per request; engine-owned
// handles and caches live separately in the worker executor. Rejected
// PHP-visible persistence is reported through
// `phrust_server_persistent_engine_rejected_persistence_total`.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PersistentMetadataStats {
    pub(crate) feedback_templates: u64,
}

#[derive(Debug, Default)]
pub(crate) struct PersistentMetadataStore {
    quickening_templates: Mutex<BTreeMap<String, Vec<QuickeningSiteSnapshot>>>,
    callsite_templates: Mutex<BTreeMap<String, Vec<FunctionCallSiteSnapshot>>>,
}

impl PersistentMetadataStore {
    pub(crate) fn quickening_templates(&self, script: &str) -> Vec<QuickeningSiteSnapshot> {
        self.quickening_templates
            .lock()
            .ok()
            .and_then(|templates| templates.get(script).cloned())
            .unwrap_or_default()
    }

    pub(crate) fn callsite_templates(&self, script: &str) -> Vec<FunctionCallSiteSnapshot> {
        self.callsite_templates
            .lock()
            .ok()
            .and_then(|templates| templates.get(script).cloned())
            .unwrap_or_default()
    }

    pub(crate) fn absorb_quickening_feedback(
        &self,
        script: &str,
        feedback: Vec<QuickeningSiteSnapshot>,
    ) -> usize {
        if feedback.is_empty() {
            return 0;
        }
        let Ok(mut templates) = self.quickening_templates.lock() else {
            return 0;
        };
        let accepted = feedback.len();
        let current = templates.entry(script.to_owned()).or_default();
        let merged = current
            .iter()
            .chain(feedback.iter())
            .map(|snapshot| (snapshot.site, *snapshot))
            .collect::<BTreeMap<_, _>>();
        *current = merged.values().copied().collect();
        accepted
    }

    pub(crate) fn absorb_callsite_feedback(
        &self,
        script: &str,
        feedback: Vec<FunctionCallSiteSnapshot>,
    ) -> usize {
        if feedback.is_empty() {
            return 0;
        }
        let Ok(mut templates) = self.callsite_templates.lock() else {
            return 0;
        };
        let accepted = feedback.len();
        let current = templates.entry(script.to_owned()).or_default();
        let merged = current
            .iter()
            .chain(feedback.iter())
            .map(|snapshot| {
                (
                    (snapshot.function, snapshot.block, snapshot.instruction),
                    snapshot.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        *current = merged.into_values().collect();
        accepted
    }

    pub(crate) fn stats(&self) -> PersistentMetadataStats {
        let feedback_templates = self
            .quickening_templates
            .lock()
            .map(|templates| templates.values().map(Vec::len).sum::<usize>() as u64)
            .unwrap_or_default();
        let callsite_templates = self
            .callsite_templates
            .lock()
            .map(|templates| templates.values().map(Vec::len).sum::<usize>() as u64)
            .unwrap_or_default();
        let feedback_templates = feedback_templates.saturating_add(callsite_templates);
        PersistentMetadataStats { feedback_templates }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use php_vm::experimental::{QuickeningSiteKey, QuickeningSpecialization, QuickeningState};

    #[test]
    fn quickening_feedback_templates_are_deduplicated_by_site() {
        let store = PersistentMetadataStore::default();
        let first = QuickeningSiteSnapshot {
            site: QuickeningSiteKey::Dense {
                unit: 1,
                function: 2,
                instruction: 3,
            },
            state: QuickeningState::Specialized,
            specialization: Some(QuickeningSpecialization::AddIntInt),
            guard_failures: 0,
        };
        let replacement = QuickeningSiteSnapshot {
            guard_failures: 4,
            ..first
        };
        let second = QuickeningSiteSnapshot {
            site: QuickeningSiteKey::Ir {
                function: 5,
                block: 6,
                instruction: 7,
            },
            state: QuickeningState::Blacklisted,
            specialization: None,
            guard_failures: 2,
        };

        assert_eq!(
            store.absorb_quickening_feedback("index.php", vec![first, second]),
            2
        );
        assert_eq!(
            store.absorb_quickening_feedback("index.php", vec![replacement]),
            1
        );

        let templates = store.quickening_templates("index.php");
        assert_eq!(templates.len(), 2);
        assert!(templates.contains(&replacement));
        assert!(templates.contains(&second));
        assert!(store.quickening_templates("other.php").is_empty());
        assert_eq!(store.stats().feedback_templates, 2);
    }

    #[test]
    fn callsite_feedback_is_deduplicated_and_scoped_by_script() {
        let store = PersistentMetadataStore::default();
        let first = FunctionCallSiteSnapshot {
            function: 1,
            block: 2,
            instruction: 3,
            lowered_name: "strlen".to_owned(),
            arity: 1,
            epoch: 4,
            target_function: 5,
        };
        let replacement = FunctionCallSiteSnapshot {
            epoch: 9,
            ..first.clone()
        };

        assert_eq!(store.absorb_callsite_feedback("index.php", vec![first]), 1);
        assert_eq!(
            store.absorb_callsite_feedback("index.php", vec![replacement.clone()]),
            1
        );
        assert_eq!(store.callsite_templates("index.php"), vec![replacement]);
        assert!(store.callsite_templates("admin.php").is_empty());
        assert_eq!(store.stats().feedback_templates, 1);
    }
}
