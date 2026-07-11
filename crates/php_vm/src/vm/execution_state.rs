//! Core request-local execution state and invalidation epochs.

use super::prelude::*;

#[derive(Debug, Default)]
pub(super) struct ExecutionState {
    /// Worker-stable symbol epochs enabled (VmOptions::worker_symbol_epoch).
    pub(super) worker_symbol_epoch: bool,
    pub(super) globals: GlobalSymbolTable,
    pub(super) included_once: Vec<PathBuf>,
    pub(super) included_once_set: HashSet<PathBuf>,
    pub(super) include_stack: Vec<PathBuf>,
    pub(super) cwd: PathBuf,
    /// Request-invariant: network builtins explicitly enabled via env.
    /// Precomputed once so builtin dispatch does not rescan the env table.
    pub(super) network_requests_enabled: bool,
    pub(super) static_locals: HashMap<(u32, String), ReferenceCell>,
    pub(super) static_properties: HashMap<(String, String), Value>,
    pub(super) enum_cases: HashMap<(String, String), ObjectRef>,
    pub(super) destructor_queue: DestructorQueue,
    pub(super) magic_property_stack: Vec<MagicPropertyCall>,
    pub(super) magic_method_stack: Vec<MagicMethodCall>,
    pub(super) property_hook_stack: Vec<PropertyHookCall>,
    pub(super) generator_continuations: HashMap<u64, GeneratorContinuation>,
    pub(super) fiber_continuations: HashMap<u64, Vec<FiberContinuation>>,
    pub(super) yield_from_delegations: HashMap<YieldFromKey, YieldFromDelegation>,
    pub(super) eval_depth: usize,
    pub(super) eval_counter: usize,
    pub(super) eval_diagnostic_spans: Vec<RuntimeSourceSpan>,
    pub(super) function_table_epoch: u64,
    pub(super) autoload_stack_epoch: u64,
    pub(super) class_table_epoch: u64,
    pub(super) include_config_epoch: u64,
    pub(super) parsed_include_path: Arc<Vec<PathBuf>>,
    pub(super) class_relation_cache: ClassRelationCache,
    pub(super) autoload_registry: AutoloadRegistry,
    pub(super) autoload_stack: Vec<String>,
    pub(super) spl_autoload_extensions: String,
    /// Composer autoload-map fingerprint observed once per request on first
    /// autoload-cache use. Outer `None` = not yet computed; inner `None` = no
    /// map detected (unknown, blocks persistent reuse keyed on it).
    pub(super) composer_map_fingerprint: Option<Option<Arc<str>>>,
    pub(super) dynamic_units: Vec<CompiledUnit>,
    pub(super) dynamic_unit_index: HashMap<u64, usize>,
    pub(super) dynamic_functions: Vec<DynamicFunctionEntry>,
    pub(super) dynamic_function_index: HashMap<String, usize>,
    pub(super) dynamic_classes: Vec<DynamicClassEntry>,
    pub(super) dynamic_class_index: HashMap<String, usize>,
    pub(super) dynamic_constants: Vec<DynamicConstantEntry>,
    pub(super) dynamic_constant_index: HashMap<String, usize>,
    pub(super) validated_class_dependencies: HashSet<String>,
    pub(super) failed_class_declarations: HashSet<String>,
    pub(super) user_constants: HashMap<String, Value>,
    pub(super) shutdown_functions: Vec<ShutdownFunctionEntry>,
    pub(super) ini: IniRegistry,
    pub(super) default_timezone: String,
    pub(super) env: Arc<Vec<(String, String)>>,
    pub(super) filter_input_arrays: Rc<BTreeMap<i64, PhpArray>>,
    pub(super) resources: ResourceTable,
    pub(super) stdin: Option<php_runtime::ResourceRef>,
    pub(super) stdout: Option<php_runtime::ResourceRef>,
    pub(super) stderr: Option<php_runtime::ResourceRef>,
    pub(super) builtins: BuiltinAdapterState,
    pub(super) last_error: Option<LastErrorEntry>,
    pub(super) request: RequestLifecycleState,
    pub(super) error_handlers: Vec<ErrorHandlerEntry>,
    pub(super) exception_handlers: Vec<CallableValue>,
    pub(super) diagnostics: Vec<RuntimeDiagnostic>,
    pub(super) suppress_array_to_string_warnings: usize,
    pub(super) execution_deadline_at: Option<Instant>,
    pub(super) execution_deadline_mutable: bool,
    pub(super) process_exit_code: Option<i32>,
    /// Throwable propagating up the call stack toward an enclosing handler.
    ///
    /// Set when a frame cannot handle a throw locally; each caller frame gets a
    /// chance to catch it before the entry point renders it as uncaught.
    pub(super) pending_throw: Option<Value>,
    /// Stack trace captured at the throw origin (before unwinding), rendered as
    /// PHP's `Stack trace:` body for the uncaught-error message.
    pub(super) pending_trace: Option<String>,
}

impl ExecutionState {
    pub(super) fn has_included(&self, path: &Path) -> bool {
        self.included_once_set.contains(path)
    }

    pub(super) fn record_included(&mut self, path: PathBuf) -> bool {
        if !self.included_once_set.insert(path.clone()) {
            return false;
        }
        self.included_once.push(path);
        true
    }
}

impl ExecutionState {
    pub(super) fn push_dynamic_unit(&mut self, unit: CompiledUnit) -> usize {
        let index = self.dynamic_units.len();
        let identity = unit.cache_identity();
        self.dynamic_units.push(unit);
        self.dynamic_unit_index.insert(identity, index);
        index
    }

    pub(super) fn push_dynamic_function(&mut self, entry: DynamicFunctionEntry) {
        let index = self.dynamic_functions.len();
        self.dynamic_function_index
            .entry(entry.name.clone())
            .or_insert(index);
        self.dynamic_functions.push(entry);
    }

    pub(super) fn push_dynamic_class(&mut self, entry: DynamicClassEntry) {
        let index = self.dynamic_classes.len();
        self.dynamic_class_index
            .entry(entry.lookup_name.clone())
            .or_insert(index);
        self.dynamic_classes.push(entry);
    }

    pub(super) fn push_dynamic_constant(&mut self, entry: DynamicConstantEntry) {
        let index = self.dynamic_constants.len();
        self.dynamic_constant_index
            .entry(entry.name.clone())
            .or_insert(index);
        self.dynamic_constants.push(entry);
    }

    pub(super) fn lookup_epoch(&self) -> InvalidationEpoch {
        InvalidationEpoch::new(self.function_table_epoch)
    }

    pub(super) fn bump_lookup_epoch(&mut self) {
        if self.worker_symbol_epoch {
            // Advance the worker ledger so the epoch stays monotonic across
            // requests on this thread; per-request state re-seeds from it.
            self.function_table_epoch = WORKER_SYMBOL_LEDGER.with(|ledger| {
                let next = ledger.epoch.get().saturating_add(1);
                ledger.epoch.set(next);
                next
            });
        } else {
            self.function_table_epoch = self.function_table_epoch.saturating_add(1);
        }
    }

    pub(super) fn autoload_class_lookup_epochs(&self) -> AutoloadClassLookupEpochs {
        AutoloadClassLookupEpochs {
            autoload_stack_epoch: self.autoload_stack_epoch,
            class_table_epoch: self.class_table_epoch,
            include_config_epoch: self.include_config_epoch,
        }
    }

    pub(super) fn class_relation_epochs(&self) -> ClassRelationEpochs {
        ClassRelationEpochs {
            class_table_epoch: self.class_table_epoch,
            autoload_epoch: self.autoload_stack_epoch,
            include_eval_epoch: self.include_config_epoch.wrapping_mul(1_000_003)
                ^ self.eval_counter as u64,
            trait_interface_map_version: self.class_table_epoch,
            method_table_version: self.function_table_epoch,
        }
    }

    pub(super) fn bump_autoload_stack_epoch(&mut self) {
        self.autoload_stack_epoch = self.autoload_stack_epoch.saturating_add(1);
        self.bump_lookup_epoch();
    }

    pub(super) fn bump_class_table_epoch(&mut self) {
        self.class_table_epoch = self.class_table_epoch.saturating_add(1);
        self.bump_lookup_epoch();
    }

    pub(super) fn bump_include_config_epoch(&mut self) {
        self.include_config_epoch = self.include_config_epoch.saturating_add(1);
        self.parsed_include_path = parse_ini_include_path(&self.ini);
        self.bump_lookup_epoch();
    }
}
