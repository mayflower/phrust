//! Bounded worker-owned cache above Region IR construction.
//!
//! The lower Cranelift code manager deduplicates machine-code emission, but its
//! key is reached only after authoritative Region IR has been rebuilt. Server
//! workers retain these immutable compile records so warm requests can reuse
//! the published handles without repeating that frontend work. Requested
//! compile keys and graph-derived function aliases use separate bounded LRU
//! segments: alias churn must never evict every next-request entry key.

use php_ir::FunctionId;
use php_jit::JitUnitCompileRecord;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

const DEFAULT_NATIVE_COMPILE_CACHE_ENTRIES: usize = 4_096;
const DEFAULT_LOADED_NATIVE_UNIT_ENTRIES: usize = 4_096;

/// Immutable cached artifact publication shared by every request in a process.
#[derive(Debug)]
pub(super) struct LoadedNativeUnit {
    _artifact: Arc<php_jit::NativeLoadedArtifact>,
    native_entries: Arc<BTreeMap<FunctionId, php_jit::JitFunctionHandle>>,
}

impl LoadedNativeUnit {
    fn new(artifact: php_jit::NativeLoadedArtifact) -> Result<Self, php_jit::NativeCacheError> {
        let artifact = Arc::new(artifact);
        let native_entries = artifact
            .image()
            .functions
            .iter()
            .map(|function| {
                let function_id = FunctionId::new(function.function_id);
                php_jit::JitFunctionHandle::from_cached_artifact(
                    Arc::clone(&artifact),
                    function_id,
                    artifact.region_metadata(function.function_id).cloned(),
                )
                .map(|handle| (function_id, handle))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self {
            _artifact: artifact,
            native_entries: Arc::new(native_entries),
        })
    }

    pub(super) fn native_entries(&self) -> &Arc<BTreeMap<FunctionId, php_jit::JitFunctionHandle>> {
        &self.native_entries
    }
}

#[derive(Debug, Default)]
pub(super) struct LoadedNativeUnitRegistry {
    units: RwLock<BTreeMap<String, Arc<LoadedNativeUnit>>>,
    hits: AtomicU64,
    maps: AtomicU64,
    entry_table_constructions: AtomicU64,
    mapped_executable_bytes: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct LoadedNativeUnitRegistryStats {
    pub(super) hits: u64,
    pub(super) maps: u64,
    pub(super) entry_table_constructions: u64,
    pub(super) mapped_executable_bytes: u64,
}

impl LoadedNativeUnitRegistry {
    /// Loads, validates, maps, and publishes one artifact identity at most once.
    pub(super) fn get_or_load(
        &self,
        identity: &php_jit::NativeCacheIdentity,
        load: impl FnOnce() -> Result<Option<php_jit::NativeLoadedArtifact>, php_jit::NativeCacheError>,
    ) -> Result<Option<Arc<LoadedNativeUnit>>, php_jit::NativeCacheError> {
        let key = identity.cache_key();
        if let Some(unit) = read_unpoisoned(&self.units).get(&key).cloned() {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(Some(unit));
        }
        // Serialize only the cold publication path. Recheck after acquiring
        // the writer so concurrent cold requests cannot map the same identity
        // twice; every warm request takes only a shared read lock.
        let mut units = write_unpoisoned(&self.units);
        if let Some(unit) = units.get(&key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(Some(Arc::clone(unit)));
        }
        let Some(artifact) = load()? else {
            return Ok(None);
        };
        let unit = Arc::new(LoadedNativeUnit::new(artifact)?);
        self.maps.fetch_add(1, Ordering::Relaxed);
        self.entry_table_constructions
            .fetch_add(1, Ordering::Relaxed);
        self.mapped_executable_bytes.fetch_add(
            unit._artifact.mapped_executable_bytes() as u64,
            Ordering::Relaxed,
        );
        while units.len() >= DEFAULT_LOADED_NATIVE_UNIT_ENTRIES {
            let Some(retired) = units
                .iter()
                .find_map(|(key, unit)| (Arc::strong_count(unit) == 1).then(|| key.clone()))
            else {
                break;
            };
            units.remove(&retired);
        }
        units.insert(key, Arc::clone(&unit));
        Ok(Some(unit))
    }

    pub(super) fn stats(&self) -> LoadedNativeUnitRegistryStats {
        LoadedNativeUnitRegistryStats {
            hits: self.hits.load(Ordering::Relaxed),
            maps: self.maps.load(Ordering::Relaxed),
            entry_table_constructions: self.entry_table_constructions.load(Ordering::Relaxed),
            mapped_executable_bytes: self.mapped_executable_bytes.load(Ordering::Relaxed),
        }
    }
}

fn read_unpoisoned<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_unpoisoned<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct NativeCompileCacheKey {
    unit_cache_identity: u64,
    function: u32,
    optimizing: bool,
    external_signatures_hash: u64,
}

impl NativeCompileCacheKey {
    pub(super) const fn new(
        unit_cache_identity: u64,
        function: FunctionId,
        optimizing: bool,
        external_signatures_hash: u64,
    ) -> Self {
        Self {
            unit_cache_identity,
            function: function.raw(),
            optimizing,
            external_signatures_hash,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeCompileCacheDisposition {
    Hit,
    Miss,
    Wait,
}

impl NativeCompileCacheDisposition {
    pub(super) const fn compiled(self) -> bool {
        matches!(self, Self::Miss)
    }
}

/// Snapshot of the process-worker cache above Region IR lowering.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeCompileCacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub insertions: u64,
    pub evictions: u64,
    pub compile_waits: u64,
    pub compile_failures: u64,
    pub compile_time_nanos: u64,
}

impl NativeCompileCacheStats {
    pub(super) fn saturating_delta(self, before: Self) -> Self {
        Self {
            entries: self.entries,
            hits: self.hits.saturating_sub(before.hits),
            misses: self.misses.saturating_sub(before.misses),
            insertions: self.insertions.saturating_sub(before.insertions),
            evictions: self.evictions.saturating_sub(before.evictions),
            compile_waits: self.compile_waits.saturating_sub(before.compile_waits),
            compile_failures: self
                .compile_failures
                .saturating_sub(before.compile_failures),
            compile_time_nanos: self
                .compile_time_nanos
                .saturating_sub(before.compile_time_nanos),
        }
    }
}

#[derive(Debug, Default)]
struct NativeCompileCacheMetrics {
    hits: u64,
    misses: u64,
    insertions: u64,
    evictions: u64,
    compile_waits: u64,
    compile_failures: u64,
    compile_time_nanos: u64,
}

#[derive(Debug)]
struct CompileWait {
    result: Mutex<Option<Result<Arc<[JitUnitCompileRecord]>, String>>>,
    ready: Condvar,
}

impl CompileWait {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    fn wait(&self) -> Result<Arc<[JitUnitCompileRecord]>, String> {
        let mut result = lock_unpoisoned(&self.result);
        while result.is_none() {
            result = self
                .ready
                .wait(result)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        match result.as_ref() {
            Some(result) => result.clone(),
            None => Err("native compile coordination ended without a result".to_owned()),
        }
    }

    fn publish(&self, result: Result<Arc<[JitUnitCompileRecord]>, String>) {
        *lock_unpoisoned(&self.result) = Some(result);
        self.ready.notify_all();
    }
}

#[derive(Debug, Default)]
struct NativeCompileCacheState {
    primary_entries: HashMap<NativeCompileCacheKey, Arc<[JitUnitCompileRecord]>>,
    primary_lru: VecDeque<NativeCompileCacheKey>,
    alias_entries: HashMap<NativeCompileCacheKey, Arc<[JitUnitCompileRecord]>>,
    alias_lru: VecDeque<NativeCompileCacheKey>,
    in_flight: HashMap<NativeCompileCacheKey, Arc<CompileWait>>,
    metrics: NativeCompileCacheMetrics,
}

#[derive(Debug)]
pub(super) struct NativeCompileCache {
    capacity: usize,
    state: Mutex<NativeCompileCacheState>,
}

impl Default for NativeCompileCache {
    fn default() -> Self {
        Self::new(DEFAULT_NATIVE_COMPILE_CACHE_ENTRIES)
    }
}

impl NativeCompileCache {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Mutex::new(NativeCompileCacheState::default()),
        }
    }

    pub(super) fn get_or_compile(
        &self,
        key: NativeCompileCacheKey,
        compile: impl FnOnce() -> Result<Vec<JitUnitCompileRecord>, String>,
    ) -> Result<(Arc<[JitUnitCompileRecord]>, NativeCompileCacheDisposition), String> {
        let wait = {
            let mut state = lock_unpoisoned(&self.state);
            if let Some(records) = state.primary_entries.get(&key).cloned() {
                state.metrics.hits = state.metrics.hits.saturating_add(1);
                touch_lru(&mut state.primary_lru, key);
                return Ok((records, NativeCompileCacheDisposition::Hit));
            }
            if let Some(records) = state.alias_entries.remove(&key) {
                state.metrics.hits = state.metrics.hits.saturating_add(1);
                remove_lru(&mut state.alias_lru, key);
                insert_primary(&mut state, key, Arc::clone(&records));
                evict_primary_entries(&mut state, self.capacity);
                return Ok((records, NativeCompileCacheDisposition::Hit));
            }
            if let Some(wait) = state.in_flight.get(&key).cloned() {
                state.metrics.hits = state.metrics.hits.saturating_add(1);
                state.metrics.compile_waits = state.metrics.compile_waits.saturating_add(1);
                Some(wait)
            } else {
                state.metrics.misses = state.metrics.misses.saturating_add(1);
                let wait = Arc::new(CompileWait::new());
                state.in_flight.insert(key, Arc::clone(&wait));
                None
            }
        };

        if let Some(wait) = wait {
            return wait
                .wait()
                .map(|records| (records, NativeCompileCacheDisposition::Wait));
        }

        let compile_started = Instant::now();
        let result = compile().map(Arc::<[JitUnitCompileRecord]>::from);
        let compile_time_nanos = compile_started
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        let wait = {
            let mut state = lock_unpoisoned(&self.state);
            let wait = state.in_flight.remove(&key);
            state.metrics.compile_time_nanos = state
                .metrics
                .compile_time_nanos
                .saturating_add(compile_time_nanos);
            if let Ok(records) = &result {
                let aliases = records
                    .iter()
                    .map(|record| NativeCompileCacheKey {
                        unit_cache_identity: key.unit_cache_identity,
                        function: record.function.raw(),
                        optimizing: key.optimizing,
                        external_signatures_hash: key.external_signatures_hash,
                    })
                    .collect::<Vec<_>>();
                insert_primary(&mut state, key, Arc::clone(records));
                for alias in aliases {
                    if alias != key
                        && !state.in_flight.contains_key(&alias)
                        && !state.primary_entries.contains_key(&alias)
                    {
                        insert_alias(&mut state, alias, Arc::clone(records));
                    }
                }
                state.metrics.insertions = state.metrics.insertions.saturating_add(1);
                evict_primary_entries(&mut state, self.capacity);
                evict_alias_entries(&mut state, self.capacity);
            } else {
                state.metrics.compile_failures = state.metrics.compile_failures.saturating_add(1);
            }
            wait
        };
        if let Some(wait) = wait {
            wait.publish(result.clone());
        }
        result.map(|records| (records, NativeCompileCacheDisposition::Miss))
    }

    pub(super) fn stats(&self) -> NativeCompileCacheStats {
        let state = lock_unpoisoned(&self.state);
        NativeCompileCacheStats {
            entries: state
                .primary_entries
                .len()
                .saturating_add(state.alias_entries.len()),
            hits: state.metrics.hits,
            misses: state.metrics.misses,
            insertions: state.metrics.insertions,
            evictions: state.metrics.evictions,
            compile_waits: state.metrics.compile_waits,
            compile_failures: state.metrics.compile_failures,
            compile_time_nanos: state.metrics.compile_time_nanos,
        }
    }
}

fn touch_lru(lru: &mut VecDeque<NativeCompileCacheKey>, key: NativeCompileCacheKey) {
    remove_lru(lru, key);
    lru.push_back(key);
}

fn remove_lru(lru: &mut VecDeque<NativeCompileCacheKey>, key: NativeCompileCacheKey) {
    if let Some(index) = lru.iter().position(|candidate| *candidate == key) {
        lru.remove(index);
    }
}

fn insert_primary(
    state: &mut NativeCompileCacheState,
    key: NativeCompileCacheKey,
    records: Arc<[JitUnitCompileRecord]>,
) {
    state.alias_entries.remove(&key);
    remove_lru(&mut state.alias_lru, key);
    state.primary_entries.insert(key, records);
    touch_lru(&mut state.primary_lru, key);
}

fn insert_alias(
    state: &mut NativeCompileCacheState,
    key: NativeCompileCacheKey,
    records: Arc<[JitUnitCompileRecord]>,
) {
    state.alias_entries.insert(key, records);
    touch_lru(&mut state.alias_lru, key);
}

fn evict_primary_entries(state: &mut NativeCompileCacheState, capacity: usize) {
    while state.primary_entries.len() > capacity {
        let Some(evicted) = state.primary_lru.pop_front() else {
            break;
        };
        if state.primary_entries.remove(&evicted).is_some() {
            state.metrics.evictions = state.metrics.evictions.saturating_add(1);
        }
    }
}

fn evict_alias_entries(state: &mut NativeCompileCacheState, capacity: usize) {
    while state.alias_entries.len() > capacity {
        let Some(evicted) = state.alias_lru.pop_front() else {
            break;
        };
        if state.alias_entries.remove(&evicted).is_some() {
            state.metrics.evictions = state.metrics.evictions.saturating_add(1);
        }
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    fn key(unit: u64) -> NativeCompileCacheKey {
        NativeCompileCacheKey::new(unit, FunctionId::new(0), false, 0)
    }

    fn record(function: u32) -> JitUnitCompileRecord {
        JitUnitCompileRecord {
            function: FunctionId::new(function),
            result: php_jit::JitCompileResult {
                status: php_jit::JitCompileStatus::Rejected {
                    reason: "test-only".to_owned(),
                },
                handle: None,
                diagnostics: Vec::new(),
                stats: php_jit::JitStats::default(),
            },
        }
    }

    #[test]
    fn cache_is_bounded_and_uses_lru_eviction() {
        let cache = NativeCompileCache::new(2);
        let compile = || Ok(Vec::new());
        cache.get_or_compile(key(1), compile).unwrap();
        cache.get_or_compile(key(2), compile).unwrap();
        cache.get_or_compile(key(1), compile).unwrap();
        cache.get_or_compile(key(3), compile).unwrap();

        let (_, disposition) = cache.get_or_compile(key(2), compile).unwrap();
        assert_eq!(disposition, NativeCompileCacheDisposition::Miss);
        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        assert_eq!(stats.evictions, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 4);
    }

    #[test]
    fn concurrent_same_key_compiles_once() {
        let cache = Arc::new(NativeCompileCache::new(2));
        let compiles = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let compiles = Arc::clone(&compiles);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    cache
                        .get_or_compile(key(1), || {
                            compiles.fetch_add(1, Ordering::Relaxed);
                            thread::sleep(Duration::from_millis(25));
                            Ok(Vec::new())
                        })
                        .unwrap()
                        .1
                })
            })
            .collect::<Vec<_>>();
        let dispositions = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(compiles.load(Ordering::Relaxed), 1);
        assert!(dispositions.contains(&NativeCompileCacheDisposition::Miss));
        assert!(dispositions.contains(&NativeCompileCacheDisposition::Wait));
        assert_eq!(cache.stats().compile_waits, 1);
    }

    #[test]
    fn one_region_record_set_publishes_all_function_aliases() {
        let cache = NativeCompileCache::new(8);
        let (records, first) = cache
            .get_or_compile(key(7), || Ok(vec![record(0), record(3)]))
            .unwrap();
        let alias_key = NativeCompileCacheKey::new(7, FunctionId::new(3), false, 0);
        let (alias_records, second) = cache
            .get_or_compile(alias_key, || panic!("published alias recompiled"))
            .unwrap();

        assert_eq!(first, NativeCompileCacheDisposition::Miss);
        assert_eq!(second, NativeCompileCacheDisposition::Hit);
        assert!(Arc::ptr_eq(&records, &alias_records));
        assert_eq!(cache.stats().entries, 2);
    }

    #[test]
    fn alias_churn_does_not_evict_requested_compile_keys() {
        let cache = NativeCompileCache::new(2);
        cache
            .get_or_compile(key(1), || {
                Ok(vec![record(0), record(1), record(2), record(3)])
            })
            .unwrap();
        cache
            .get_or_compile(key(2), || {
                Ok(vec![record(0), record(1), record(2), record(3)])
            })
            .unwrap();

        let (_, disposition) = cache
            .get_or_compile(key(1), || panic!("alias churn evicted a requested key"))
            .unwrap();

        assert_eq!(disposition, NativeCompileCacheDisposition::Hit);
        assert_eq!(cache.stats().entries, 4);
    }
}
