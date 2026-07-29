// Audited native ABI surface; see ADR 0017. The product compiler graph always
// includes this module.
use php_ir::module::{normalize_class_name, normalized_class_name};
use php_runtime::experimental::WeakObjectHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

mod baseline_call_dispatch;
mod baseline_call_support;
mod baseline_callables;
mod baseline_class_constants;
mod baseline_context;
mod baseline_dynamic_code;
mod baseline_fibers;
mod baseline_internal_classes;
mod baseline_iterators;
mod baseline_native_builtins;
mod baseline_object_materialization;
mod baseline_object_support;
mod baseline_properties;
mod baseline_reference_ownership;
mod baseline_request_boundaries;
mod baseline_root_index;
mod baseline_runtime_ops;
mod baseline_semantic_dispatch;
mod baseline_static_properties;
mod baseline_value_plane;
mod baseline_value_semantics;
mod cold_diagnostics;
mod cold_dynamic_units;
mod cold_publication;
mod diagnostic_helpers;
mod diagnostic_telemetry;
mod exact_call_dispatch;
mod exact_runtime_ops;
mod frame_arena;
mod request_state;

use cold_dynamic_units::*;
pub(super) use cold_dynamic_units::{jit_native_function_resolve_abi, native_entries_from_records};
use frame_arena::NativeFrameArena;
pub(super) use frame_arena::{jit_native_frame_alloc_abi, jit_native_frame_release_abi};

pub(super) use baseline_call_dispatch::{
    jit_baseline_native_builtin_dispatch_abi, jit_baseline_native_builtin_dispatch_diagnostic_abi,
    jit_baseline_native_call_dispatch_abi, jit_baseline_native_call_dispatch_diagnostic_abi,
};
use baseline_call_support::*;
use baseline_callables::{
    execute_baseline_acquire_callable, execute_baseline_resolve_callable,
    rebind_baseline_materialized_closure,
};
use baseline_class_constants::{
    baseline_class_constant_result_is_cacheable, execute_baseline_class_constant,
};
pub(in crate::vm) use baseline_context::activate_native_context;
#[cfg(test)]
use baseline_context::{
    ACTIVE_BASELINE_CONTEXT, ActiveBaselineContext, NativeRequestActivationGuard,
};
use baseline_context::{
    active_baseline_cold_context, with_baseline_native_context_for,
    with_baseline_native_context_for_unit,
};
pub(super) use baseline_dynamic_code::jit_native_dynamic_code_abi;
use baseline_dynamic_code::{
    BASELINE_INCLUDE_CONSTANTS, BASELINE_INCLUDE_DEFAULT_TIMEZONE, BASELINE_INCLUDE_EXPORTS,
    BASELINE_INCLUDE_FILES, BASELINE_INCLUDE_FILTER_INPUT_ARRAYS, BASELINE_INCLUDE_FUNCTION_NAMES,
    BASELINE_INCLUDE_GLOBALS, BASELINE_INCLUDE_HTTP_RESPONSE, BASELINE_INCLUDE_INI,
    BASELINE_INCLUDE_MYSQL, BASELINE_INCLUDE_SYMBOLS,
};
use baseline_internal_classes::*;
use baseline_iterators::{BaselineGeneratorDelegation, NativeColdIterator};
use baseline_native_builtins::{
    NativeDimensionOperation, emit_native_array_dimension_conversion_diagnostic,
    emit_native_deprecated_call, emit_native_dimension_conversion_diagnostic,
    emit_native_external_deprecated_call, emit_native_php_diagnostic, emit_native_php_warning,
    execute_baseline_native_builtin, execute_baseline_native_builtin_control,
    execute_baseline_prepared_runtime_builtin, native_internal_class_constant_exists,
    native_php_function_exists, native_source_line, native_source_line_for_span, native_string,
};
use baseline_object_materialization::*;
use baseline_object_support::*;
use baseline_properties::execute_native_property_instruction;
use baseline_root_index::{
    RequestRootIndex, RootMutationReason, baseline_shared_array_storage_contains_object,
    rooted_membership_may_change, values_contain_object,
};
pub(super) use baseline_runtime_ops::{
    jit_baseline_native_binary_abi, jit_baseline_native_cast_abi, jit_baseline_native_compare_abi,
    jit_baseline_native_unary_abi, jit_native_argument_check_abi, jit_native_array_fetch_abi,
    jit_native_array_insert_abi, jit_native_array_insert_local_abi, jit_native_array_new_abi,
    jit_native_array_spread_abi, jit_native_array_unset_abi, jit_native_constant_fetch_abi,
    jit_native_echo_abi, jit_native_exception_new_abi, jit_native_execution_poll_abi,
    jit_native_foreach_cleanup_abi, jit_native_foreach_init_abi, jit_native_foreach_next_abi,
    jit_native_local_fetch_abi, jit_native_local_store_abi, jit_native_object_clone_abi,
    jit_native_object_clone_with_abi, jit_native_object_new_abi, jit_native_property_assign_abi,
    jit_native_property_fetch_abi, jit_native_reference_bind_abi, jit_native_return_check_abi,
    jit_native_runtime_fatal_abi, jit_native_stable_length_abi, jit_native_string_predicate_abi,
    jit_native_truthy_abi, jit_native_type_predicate_abi, jit_native_value_release_abi,
};
use baseline_semantic_dispatch::*;
pub(super) use baseline_semantic_dispatch::{
    jit_baseline_native_semantic_dispatch_abi, jit_baseline_native_semantic_dispatch_diagnostic_abi,
};
use baseline_static_properties::execute_native_static_property;
use baseline_value_plane::{
    BaselineValueState, NativeIncludeExports, NativeIncludeSymbols,
    NativeRegisteredAutoloadCallback, NativeRegisteredCallbackSource,
    NativeRegisteredCallbackState, NativeRegisteredCallbackTransfer, NativeRegisteredErrorHandler,
    NativeRegisteredShutdownCallback, baseline_shared_array_storage_is_empty,
    release_baseline_shared_array_storage,
};
use baseline_value_semantics::*;
use cold_diagnostics::*;
pub(in crate::vm) use diagnostic_helpers::*;
use diagnostic_telemetry::NativeRuntimeTelemetry;
pub(super) use exact_call_dispatch::{
    jit_native_addcslashes_abi, jit_native_array_merge_recursive_abi,
    jit_native_array_multisort_abi, jit_native_array_rand_abi,
    jit_native_array_replace_recursive_abi, jit_native_array_sum_abi, jit_native_arsort_abi,
    jit_native_asort_abi, jit_native_base_convert_abi, jit_native_base64_decode_abi,
    jit_native_base64_encode_abi, jit_native_basename_abi, jit_native_bcadd_abi,
    jit_native_bccomp_abi, jit_native_bcdiv_abi, jit_native_bcmod_abi, jit_native_bcmul_abi,
    jit_native_bcpow_abi, jit_native_bcpowmod_abi, jit_native_bcscale_abi, jit_native_bcsqrt_abi,
    jit_native_bcsub_abi, jit_native_bin2hex_abi, jit_native_bindec_abi, jit_native_chdir_abi,
    jit_native_checkdate_abi, jit_native_chmod_abi, jit_native_class_exists_abi,
    jit_native_class_implements_abi, jit_native_clearstatcache_abi, jit_native_closedir_abi,
    jit_native_compact_abi, jit_native_constant_abi, jit_native_convert_uudecode_abi,
    jit_native_convert_uuencode_abi, jit_native_crc32_abi, jit_native_date_abi,
    jit_native_date_default_timezone_get_abi, jit_native_date_default_timezone_set_abi,
    jit_native_decbin_abi, jit_native_dechex_abi, jit_native_decoct_abi, jit_native_define_abi,
    jit_native_defined_abi, jit_native_dirname_abi, jit_native_disk_free_space_abi,
    jit_native_disk_total_space_abi, jit_native_enum_exists_abi, jit_native_error_clear_last_abi,
    jit_native_error_get_last_abi, jit_native_extension_loaded_abi, jit_native_fclose_abi,
    jit_native_feof_abi, jit_native_fflush_abi, jit_native_fgetc_abi, jit_native_fgets_abi,
    jit_native_file_abi, jit_native_file_exists_abi, jit_native_file_get_contents_abi,
    jit_native_file_put_contents_abi, jit_native_filegroup_abi, jit_native_filemtime_abi,
    jit_native_fileowner_abi, jit_native_fileperms_abi, jit_native_filesize_abi,
    jit_native_filetype_abi, jit_native_filter_has_var_abi, jit_native_filter_id_abi,
    jit_native_filter_input_abi, jit_native_filter_input_array_abi, jit_native_filter_list_abi,
    jit_native_filter_var_abi, jit_native_filter_var_array_abi, jit_native_fopen_abi,
    jit_native_fread_abi, jit_native_fseek_abi, jit_native_ftell_abi, jit_native_ftruncate_abi,
    jit_native_func_get_arg_abi, jit_native_func_get_args_abi, jit_native_func_num_args_abi,
    jit_native_function_exists_abi, jit_native_fwrite_abi, jit_native_gc_collect_cycles_abi,
    jit_native_gc_disable_abi, jit_native_gc_enable_abi, jit_native_gc_enabled_abi,
    jit_native_gc_mem_caches_abi, jit_native_gc_status_abi, jit_native_get_cfg_var_abi,
    jit_native_get_class_methods_abi, jit_native_get_class_vars_abi,
    jit_native_get_current_user_abi, jit_native_get_declared_classes_abi,
    jit_native_get_declared_interfaces_abi, jit_native_get_declared_traits_abi,
    jit_native_get_defined_constants_abi, jit_native_get_defined_functions_abi,
    jit_native_get_exception_handler_abi, jit_native_get_include_path_abi,
    jit_native_get_included_files_abi, jit_native_get_loaded_extensions_abi,
    jit_native_get_mangled_object_vars_abi, jit_native_get_object_vars_abi,
    jit_native_get_parent_class_abi, jit_native_get_resource_id_abi,
    jit_native_get_resource_type_abi, jit_native_get_resources_abi, jit_native_getcwd_abi,
    jit_native_getenv_abi, jit_native_getrandmax_abi, jit_native_glob_abi, jit_native_gmdate_abi,
    jit_native_gmmktime_abi, jit_native_gzcompress_abi, jit_native_gzdecode_abi,
    jit_native_gzdeflate_abi, jit_native_gzencode_abi, jit_native_gzinflate_abi,
    jit_native_gzuncompress_abi, jit_native_hash_abi, jit_native_hash_equals_abi,
    jit_native_hash_hmac_abi, jit_native_header_abi, jit_native_header_remove_abi,
    jit_native_headers_list_abi, jit_native_headers_sent_abi, jit_native_hex2bin_abi,
    jit_native_hexdec_abi, jit_native_hrtime_abi, jit_native_html_entity_decode_abi,
    jit_native_htmlentities_abi, jit_native_htmlspecialchars_abi,
    jit_native_htmlspecialchars_decode_abi, jit_native_http_build_query_abi,
    jit_native_http_response_code_abi, jit_native_inet_ntop_abi, jit_native_inet_pton_abi,
    jit_native_ini_get_abi, jit_native_ini_get_all_abi, jit_native_ini_set_abi,
    jit_native_interface_exists_abi, jit_native_intval_base_abi, jit_native_ip2long_abi,
    jit_native_is_a_abi, jit_native_is_callable_abi, jit_native_is_dir_abi, jit_native_is_file_abi,
    jit_native_is_link_abi, jit_native_is_readable_abi, jit_native_is_subclass_of_abi,
    jit_native_is_uploaded_file_abi, jit_native_is_writable_abi, jit_native_json_decode_abi,
    jit_native_json_encode_abi, jit_native_json_last_error_abi, jit_native_json_last_error_msg_abi,
    jit_native_json_validate_abi, jit_native_krsort_abi, jit_native_ksort_abi,
    jit_native_long2ip_abi, jit_native_lstat_abi, jit_native_mb_check_encoding_abi,
    jit_native_mb_chr_abi, jit_native_mb_convert_case_abi, jit_native_mb_convert_encoding_abi,
    jit_native_mb_detect_encoding_abi, jit_native_mb_encoding_aliases_abi,
    jit_native_mb_internal_encoding_abi, jit_native_mb_lcfirst_abi,
    jit_native_mb_list_encodings_abi, jit_native_mb_ord_abi, jit_native_mb_parse_str_abi,
    jit_native_mb_strcut_abi, jit_native_mb_strimwidth_abi, jit_native_mb_stripos_abi,
    jit_native_mb_strlen_abi, jit_native_mb_strpos_abi, jit_native_mb_strripos_abi,
    jit_native_mb_strrpos_abi, jit_native_mb_strtolower_abi, jit_native_mb_strtoupper_abi,
    jit_native_mb_strwidth_abi, jit_native_mb_substitute_character_abi, jit_native_mb_substr_abi,
    jit_native_mb_substr_count_abi, jit_native_mb_ucfirst_abi, jit_native_md5_abi,
    jit_native_memory_get_peak_usage_abi, jit_native_memory_get_usage_abi,
    jit_native_method_exists_abi, jit_native_microtime_abi, jit_native_mkdir_abi,
    jit_native_mktime_abi, jit_native_mt_getrandmax_abi, jit_native_mt_rand_abi,
    jit_native_natcasesort_abi, jit_native_natsort_abi, jit_native_number_format_abi,
    jit_native_ob_end_clean_abi, jit_native_ob_end_flush_abi, jit_native_ob_get_clean_abi,
    jit_native_ob_get_contents_abi, jit_native_ob_get_flush_abi, jit_native_ob_get_length_abi,
    jit_native_ob_get_level_abi, jit_native_ob_start_abi, jit_native_octdec_abi,
    jit_native_opendir_abi, jit_native_pack_abi, jit_native_parse_str_abi,
    jit_native_parse_url_abi, jit_native_pathinfo_abi, jit_native_php_sapi_name_abi,
    jit_native_php_uname_abi, jit_native_preg_callback_assemble_abi,
    jit_native_preg_callback_plan_abi, jit_native_preg_filter_abi, jit_native_preg_grep_abi,
    jit_native_preg_last_error_abi, jit_native_preg_last_error_msg_abi, jit_native_preg_match_abi,
    jit_native_preg_match_all_abi, jit_native_preg_quote_abi, jit_native_preg_replace_abi,
    jit_native_preg_split_abi, jit_native_printf_abi, jit_native_property_exists_abi,
    jit_native_quoted_printable_decode_abi, jit_native_quotemeta_abi, jit_native_rand_abi,
    jit_native_random_bytes_abi, jit_native_random_int_abi, jit_native_rawurldecode_abi,
    jit_native_rawurlencode_abi, jit_native_readdir_abi, jit_native_readfile_abi,
    jit_native_realpath_abi, jit_native_register_shutdown_function_abi, jit_native_rename_abi,
    jit_native_restore_error_handler_abi, jit_native_restore_exception_handler_abi,
    jit_native_rewind_abi, jit_native_rewinddir_abi, jit_native_rmdir_abi, jit_native_rsort_abi,
    jit_native_scandir_abi, jit_native_serialize_abi, jit_native_session_abort_abi,
    jit_native_session_cache_expire_abi, jit_native_session_cache_limiter_abi,
    jit_native_session_commit_abi, jit_native_session_create_id_abi, jit_native_session_decode_abi,
    jit_native_session_destroy_abi, jit_native_session_encode_abi, jit_native_session_gc_abi,
    jit_native_session_get_cookie_params_abi, jit_native_session_id_abi,
    jit_native_session_module_name_abi, jit_native_session_name_abi,
    jit_native_session_regenerate_id_abi, jit_native_session_register_shutdown_abi,
    jit_native_session_reset_abi, jit_native_session_save_path_abi,
    jit_native_session_set_cookie_params_abi, jit_native_session_set_save_handler_abi,
    jit_native_session_start_abi, jit_native_session_status_abi, jit_native_session_unset_abi,
    jit_native_session_write_close_abi, jit_native_set_error_handler_abi,
    jit_native_set_exception_handler_abi, jit_native_set_include_path_abi,
    jit_native_setcookie_abi, jit_native_setrawcookie_abi, jit_native_settype_abi,
    jit_native_sha1_abi, jit_native_shuffle_abi, jit_native_sort_abi,
    jit_native_spl_autoload_functions_abi, jit_native_spl_autoload_register_abi,
    jit_native_spl_autoload_unregister_abi, jit_native_spl_object_hash_abi,
    jit_native_spl_object_id_abi, jit_native_sprintf_abi, jit_native_stat_abi,
    jit_native_str_pad_abi, jit_native_str_split_abi, jit_native_stream_context_create_abi,
    jit_native_stream_context_get_default_abi, jit_native_stream_context_get_options_abi,
    jit_native_stream_context_set_default_abi, jit_native_stream_context_set_option_abi,
    jit_native_stream_context_set_options_abi, jit_native_stream_copy_to_stream_abi,
    jit_native_stream_filter_append_abi, jit_native_stream_filter_prepend_abi,
    jit_native_stream_filter_remove_abi, jit_native_stream_get_contents_abi,
    jit_native_stream_get_meta_data_abi, jit_native_stream_get_wrappers_abi,
    jit_native_stream_is_local_abi, jit_native_stream_isatty_abi,
    jit_native_stream_resolve_include_path_abi, jit_native_stream_set_timeout_abi,
    jit_native_strip_tags_abi, jit_native_stripcslashes_abi, jit_native_stripslashes_abi,
    jit_native_stristr_abi, jit_native_strnatcasecmp_abi, jit_native_strnatcmp_abi,
    jit_native_strpbrk_abi, jit_native_strrchr_abi, jit_native_strstr_abi,
    jit_native_strtotime_abi, jit_native_strtr_abi, jit_native_substr_compare_abi,
    jit_native_substr_replace_abi, jit_native_symlink_abi, jit_native_sys_get_temp_dir_abi,
    jit_native_tempnam_abi, jit_native_time_abi, jit_native_timezone_identifiers_list_abi,
    jit_native_tmpfile_abi, jit_native_token_get_all_abi, jit_native_token_name_abi,
    jit_native_touch_abi, jit_native_trait_exists_abi, jit_native_ucwords_abi,
    jit_native_umask_abi, jit_native_unlink_abi, jit_native_unpack_abi, jit_native_unserialize_abi,
    jit_native_urldecode_abi, jit_native_urlencode_abi, jit_native_version_compare_abi,
    jit_native_vprintf_abi, jit_native_vsprintf_abi, jit_native_zlib_decode_abi,
    jit_native_zlib_encode_abi,
};
pub(super) use exact_runtime_ops::{
    jit_native_acos_f64_abi, jit_native_acosh_f64_abi, jit_native_acquire_callable_abi,
    jit_native_add_abi, jit_native_array_cast_abi, jit_native_asin_f64_abi,
    jit_native_asinh_f64_abi, jit_native_atan_f64_abi, jit_native_atan2_f64_abi,
    jit_native_atanh_f64_abi, jit_native_bit_and_abi, jit_native_bit_not_abi,
    jit_native_bit_or_abi, jit_native_bit_xor_abi, jit_native_callback_return_string_abi,
    jit_native_concat_abi, jit_native_cos_f64_abi, jit_native_cosh_f64_abi, jit_native_count_abi,
    jit_native_deg2rad_f64_abi, jit_native_divide_abi, jit_native_dynamic_instanceof_abi,
    jit_native_dynamic_property_slot_abi, jit_native_dynamic_property_test_slot_abi,
    jit_native_echo_bytes_abi, jit_native_equal_abi, jit_native_exp_f64_abi,
    jit_native_expm1_f64_abi, jit_native_float_cast_abi, jit_native_float_to_string_abi,
    jit_native_fmod_f64_abi, jit_native_fpow_f64_abi, jit_native_greater_abi,
    jit_native_greater_equal_abi, jit_native_hypot_f64_abi, jit_native_identical_abi,
    jit_native_int_cast_abi, jit_native_less_abi, jit_native_less_equal_abi,
    jit_native_log_f64_abi, jit_native_log1p_f64_abi, jit_native_log10_f64_abi,
    jit_native_modulo_abi, jit_native_multiply_abi, jit_native_not_equal_abi,
    jit_native_not_identical_abi, jit_native_numeric_string_abi, jit_native_object_cast_abi,
    jit_native_object_class_name_abi, jit_native_plain_object_clone_abi, jit_native_power_abi,
    jit_native_prepared_closure_new_abi, jit_native_prepared_exception_new_abi,
    jit_native_prepared_object_new_abi, jit_native_rad2deg_f64_abi,
    jit_native_resolve_callable_abi, jit_native_round_f64_abi, jit_native_shift_left_abi,
    jit_native_shift_right_abi, jit_native_sin_f64_abi, jit_native_sinh_f64_abi,
    jit_native_sizeof_abi, jit_native_spaceship_abi, jit_native_string_cast_abi,
    jit_native_subtract_abi, jit_native_tan_f64_abi, jit_native_tanh_f64_abi,
    jit_native_unary_minus_abi, jit_native_unary_plus_abi,
};
use request_state::{
    NativeBacktraceFrame, NativeFunctionNameScope, NativeLastError,
    NativeRegisteredExtensionRequestState,
};

static NATIVE_TEMPNAM_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn native_direct_string_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn php_constant_category(extension: &str) -> &str {
    match extension {
        "core" => "Core",
        "pdo" => "PDO",
        "phar" => "Phar",
        "spl" => "SPL",
        extension => extension,
    }
}

fn php_core_runtime_constant(name: &str) -> bool {
    matches!(name, "STDIN" | "STDOUT" | "STDERR")
}

#[derive(Clone, Copy)]
pub(crate) struct NativeFixedCallablePlan {
    pub(crate) function: php_ir::FunctionId,
    pub(crate) visible_arity: u32,
    pub(crate) has_receiver: bool,
    pub(crate) first_parameter_by_reference: bool,
    pub(crate) returns_int: bool,
    pub(crate) returns_string: bool,
    pub(crate) returns_releasable_scalar: bool,
}

/// Narrow live capability used by exact symbol-query builtins.
///
/// The pointers address only symbol/class/constant publication fields inside
/// the stable request owner. They continue to observe include/eval updates
/// because those fields are mutated in place, while keeping the complete
/// execution coordinator, value compatibility plane, output, frames, and
/// extension state unreachable from successful exact queries.
#[derive(Default)]
struct NativeSymbolQueryCapability {
    active_compiled: *const crate::compiled_unit::CompiledUnit,
    current_dynamic_unit: *const Option<usize>,
    dynamic_units: *const Vec<NativeDynamicUnit>,
    dynamic_functions: *const std::collections::BTreeMap<String, php_ir::FunctionId>,
    external_functions: *const std::collections::HashMap<String, NativeDynamicFunction>,
    external_class_units: *const std::collections::HashMap<String, usize>,
    deployment_functions:
        *const std::sync::Arc<std::collections::HashMap<std::sync::Arc<str>, php_ir::FunctionId>>,
    deployment_classes: *const std::sync::Arc<std::collections::HashSet<std::sync::Arc<str>>>,
    visible_function_names: *const Rc<NativeFunctionNameScope>,
    native_dynamic_constants: *mut std::collections::BTreeMap<String, i64>,
    trusted_dynamic_constant_sites: *const std::collections::BTreeMap<String, Vec<usize>>,
    dynamic_classes: *const std::collections::BTreeSet<String>,
    class_aliases: *const std::collections::BTreeMap<String, String>,
}

/// Narrow live capability for exact request configuration builtins.
///
/// All pointers address stable fields in the separately boxed request owner.
/// Mutations therefore remain visible to baseline/include code without
/// exposing the cold execution coordinator or materializing Rust `Value`s.
#[derive(Default)]
struct NativeConfigurationCapability {
    ini_registry: *mut php_runtime::api::IniRegistry,
    include_path: *mut Arc<Vec<std::path::PathBuf>>,
    display_errors: *mut bool,
    default_timezone: *mut String,
}

/// Narrow live capability for exact HTTP response builtins.
///
/// The response owner is separately boxed and remains stable for the request.
/// Exact handlers mutate only this published response state and cannot recover
/// the complete cold execution coordinator.
#[derive(Default)]
struct NativeHttpResponseCapability {
    response: *mut php_runtime::api::RuntimeHttpResponseState,
}

/// Narrow live capability for exact request/environment query builtins.
///
/// These pointers address only the three request-owned collections/strings
/// required by the family. Mutating baseline operations update those owners
/// in place, so later exact reads remain current without recovering the cold
/// execution coordinator or materializing Rust `Value`s.
#[derive(Default)]
struct NativeRequestQueryCapability {
    environment: *const std::sync::Arc<Vec<(String, String)>>,
    included_files: *const std::collections::BTreeSet<std::path::PathBuf>,
    sapi_name: *const String,
}

/// Narrow live capability for the cooperative execution deadline.
///
/// Optimizing loop headers must preserve `max_execution_time` semantics, but
/// polling must not recover the complete cold execution coordinator. Both
/// pointers address stable fields in the separately boxed request owner.
#[derive(Default)]
struct NativeExecutionDeadlineCapability {
    deadline: *const Option<std::time::Instant>,
    diagnostic: *mut Option<php_runtime::api::RuntimeDiagnostic>,
}

/// Narrow request-stable capability for generated native call-frame storage.
///
/// The arena remains owned by the request buffers, but optimizing frame
/// allocation reaches only this native allocator and its diagnostic sink. It
/// never recovers `NativeRequestColdState` from the fast-state pointer.
#[derive(Default)]
struct NativeFrameArenaCapability {
    arena: *mut NativeFrameArena,
    diagnostic: *mut Option<php_runtime::api::RuntimeDiagnostic>,
}

/// Narrow request-stable mbstring capability.
///
/// Exact mbstring handlers can observe and update only the two PHP-visible
/// request settings. They cannot recover the registered-extension state or
/// the Rust `Value` execution plane that owns it.
#[repr(C)]
#[derive(Default)]
struct NativeMbstringCapability {
    internal_encoding: *mut String,
    substitute_character: *mut php_runtime::api::MbSubstituteCharacter,
}

#[repr(C)]
#[derive(Default)]
struct NativeBcmathCapability {
    scale: *mut usize,
}

/// Explicit request-published access to the platform CSPRNG.
///
/// Exact handlers cannot call the ambient random source without this
/// capability; tests and nested owners can publish a different fill function.
#[repr(C)]
#[derive(Default)]
struct NativeRandomCapability {
    fill: Option<fn(&mut [u8]) -> bool>,
}

/// Five immutable request-input roots published at owner construction.
///
/// The bitset distinguishes an absent source from a present empty array.
#[repr(C)]
#[derive(Default)]
struct NativeFilterCapability {
    roots: [i64; 5],
    present: u8,
}

/// Value-free session control capability.
///
/// `SessionState` physically keeps `PhpArray` payloads in a sibling private
/// record. Exact handlers receive only this pointer and cannot recover the
/// baseline session graph.
#[repr(C)]
#[derive(Default)]
struct NativeSessionCapability {
    control: *mut php_runtime::api::NativeSessionControlState,
    /// Canonical request-global reference for `$_SESSION`.
    global_reference: i64,
    /// Independently owned native COW snapshot used by commit/abort/reset.
    committed: i64,
    /// Transport callbacks are cold capabilities and force the one baseline
    /// continuation only when the requested operation actually needs them.
    has_loader: u8,
    has_id_generator: u8,
}

/// Authoritative request-local stream-context option owners.
///
/// Every handle is an independently owned direct native array. The runtime
/// resource keeps no parallel `PhpArray`; a baseline continuation
/// materializes and immediately republishes one compatibility snapshot.
#[derive(Default)]
struct NativeStreamContextState {
    default_options: i64,
    resource_options: std::collections::BTreeMap<u64, i64>,
}

/// Compact stable prefix passed through every generated entry and compiled
/// call. Exact native operations can reach only explicitly published
/// request-owned capabilities from this type.
#[repr(C)]
#[derive(Default)]
pub(super) struct NativeRequestFastState {
    header: php_jit::JitNativeFastStateHeader,
    output: *mut php_runtime::api::OutputBuffer,
    json_state: *mut php_runtime::api::JsonRequestState,
    pcre_state: *mut php_runtime::api::PcreRequestState,
    gc_state: *mut php_runtime::api::GcRequestState,
    cwd: *mut std::path::PathBuf,
    filesystem_capabilities: *const php_runtime::api::FilesystemCapabilities,
    filesystem_state: *mut php_runtime::api::FilesystemRuntimeState,
    stdin: *const std::sync::Arc<[u8]>,
    resources: *mut php_runtime::api::ResourceTable,
    upload_registry: *mut php_runtime::api::UploadRegistry,
    last_error: *mut Option<NativeLastError>,
    direct_resource_handles: *mut std::collections::HashMap<u64, u32>,
    direct_closure_handles: *mut std::collections::HashMap<u64, u32>,
    execution_scope: *const NativeExecutionScope,
    symbol_query: NativeSymbolQueryCapability,
    configuration: NativeConfigurationCapability,
    http_response: NativeHttpResponseCapability,
    request_query: NativeRequestQueryCapability,
    mbstring: NativeMbstringCapability,
    bcmath: NativeBcmathCapability,
    random: NativeRandomCapability,
    filter: NativeFilterCapability,
    session: NativeSessionCapability,
    stream_context: *mut NativeStreamContextState,
    callback_handlers: *mut NativeRegisteredCallbackState,
    callback_transient_export: u8,
    /// Request-stable immutable absence cell returned only by non-mutating
    /// dynamic-property tests on classes proven not to implement `__isset`.
    absent_dynamic_property_slot: php_runtime::api::NativeDeclaredPropertySlot,
    execution_deadline: NativeExecutionDeadlineCapability,
    frame_arena: NativeFrameArenaCapability,
}

/// Transactional writer for an unpublished range in the authoritative native
/// array arena. Every pushed entry transfers one key owner and one value
/// owner; publication commits the written prefix, while failure releases it
/// in reverse order.
struct NativeOwnedDirectArrayWriter {
    entries: *mut php_jit::JitNativeDirectArrayEntry,
    start: usize,
    capacity: u32,
    length: usize,
    maximum_length: usize,
}

impl NativeOwnedDirectArrayWriter {
    fn len(&self) -> usize {
        self.length
    }

    fn get(&self, index: usize) -> Option<php_jit::JitNativeDirectArrayEntry> {
        if index >= self.length {
            return None;
        }
        // SAFETY: the writer owns a reserved stable arena range and `index`
        // was checked against its initialized prefix.
        #[allow(unsafe_code)]
        Some(unsafe { *self.entries.add(index) })
    }

    fn push_owned(
        &mut self,
        entry: php_jit::JitNativeDirectArrayEntry,
    ) -> Result<(), &'static str> {
        if self.length >= self.maximum_length {
            return Err("native direct array writer exceeded its reserved range");
        }
        if self.length >= self.capacity as usize {
            return Err("native direct array writer requires growth");
        }
        // SAFETY: the reserved range has room for this next initialized entry.
        #[allow(unsafe_code)]
        unsafe {
            *self.entries.add(self.length) = entry;
        }
        self.length += 1;
        Ok(())
    }

    fn replace_owned(
        &mut self,
        index: usize,
        entry: php_jit::JitNativeDirectArrayEntry,
    ) -> Option<php_jit::JitNativeDirectArrayEntry> {
        if index >= self.length {
            return None;
        }
        // SAFETY: `index` addresses one initialized entry in the reserved
        // unpublished range.
        #[allow(unsafe_code)]
        Some(unsafe { std::mem::replace(&mut *self.entries.add(index), entry) })
    }
}

/// Bounded direct publisher for the scalar/array subset of PHP's serialized
/// wire format. Object construction, reference records, malformed input
/// warnings, and option-dependent semantics retain one baseline continuation.
struct NativeSerializedParser<'a> {
    bytes: &'a [u8],
    offset: usize,
    parsed_items: usize,
}

impl NativeSerializedParser<'_> {
    const MAX_DEPTH: usize = 64;
    const MAX_ITEMS: usize = 16_384;
    const MAX_BYTES: usize = 1_048_576;

    fn parse(mut self, publisher: &mut NativeRequestFastState) -> Option<i64> {
        if self.bytes.len() > Self::MAX_BYTES {
            return None;
        }
        let value = self.parse_value(publisher, 0)?;
        if self.offset != self.bytes.len() {
            let _ = publisher.discard_owned_direct_value(value);
            return None;
        }
        Some(value)
    }

    fn parse_prefix(mut self, publisher: &mut NativeRequestFastState) -> Option<(i64, usize)> {
        if self.bytes.len() > Self::MAX_BYTES {
            return None;
        }
        let value = self.parse_value(publisher, 0)?;
        Some((value, self.offset))
    }

    fn parse_value(&mut self, publisher: &mut NativeRequestFastState, depth: usize) -> Option<i64> {
        if depth > Self::MAX_DEPTH {
            return None;
        }
        match self.take_byte()? {
            b'N' => {
                self.expect(b';')?;
                Some(php_jit::jit_encode_constant(u32::MAX))
            }
            b'b' => {
                self.expect(b':')?;
                let value = match self.take_byte()? {
                    b'0' => false,
                    b'1' => true,
                    _ => return None,
                };
                self.expect(b';')?;
                Some(php_jit::jit_encode_constant(if value {
                    php_jit::JIT_VALUE_TRUE
                } else {
                    php_jit::JIT_VALUE_FALSE
                }))
            }
            b'i' => {
                self.expect(b':')?;
                let value = self.take_ascii_until(b';')?.parse().ok()?;
                publisher.publish_direct_int(value).ok()
            }
            b'd' => {
                self.expect(b':')?;
                let value = match self.take_ascii_until(b';')? {
                    "NAN" => f64::NAN,
                    "INF" => f64::INFINITY,
                    "-INF" => f64::NEG_INFINITY,
                    value => value.parse().ok()?,
                };
                publisher.publish_direct_float(value).ok()
            }
            b's' => {
                let (start, length) = self.parse_string_range()?;
                publisher
                    .publish_direct_string_bytes(self.bytes.get(start..start.checked_add(length)?)?)
                    .ok()
            }
            b'a' => self.parse_array(publisher, depth),
            // Native object publication and reference graphs are separate
            // semantic families, so their wire tags take the baseline once.
            b'O' | b'R' | b'r' => None,
            _ => None,
        }
    }

    fn parse_array(&mut self, publisher: &mut NativeRequestFastState, depth: usize) -> Option<i64> {
        self.expect(b':')?;
        let length = self.take_ascii_until(b':')?.parse::<usize>().ok()?;
        self.parsed_items = self.parsed_items.checked_add(length)?;
        if self.parsed_items > Self::MAX_ITEMS {
            return None;
        }
        self.expect(b'{')?;
        publisher
            .publish_owned_direct_array_dynamic(length, |publisher, writer| {
                for _ in 0..length {
                    let key = self
                        .parse_key(publisher, depth + 1)
                        .ok_or("native serialized array key is malformed")?;
                    let Some(value) = self.parse_value(publisher, depth + 1) else {
                        let _ = publisher.discard_owned_direct_value(key);
                        return Err("native serialized array value is malformed");
                    };
                    let existing = (0..writer.len()).find(|&index| {
                        writer.get(index).is_some_and(|entry| {
                            publisher.native_compare_array_keys(entry.key, key)
                                == Some(std::cmp::Ordering::Equal)
                        })
                    });
                    let entry = php_jit::JitNativeDirectArrayEntry { key, value };
                    if let Some(index) = existing {
                        let previous = writer
                            .get(index)
                            .ok_or("native serialized array entry disappeared")?;
                        let _ = publisher.discard_owned_direct_value(key);
                        let Some(replaced) = writer.replace_owned(
                            index,
                            php_jit::JitNativeDirectArrayEntry {
                                key: previous.key,
                                value,
                            },
                        ) else {
                            let _ = publisher.discard_owned_direct_value(value);
                            return Err("native serialized array replacement failed");
                        };
                        let _ = publisher.discard_owned_direct_value(replaced.value);
                    } else if let Err(error) = writer.push_owned(entry) {
                        let _ = publisher.discard_owned_direct_value(value);
                        let _ = publisher.discard_owned_direct_value(key);
                        return Err(error);
                    }
                }
                self.expect(b'}')
                    .ok_or("native serialized array is not terminated")
            })
            .ok()
    }

    fn parse_key(&mut self, publisher: &mut NativeRequestFastState, depth: usize) -> Option<i64> {
        if depth > Self::MAX_DEPTH {
            return None;
        }
        match self.take_byte()? {
            b'i' => {
                self.expect(b':')?;
                let value = self.take_ascii_until(b';')?.parse().ok()?;
                publisher.publish_direct_int(value).ok()
            }
            b's' => {
                let (start, length) = self.parse_string_range()?;
                let bytes = self.bytes.get(start..start.checked_add(length)?)?;
                if let Some(value) = php_runtime::api::array_key_integer_bytes(bytes) {
                    publisher.publish_direct_int(value).ok()
                } else {
                    publisher.publish_direct_string_bytes(bytes).ok()
                }
            }
            _ => None,
        }
    }

    fn parse_string_range(&mut self) -> Option<(usize, usize)> {
        self.expect(b':')?;
        let length = self.take_ascii_until(b':')?.parse::<usize>().ok()?;
        self.expect(b'"')?;
        let start = self.offset;
        let end = self.offset.checked_add(length)?;
        self.bytes.get(self.offset..end)?;
        self.offset = end;
        self.expect(b'"')?;
        self.expect(b';')?;
        Some((start, length))
    }

    fn take_ascii_until(&mut self, delimiter: u8) -> Option<&str> {
        let start = self.offset;
        while self.bytes.get(self.offset).copied()? != delimiter {
            self.offset = self.offset.checked_add(1)?;
        }
        let value = std::str::from_utf8(self.bytes.get(start..self.offset)?).ok()?;
        self.offset += 1;
        Some(value)
    }

    fn take_byte(&mut self) -> Option<u8> {
        let byte = self.bytes.get(self.offset).copied()?;
        self.offset += 1;
        Some(byte)
    }

    fn expect(&mut self, expected: u8) -> Option<()> {
        (self.take_byte()? == expected).then_some(())
    }
}

/// Allocation-free structural cursor over the native serialized subset.
/// Session decoding uses it to determine the exact top-level entry count
/// before reserving the authoritative result array.
struct NativeSerializedCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    parsed_items: usize,
}

impl<'a> NativeSerializedCursor<'a> {
    fn skip_prefix(bytes: &'a [u8]) -> Option<usize> {
        if bytes.len() > NativeSerializedParser::MAX_BYTES {
            return None;
        }
        let mut cursor = Self {
            bytes,
            offset: 0,
            parsed_items: 0,
        };
        cursor.skip_value(0)?;
        Some(cursor.offset)
    }

    fn skip_value(&mut self, depth: usize) -> Option<()> {
        if depth > NativeSerializedParser::MAX_DEPTH {
            return None;
        }
        match self.take_byte()? {
            b'N' => self.expect(b';'),
            b'b' => {
                self.expect(b':')?;
                if !matches!(self.take_byte()?, b'0' | b'1') {
                    return None;
                }
                self.expect(b';')
            }
            b'i' => {
                self.expect(b':')?;
                self.take_ascii_until(b';')?.parse::<i64>().ok()?;
                Some(())
            }
            b'd' => {
                self.expect(b':')?;
                match self.take_ascii_until(b';')? {
                    "NAN" | "INF" | "-INF" => {}
                    value => {
                        value.parse::<f64>().ok()?;
                    }
                }
                Some(())
            }
            b's' => self.skip_string(),
            b'a' => self.skip_array(depth),
            b'O' | b'R' | b'r' => None,
            _ => None,
        }
    }

    fn skip_array(&mut self, depth: usize) -> Option<()> {
        self.expect(b':')?;
        let length = self.take_ascii_until(b':')?.parse::<usize>().ok()?;
        self.parsed_items = self.parsed_items.checked_add(length)?;
        if self.parsed_items > NativeSerializedParser::MAX_ITEMS {
            return None;
        }
        self.expect(b'{')?;
        for _ in 0..length {
            self.skip_key(depth + 1)?;
            self.skip_value(depth + 1)?;
        }
        self.expect(b'}')
    }

    fn skip_key(&mut self, depth: usize) -> Option<()> {
        if depth > NativeSerializedParser::MAX_DEPTH {
            return None;
        }
        match self.take_byte()? {
            b'i' => {
                self.expect(b':')?;
                self.take_ascii_until(b';')?.parse::<i64>().ok()?;
                Some(())
            }
            b's' => self.skip_string(),
            _ => None,
        }
    }

    fn skip_string(&mut self) -> Option<()> {
        self.expect(b':')?;
        let length = self.take_ascii_until(b':')?.parse::<usize>().ok()?;
        self.expect(b'"')?;
        self.offset = self.offset.checked_add(length)?;
        self.bytes.get(..self.offset)?;
        self.expect(b'"')?;
        self.expect(b';')
    }

    fn take_ascii_until(&mut self, delimiter: u8) -> Option<&str> {
        let start = self.offset;
        while self.bytes.get(self.offset).copied()? != delimiter {
            self.offset = self.offset.checked_add(1)?;
        }
        let value = std::str::from_utf8(self.bytes.get(start..self.offset)?).ok()?;
        self.offset += 1;
        Some(value)
    }

    fn take_byte(&mut self) -> Option<u8> {
        let byte = self.bytes.get(self.offset).copied()?;
        self.offset += 1;
        Some(byte)
    }

    fn expect(&mut self, expected: u8) -> Option<()> {
        (self.take_byte()? == expected).then_some(())
    }
}

include!("jit_abi/native_request_fast_state.rs");

// Real applications routinely cross dozens of PHP frames (for example,
// WordPress metadata and hook dispatch). Keep a deterministic native-stack
// guard, but leave enough headroom for those non-recursive call chains.
const NATIVE_CALL_DEPTH_LIMIT: usize = 256;
const NATIVE_RUNTIME_ERROR_MARKER: &str = "E_PHP_NATIVE_RUNTIME_ERROR";

#[derive(Clone)]
struct NativeTypedStaticReferenceConstraint {
    owner_display_name: String,
    property: String,
    type_: php_ir::IrReturnType,
}

#[derive(Clone, Copy)]
struct NativeDynamicFunction {
    unit: usize,
    function: php_ir::FunctionId,
}

#[derive(Clone, Copy)]
enum NativeMethodPicTarget {
    CurrentUnit {
        function: php_ir::FunctionId,
        is_static: bool,
    },
    DynamicUnit {
        function: NativeDynamicFunction,
        is_static: bool,
    },
}

struct NativeMethodPicEntry {
    receiver_class: std::sync::Arc<str>,
    method: std::sync::Arc<str>,
    class_layout_epoch: u64,
    method_table_epoch: u64,
    target: NativeMethodPicTarget,
}

#[derive(Default)]
struct NativeMethodPic {
    entries: Vec<NativeMethodPicEntry>,
    megamorphic: bool,
}

const NATIVE_METHOD_PIC_LIMIT: usize = 4;

struct NativeDynamicUnit {
    compiled: crate::compiled_unit::CompiledUnit,
    cross_unit_global_names: std::sync::Arc<[String]>,
    native_entries:
        std::sync::Arc<std::collections::BTreeMap<php_ir::FunctionId, php_jit::JitFunctionHandle>>,
    native_entry_signature_hashes: std::collections::BTreeMap<php_ir::FunctionId, u64>,
    native_entry_signature_epochs: std::collections::BTreeMap<php_ir::FunctionId, u64>,
    runtime_state: NativeUnitRuntimeState,
    linked_functions: Box<[php_jit::JitNativeLinkedFunction]>,
    published_runtime_view: Box<php_jit::JitNativeRuntimeView>,
}

impl NativeDynamicUnit {
    /// Rebind one transferred unit to the new request owner's native arenas.
    ///
    /// Include/eval execution moves symbol packages between separately owned
    /// request contexts. Code handles and immutable unit metadata survive
    /// that move, but every runtime-view pointer and prepared slot belongs to
    /// the old owner's arenas and must be rebuilt before native entry.
    fn reset_runtime_publication(&mut self) {
        self.runtime_state = NativeUnitRuntimeState::for_compiled(&self.compiled);
        self.linked_functions
            .fill(php_jit::JitNativeLinkedFunction::default());
        *self.published_runtime_view = Default::default();
    }
}

/// Request-owned, unit-scoped native publication state.
///
/// The old dynamic-unit activation rebuilt these tables into temporary
/// vectors and discarded them on every cross-unit call. Besides repeating
/// publication work, that made it impossible to expose a stable native view
/// for a linked compiled callee. Keeping the allocations with the unit makes
/// their addresses stable while ownership moves between the inactive package
/// and the active execution context.
#[derive(Default)]
struct NativeUnitRuntimeState {
    prepared_native_metadata_functions: std::collections::BTreeSet<php_ir::FunctionId>,
    trusted_request_local_function_offsets: Vec<u32>,
    trusted_request_local_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeRequestLocalSlot>,
    trusted_property_function_offsets: Vec<u32>,
    trusted_property_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedPropertySlot>,
    trusted_closure_plans: php_runtime::api::StableNativeArena<u64>,
    trusted_exception_plans: php_runtime::api::StableNativeArena<u64>,
    trusted_exception_plan_owners:
        std::collections::BTreeMap<usize, Box<PreparedNativeThrowableSite>>,
    trusted_constant_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedConstantSlot>,
    trusted_dynamic_constant_sites: std::collections::BTreeMap<String, Vec<usize>>,
    trusted_global_reference_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedGlobalReferenceSlot>,
    trusted_global_reference_names: std::collections::BTreeMap<usize, Box<str>>,
    trusted_static_local_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedStaticLocalSlot>,
    trusted_static_property_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedStaticPropertySlot>,
    trusted_instanceof_plans: php_runtime::api::StableNativeArena<php_jit::JitNativeInstanceOfPlan>,
    trusted_instanceof_entries: Vec<php_jit::JitNativeInstanceOfEntry>,
    trusted_exception_route_plans:
        php_runtime::api::StableNativeArena<php_jit::JitNativeExceptionRoutePlan>,
    trusted_exception_route_entries: Vec<php_jit::JitNativeExceptionRouteEntry>,
    trusted_exception_route_symbol_epoch: u64,
    trusted_class_plans: Vec<php_jit::JitNativePreparedClassPlan>,
}

impl NativeUnitRuntimeState {
    fn for_compiled(compiled: &crate::compiled_unit::CompiledUnit) -> Self {
        let (trusted_property_function_offsets, continuation_capacity) =
            trusted_continuation_storage(compiled.unit());
        let (trusted_request_local_function_offsets, trusted_request_local_slots) =
            trusted_request_local_storage(compiled.unit());
        Self {
            prepared_native_metadata_functions: std::collections::BTreeSet::new(),
            trusted_request_local_function_offsets,
            trusted_request_local_slots,
            trusted_property_function_offsets,
            trusted_property_slots: php_runtime::api::StableNativeArena::new(continuation_capacity),
            trusted_closure_plans: php_runtime::api::StableNativeArena::new(continuation_capacity),
            trusted_exception_plans: php_runtime::api::StableNativeArena::new(
                continuation_capacity,
            ),
            trusted_exception_plan_owners: std::collections::BTreeMap::new(),
            trusted_constant_slots: php_runtime::api::StableNativeArena::new(continuation_capacity),
            trusted_dynamic_constant_sites: std::collections::BTreeMap::new(),
            trusted_global_reference_slots: php_runtime::api::StableNativeArena::new(
                continuation_capacity,
            ),
            trusted_global_reference_names: std::collections::BTreeMap::new(),
            trusted_static_local_slots: php_runtime::api::StableNativeArena::new(
                continuation_capacity,
            ),
            trusted_static_property_slots: php_runtime::api::StableNativeArena::new(
                continuation_capacity,
            ),
            trusted_instanceof_plans: php_runtime::api::StableNativeArena::new(
                continuation_capacity,
            ),
            trusted_instanceof_entries: Vec::new(),
            trusted_exception_route_plans: php_runtime::api::StableNativeArena::new(
                continuation_capacity,
            ),
            trusted_exception_route_entries: Vec::new(),
            trusted_exception_route_symbol_epoch: 0,
            trusted_class_plans: Vec::new(),
        }
    }
}

fn native_active_class_handle(
    context: &NativeRequestColdState<'_>,
    name: &str,
) -> Option<crate::compiled_unit::CompiledClass> {
    context.current_dynamic_unit.map_or_else(
        || context.compiled.lookup_unit_class_handle(name),
        |unit| {
            context
                .dynamic_units
                .get(unit)?
                .compiled
                .lookup_unit_class_handle(name)
        },
    )
}

#[derive(Clone, Copy)]
struct ActiveNativeUnit(*const php_ir::IrUnit);

impl ActiveNativeUnit {
    fn new(compiled: &crate::compiled_unit::CompiledUnit) -> Self {
        Self(compiled.unit() as *const php_ir::IrUnit)
    }
}

// SAFETY: The pointed-to IR is owned by `NativeRequestColdState::compiled` or
// by one of its `dynamic_units`. Scoped unit switches retain the prior and new
// `CompiledUnit` handles until after this pointer is restored.
#[allow(unsafe_code)]
impl std::ops::Deref for ActiveNativeUnit {
    type Target = php_ir::IrUnit;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Established by `ActiveNativeUnit::new` and the context
        // ownership invariant documented on this implementation.
        unsafe { &*self.0 }
    }
}

#[derive(Clone, Copy)]
struct NativeInstructionPtr(*const php_ir::Instruction);

// SAFETY: Continuation instructions are owned by the active immutable
// CompiledUnit (or its immutable IR unit fallback). Both outlive every
// synchronous native helper invocation that receives this pointer.
#[allow(unsafe_code)]
impl std::ops::Deref for NativeInstructionPtr {
    type Target = php_ir::Instruction;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

#[derive(Clone, Copy)]
pub(super) struct NativeFunctionMetadataPtr(
    *const crate::compiled_unit::PreparedNativeFunctionMetadata,
);

impl NativeFunctionMetadataPtr {
    fn from_compiled(
        compiled: &crate::compiled_unit::CompiledUnit,
        function: php_ir::FunctionId,
    ) -> Option<Self> {
        compiled
            .prepared_native_function_metadata_ptr(function)
            .map(Self)
    }
}

// SAFETY: Prepared function metadata is immutable and owned by the active
// CompiledUnit. NativeRequestColdState retains that unit (including dynamic
// units) for the lifetime of every synchronous native frame using this view.
#[allow(unsafe_code)]
impl std::ops::Deref for NativeFunctionMetadataPtr {
    type Target = crate::compiled_unit::PreparedNativeFunctionMetadata;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

type RuntimeClassCache =
    RefCell<std::collections::HashMap<(Option<usize>, String), Rc<PreparedNativeRuntimeClass>>>;
type NativeClassConstantCache = std::collections::HashMap<
    (Option<usize>, u32),
    std::collections::HashMap<String, std::collections::HashMap<String, i64>>,
>;

pub(super) struct NativeRequestColdState<'a> {
    compiled: crate::compiled_unit::CompiledUnit,
    unit: ActiveNativeUnit,
    unit_identity: u64,
    options: &'a super::VmOptions,
    worker_state: &'a super::VmWorkerState,
    /// Stable owner-published fast-state address used only when cold code
    /// re-enters a native artifact. Direct generated operations never reach
    /// this back-pointer through the cold state.
    fast_state: *mut NativeRequestFastState,
    native_entries:
        std::sync::Arc<std::collections::BTreeMap<php_ir::FunctionId, php_jit::JitFunctionHandle>>,
    native_call_encoded_scratch: Vec<i64>,
    native_frame_arena: NativeFrameArena,
    /// One optimizing StoreLocal continuation has transferred its source
    /// owner into the baseline replay frame. The replaying store consumes
    /// that owner exactly once instead of retaining a second hidden copy.
    baseline_transition_store_owner_pending: bool,
    /// Demand-backed native continuation stack used only when a compiled
    /// caller observes `SUSPEND_FIBER`. Generated code writes these records
    /// through the fast-state view; cold code consumes them exactly once when
    /// it installs the suspended Fiber execution tree.
    fiber_suspension_states: php_runtime::api::StableNativeArena<php_jit::JitDeoptState>,
    fiber_suspension_next: Box<u32>,
    // Each scope address is published through the fast-state capability and
    // must remain stable when the outer vector grows during nested execution.
    #[allow(clippy::vec_box)]
    native_execution_scopes: Vec<Box<NativeExecutionScope>>,
    current_native_execution_scope: u32,
    native_method_pics: std::collections::BTreeMap<u64, NativeMethodPic>,
    pub(super) output: php_runtime::api::OutputBuffer,
    direct_value_slots: php_runtime::api::StableNativeArena<php_jit::JitNativeValueSlot>,
    direct_value_next: Box<u32>,
    direct_object_owners: php_runtime::api::StableNativeArena<u64>,
    direct_array_states: php_runtime::api::StableNativeArena<php_jit::JitNativeDirectArrayState>,
    direct_array_entries: php_runtime::api::StableNativeArena<php_jit::JitNativeDirectArrayEntry>,
    direct_array_next: Box<u32>,
    direct_value_free_head: Box<u32>,
    direct_value_reused_bytes: Box<u64>,
    direct_array_free_heads: Box<[u32; php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_BUCKETS]>,
    direct_array_reused_bytes: Box<u64>,
    direct_string_bytes: php_runtime::api::StableNativeArena<u8>,
    direct_string_next: Box<u32>,
    direct_string_free_heads: Box<[u32; php_jit::JIT_NATIVE_DIRECT_STRING_FREE_BUCKETS]>,
    direct_string_reused_bytes: Box<u64>,
    /// Authoritative storage for exact static properties admitted at request
    /// publication. Dynamic/autoloaded declarations remain in the cold map.
    static_property_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeStaticPropertySlot>,
    static_property_next: Box<u32>,
    static_property_indices: std::collections::BTreeMap<(String, String), u32>,
    /// Request-owned authoritative handles for PHP globals. The Rust
    /// `ReferenceCell` remains the cold identity sidecar; ordinary reads and
    /// writes use the direct reference payload without rebuilding its graph.
    native_global_reference_handles: std::collections::BTreeMap<String, i64>,
    /// Request-wide identity for direct resource capabilities. The stable
    /// `ResourceRef` owner lives in the direct value slot's `aux` pointer.
    direct_resource_handles: std::collections::HashMap<u64, u32>,
    /// Request-wide identity for authoritative direct closure records. The
    /// record itself is owned by the direct value slot's `aux` pointer.
    direct_closure_handles: std::collections::HashMap<u64, u32>,
    /// Native-only content index for immutable strings entering from a cold
    /// PHP value boundary. Buckets contain no copied string or owning
    /// reference: equality is checked against authoritative arena bytes and
    /// final slot release removes the numeric index.
    direct_string_interned_slots: std::collections::HashMap<u64, Vec<u32>>,
    /// Direct graphs whose unit-indexed literals already use request-wide
    /// native encodings. Mutations conservatively invalidate this set.
    cross_unit_stable_values: std::collections::HashSet<usize>,
    native_poll_counter: Box<u32>,
    native_root_mutation_pending: Box<u32>,
    /// Explicit cold compatibility plane. Optimizing and exact execution
    /// cannot reach any of the Rust `Value` graphs stored behind this field.
    baseline_values: BaselineValueState,
    /// Long-lived callable roots live exclusively as authoritative native
    /// owners. The baseline value plane is used only to transfer them across
    /// a separately owned include/eval request arena.
    registered_callbacks: NativeRegisteredCallbackState,
    runtime_class_cache: RuntimeClassCache,
    /// Immutable object layout identities keyed by their declaring unit and
    /// normalized class name. `instanceof` publication consumes these ids
    /// without rebuilding complete runtime class records on every unit
    /// activation.
    runtime_class_layout_cache: RefCell<std::collections::HashMap<(Option<usize>, String), u64>>,
    /// Dense, active-unit class allocation plans published once before native
    /// execution. Generated code indexes this table by immutable ClassId.
    trusted_class_plans: Vec<php_jit::JitNativePreparedClassPlan>,
    /// Long-lived request roots (globals, statics, callbacks, sessions, and
    /// suspended state). This index must not be invalidated by every call.
    root_index: RequestRootIndex,
    resources: php_runtime::api::ResourceTable,
    builtin_request_state: php_runtime::api::BuiltinRequestState,
    registered_extensions: NativeRegisteredExtensionRequestState,
    native_stream_context: NativeStreamContextState,
    pub(super) http_response: php_runtime::api::RuntimeHttpResponseState,
    pub(super) upload_registry: php_runtime::api::UploadRegistry,
    pub(super) session: php_runtime::api::SessionState,
    ini_registry: php_runtime::api::IniRegistry,
    default_timezone: String,
    mysql_state: std::rc::Rc<RefCell<php_runtime::api::MysqlState>>,
    /// Constants created by admitted exact `define()` calls. Values remain
    /// authoritative native encodings until a cold include/final boundary.
    native_dynamic_constants: std::collections::BTreeMap<String, i64>,
    /// Publication-resolved `FetchConst` slot indices keyed by the exact
    /// dynamic constant name. Exact `define()` updates this plan directly and
    /// never rescans functions or continuation instructions.
    trusted_dynamic_constant_sites: std::collections::BTreeMap<String, Vec<usize>>,
    visible_function_names: Rc<NativeFunctionNameScope>,
    dynamic_functions: std::collections::BTreeMap<String, php_ir::FunctionId>,
    deployment_functions:
        std::sync::Arc<std::collections::HashMap<std::sync::Arc<str>, php_ir::FunctionId>>,
    deployment_classes: std::sync::Arc<std::collections::HashSet<std::sync::Arc<str>>>,
    external_functions: std::collections::HashMap<String, NativeDynamicFunction>,
    external_class_units: std::collections::HashMap<String, usize>,
    /// Monotonic identity of the visible cross-unit declaration set.
    ///
    /// A newly visible signature can replace a late-link placeholder with
    /// typed, named/default, variadic, reference, generator, or attribute
    /// semantics. Affected callers are republished at the declaration
    /// boundary; unrelated entries merely advance to this epoch when their
    /// unchanged signature hash is next checked.
    external_signature_epoch: u64,
    dynamic_units: Vec<NativeDynamicUnit>,
    current_dynamic_unit: Option<usize>,
    /// PHP type guards attached to references held by static properties.
    /// Reference identity is stable across calls and include transfer, so
    /// every write observes the property constraint without re-resolving a
    /// class or property at the assignment site.
    typed_static_reference_constraints:
        std::collections::BTreeMap<u64, Vec<NativeTypedStaticReferenceConstraint>>,
    class_constant_cache: NativeClassConstantCache,
    baseline_generator_iterators: std::collections::BTreeMap<u64, i64>,
    fiber_executions: std::collections::BTreeMap<u64, NativeFiberExecution>,
    active_fiber: Option<u64>,
    pending_fiber_suspension_value: Option<i64>,
    completed_nested_fiber_call: Option<(u32, u32, php_jit::JitCallStatus, i64)>,
    called_classes: Vec<Arc<str>>,
    lexical_scope_classes: Vec<String>,
    call_frames: Vec<NativeBacktraceFrame>,
    dynamic_classes: std::collections::BTreeSet<String>,
    class_aliases: std::collections::BTreeMap<String, String>,
    /// Native object destruction work discovered at request shutdown.
    ///
    /// The queue is populated once in direct-slot order and extended only
    /// when a destructor publishes a genuinely new object. This preserves
    /// the existing reverse-order shutdown behavior without rescanning the
    /// complete direct value arena once per object.
    shutdown_destructor_queue: Option<Vec<WeakObjectHandle>>,
    destroyed_objects: std::collections::BTreeMap<u64, WeakObjectHandle>,
    autoload_in_progress: std::collections::BTreeSet<String>,
    error_reporting: i64,
    display_errors: bool,
    last_error: Option<NativeLastError>,
    explicit_reference_ids: std::collections::BTreeSet<u64>,
    environment: std::sync::Arc<Vec<(String, String)>>,
    included_files: std::collections::BTreeSet<std::path::PathBuf>,
    include_path: Arc<Vec<std::path::PathBuf>>,
    cwd: std::path::PathBuf,
    /// Stable request owner loaded directly for the special `$GLOBALS`
    /// local in optimizing functions.
    trusted_globals_proxy: i64,
    /// Authoritative numeric lvalue slots for top-level/include locals and
    /// superglobals, indexed by immutable `(FunctionId, LocalId)` offsets.
    trusted_request_local_function_offsets: Vec<u32>,
    trusted_request_local_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeRequestLocalSlot>,
    trusted_property_function_offsets: Vec<u32>,
    trusted_property_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedPropertySlot>,
    /// Opaque immutable exact closure plans, flattened with the same
    /// function/continuation offsets as trusted property and lvalue plans.
    trusted_closure_plans: php_runtime::api::StableNativeArena<u64>,
    /// Opaque immutable exact throwable plans and their stable request/unit
    /// owners, indexed through the same continuation offsets.
    trusted_exception_plans: php_runtime::api::StableNativeArena<u64>,
    trusted_exception_plan_owners:
        std::collections::BTreeMap<usize, Box<PreparedNativeThrowableSite>>,
    /// Exact global constants resolved by their cold continuation once. This
    /// parallel continuation table owns one encoded value per published slot.
    trusted_constant_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedConstantSlot>,
    /// Request-owned native literal encodings, keyed by immutable compiled
    /// unit identity. These slots remain in this request arena when include
    /// symbol metadata moves through a nested VM.
    trusted_literal_slots:
        std::collections::BTreeMap<u64, Box<[php_jit::JitNativeTrustedLiteralSlot]>>,
    trusted_global_reference_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedGlobalReferenceSlot>,
    trusted_global_reference_names: std::collections::BTreeMap<usize, Box<str>>,
    trusted_static_local_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedStaticLocalSlot>,
    trusted_static_property_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeTrustedStaticPropertySlot>,
    /// Dense static `instanceof` plans indexed by the existing continuation
    /// offsets. Their immutable entries are rebuilt only when the active unit
    /// changes; generated code performs an exact layout-id lookup.
    trusted_instanceof_plans: php_runtime::api::StableNativeArena<php_jit::JitNativeInstanceOfPlan>,
    trusted_instanceof_entries: Vec<php_jit::JitNativeInstanceOfEntry>,
    /// Immutable direct-call unwind decisions. These use the same dense
    /// function/continuation offsets and map throwable layouts to generated
    /// catch/finally resume entries without consulting Rust on the hot edge.
    trusted_exception_route_plans:
        php_runtime::api::StableNativeArena<php_jit::JitNativeExceptionRoutePlan>,
    trusted_exception_route_entries: Vec<php_jit::JitNativeExceptionRouteEntry>,
    /// Visible declaration epoch used to build the active unit's immutable
    /// catch-layout tables. Native function tier replacement leaves source
    /// continuation and handler IDs unchanged and does not invalidate it.
    trusted_exception_route_symbol_epoch: u64,
    /// Temporary publication scope. Ordinary runtime consumers see every
    /// published function; the metadata boundary narrows its preparers to
    /// functions not already materialized for the active unit.
    native_metadata_preparation_scope: Option<Vec<php_ir::FunctionId>>,
    prepared_native_metadata_functions: std::collections::BTreeSet<php_ir::FunctionId>,
    include_child: bool,
    execution_deadline_at: Option<std::time::Instant>,
    execution_deadline_mutable: bool,
    runtime_telemetry: Rc<RefCell<NativeRuntimeTelemetry>>,
    pub(super) diagnostic: Option<php_runtime::api::RuntimeDiagnostic>,
}

/// Request lifetime owner. Fast and cold state are separately allocated so
/// generated code can retain the compact ABI pointer without pointing at a
/// facade whose first operation recovers the complete Rust coordinator.
pub(super) struct NativeRequestOwner<'a> {
    cold: Box<NativeRequestColdState<'a>>,
    _fast: Box<NativeRequestFastState>,
}

impl NativeSymbolQueryCapability {
    fn published(context: &NativeRequestColdState<'_>) -> Self {
        Self {
            active_compiled: std::ptr::from_ref(&context.compiled),
            current_dynamic_unit: std::ptr::from_ref(&context.current_dynamic_unit),
            dynamic_units: std::ptr::from_ref(&context.dynamic_units),
            dynamic_functions: std::ptr::from_ref(&context.dynamic_functions),
            external_functions: std::ptr::from_ref(&context.external_functions),
            external_class_units: std::ptr::from_ref(&context.external_class_units),
            deployment_functions: std::ptr::from_ref(&context.deployment_functions),
            deployment_classes: std::ptr::from_ref(&context.deployment_classes),
            visible_function_names: std::ptr::from_ref(&context.visible_function_names),
            native_dynamic_constants: std::ptr::from_ref(&context.native_dynamic_constants)
                as *mut std::collections::BTreeMap<String, i64>,
            trusted_dynamic_constant_sites: std::ptr::from_ref(
                &context.trusted_dynamic_constant_sites,
            ),
            dynamic_classes: std::ptr::from_ref(&context.dynamic_classes),
            class_aliases: std::ptr::from_ref(&context.class_aliases),
        }
    }

    #[allow(unsafe_code)]
    fn active_compiled(&self) -> Option<&crate::compiled_unit::CompiledUnit> {
        unsafe { self.active_compiled.as_ref() }
    }

    #[allow(unsafe_code)]
    fn current_dynamic_unit(&self) -> Option<usize> {
        unsafe { self.current_dynamic_unit.as_ref() }
            .copied()
            .flatten()
    }

    #[allow(unsafe_code)]
    fn dynamic_units(&self) -> Option<&[NativeDynamicUnit]> {
        unsafe { self.dynamic_units.as_ref() }.map(Vec::as_slice)
    }

    #[allow(unsafe_code)]
    fn class_is_visible(&self, normalized: &str) -> bool {
        unsafe { self.deployment_classes.as_ref() }
            .is_some_and(|classes| classes.as_ref().contains(normalized))
            || unsafe { self.dynamic_classes.as_ref() }
                .is_some_and(|classes| classes.contains(normalized))
    }

    #[allow(unsafe_code)]
    fn external_class_handle(&self, name: &str) -> Option<crate::compiled_unit::CompiledClass> {
        let requested = normalized_class_name(name);
        let normalized = unsafe { self.class_aliases.as_ref() }
            .and_then(|aliases| aliases.get(requested.as_ref()))
            .map_or(requested.as_ref(), String::as_str);
        let unit = unsafe { self.external_class_units.as_ref() }
            .and_then(|classes| classes.get(normalized).copied())
            .or_else(|| {
                unsafe { self.deployment_classes.as_ref() }
                    .is_some_and(|classes| classes.as_ref().contains(normalized))
                    .then_some(0)
            })?;
        if self.current_dynamic_unit() == Some(unit) {
            return None;
        }
        self.dynamic_units()?
            .get(unit)?
            .compiled
            .lookup_unit_class_handle(normalized)
    }

    fn class_handle(&self, name: &str) -> Option<crate::compiled_unit::CompiledClass> {
        let normalized = normalize_class_name(name);
        self.active_compiled()?
            .lookup_unit_class_handle(&normalized)
            .or_else(|| self.external_class_handle(&normalized))
    }

    fn caller_class(&self, function: u32) -> Option<String> {
        self.active_compiled()?
            .unit()
            .classes
            .iter()
            .find(|class| {
                class
                    .methods
                    .iter()
                    .any(|method| method.function.raw() == function)
            })
            .map(|class| class.name.clone())
    }

    fn class_lineage_any(
        &self,
        name: &str,
        predicate: &mut impl FnMut(&crate::compiled_unit::CompiledClass) -> bool,
    ) -> bool {
        fn visit(
            symbols: &NativeSymbolQueryCapability,
            name: &str,
            depth: usize,
            predicate: &mut impl FnMut(&crate::compiled_unit::CompiledClass) -> bool,
        ) -> bool {
            if depth >= php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY {
                return false;
            }
            let Some(class) = symbols.class_handle(name) else {
                return false;
            };
            if predicate(&class) {
                return true;
            }
            class
                .parent
                .as_deref()
                .is_some_and(|parent| visit(symbols, parent, depth + 1, predicate))
        }
        visit(self, name, 0, predicate)
    }

    /// Resolves an exact class/interface ancestry query from the published
    /// unit, deployment, and internal-class metadata. `None` means some
    /// ancestry node is not represented by this capability and must take the
    /// instruction's single baseline continuation.
    #[allow(unsafe_code)]
    fn class_is_a(&self, class_name: &str, target: &str) -> Option<bool> {
        fn visit(
            symbols: &NativeSymbolQueryCapability,
            candidate: &str,
            target: &str,
            depth: usize,
        ) -> Option<bool> {
            if depth >= php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY {
                return None;
            }
            let candidate = normalize_class_name(candidate);
            if candidate == target {
                return Some(true);
            }
            if candidate == "arrayiterator" && matches!(target, "iterator" | "traversable") {
                return Some(true);
            }
            if let Some(class) = symbols.class_handle(&candidate) {
                if let Some(parent) = class.parent.as_deref()
                    && visit(symbols, parent, target, depth + 1)?
                {
                    return Some(true);
                }
                for interface in &class.interfaces {
                    if visit(symbols, interface, target, depth + 1)? {
                        return Some(true);
                    }
                }
                return Some(false);
            }
            if let Some(class) =
                php_std::ExtensionRegistry::standard_library().enabled_class(&candidate)
                && let Some(metadata) = class.source_metadata()
            {
                if let Some(parent) = metadata.parent
                    && visit(symbols, parent, target, depth + 1)?
                {
                    return Some(true);
                }
                for interface in metadata.interfaces {
                    if visit(symbols, interface, target, depth + 1)? {
                        return Some(true);
                    }
                }
                return Some(false);
            }
            None
        }

        let target = normalize_class_name(target);
        let target = unsafe { self.class_aliases.as_ref() }
            .and_then(|aliases| aliases.get(&target))
            .map_or(target.as_str(), String::as_str)
            .to_owned();
        visit(self, class_name, &target, 0)
    }

    #[allow(unsafe_code)]
    fn constant_exists(&self, name: &str) -> bool {
        unsafe { self.native_dynamic_constants.as_ref() }
            .is_some_and(|values| values.contains_key(name))
            || self.active_compiled().is_some_and(|compiled| {
                compiled
                    .unit()
                    .constant_table
                    .iter()
                    .any(|constant| constant.name == name)
            })
            || native_internal_class_constant_exists(name)
            || php_std::ExtensionRegistry::standard_library()
                .enabled_constant(name)
                .and_then(php_std::ConstantDescriptor::value)
                .is_some()
    }

    #[allow(unsafe_code)]
    fn native_constants(&self) -> Option<&std::collections::BTreeMap<String, i64>> {
        unsafe { self.native_dynamic_constants.as_ref() }
    }

    #[allow(unsafe_code)]
    fn dynamic_constant_sites(&self, name: &str) -> (*const usize, usize) {
        let sites: &[usize] = unsafe { self.trusted_dynamic_constant_sites.as_ref() }
            .and_then(|sites| sites.get(name))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        (sites.as_ptr(), sites.len())
    }

    #[allow(unsafe_code)]
    fn function_exists(&self, name: &str) -> bool {
        let normalized = name.to_ascii_lowercase();
        let active = self.active_compiled().is_some_and(|compiled| {
            compiled
                .unit()
                .function_table
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(name))
        });
        let dynamic = unsafe { self.dynamic_functions.as_ref() }.is_some_and(|functions| {
            functions.contains_key(name) || functions.contains_key(&normalized)
        });
        let external = unsafe { self.external_functions.as_ref() }.is_some_and(|functions| {
            functions.contains_key(name) || functions.contains_key(&normalized)
        });
        let deployment = unsafe { self.deployment_functions.as_ref() }
            .is_some_and(|functions| functions.as_ref().contains_key(normalized.as_str()));
        let visible = unsafe { self.visible_function_names.as_ref() }
            .is_some_and(|functions| functions.contains(&normalized));
        active
            || dynamic
            || external
            || deployment
            || visible
            || native_php_function_exists(&normalized)
    }

    fn same_unit_callable_plan(&self, name: &str) -> Option<NativeFixedCallablePlan> {
        let compiled = self.active_compiled()?;
        let normalized = name.trim_start_matches('\\').to_ascii_lowercase();
        let function = compiled.lookup_function(&normalized).or_else(|| {
            normalized
                .rsplit_once('\\')
                .and_then(|(_, basename)| compiled.lookup_function(basename))
        })?;
        native_fixed_callable_plan(compiled, function, false)
    }

    /// Resolve one public method against the immutable same-unit hierarchy.
    ///
    /// Callable publication is the semantic boundary: the exact method
    /// identity, staticness and fixed by-value signature are recorded once.
    /// Dynamic classes, inaccessible methods, magic dispatch, and
    /// late-static-scope-sensitive bodies remain on the single baseline
    /// continuation.
    fn same_unit_method_callable_plan(
        &self,
        class_name: &str,
        method_name: &str,
        object_target: bool,
    ) -> Option<NativeFixedCallablePlan> {
        let compiled = self.active_compiled()?;
        let mut candidate = normalize_class_name(class_name);
        loop {
            let class = compiled
                .unit()
                .classes
                .iter()
                .find(|class| class.name == candidate)?;
            if let Some(method) = class
                .methods
                .iter()
                .find(|method| method.name.eq_ignore_ascii_case(method_name))
            {
                if method.flags.is_abstract
                    || method.flags.is_private
                    || method.flags.is_protected
                    || (!object_target && !method.flags.is_static)
                {
                    return None;
                }
                let function = compiled.unit().functions.get(method.function.index())?;
                if native_function_requires_non_reference_trampoline(function, true) {
                    return None;
                }
                let has_receiver = !method.flags.is_static;
                let plan = native_fixed_callable_plan(compiled, method.function, has_receiver)?;
                if usize::from(has_receiver).saturating_add(plan.visible_arity as usize)
                    > u8::MAX as usize
                {
                    return None;
                }
                return Some(plan);
            }
            candidate = normalize_class_name(class.parent.as_ref()?);
        }
    }

    /// Decides callable visibility from published immutable class metadata.
    ///
    /// Public concrete methods and public magic dispatch are representation
    /// complete here. Visibility-sensitive, abstract, or unpublished class
    /// shapes return `None` so the callsite takes its single baseline
    /// continuation before producing an observable result.
    fn method_is_callable(
        &self,
        class_name: &str,
        method_name: &str,
        object_target: bool,
    ) -> Option<bool> {
        let mut candidate = normalize_class_name(class_name);
        for _ in 0..php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY {
            let class = self.class_handle(&candidate)?;
            if let Some(method) = class
                .methods
                .iter()
                .find(|method| method.name.eq_ignore_ascii_case(method_name))
            {
                if method.flags.is_abstract || method.flags.is_private || method.flags.is_protected
                {
                    return None;
                }
                if !object_target && !method.flags.is_static {
                    return None;
                }
                return Some(true);
            }
            let magic_name = if object_target {
                "__call"
            } else {
                "__callStatic"
            };
            if let Some(magic) = class
                .methods
                .iter()
                .find(|method| method.name.eq_ignore_ascii_case(magic_name))
            {
                if magic.flags.is_abstract
                    || magic.flags.is_private
                    || magic.flags.is_protected
                    || (!object_target && !magic.flags.is_static)
                {
                    return None;
                }
                return Some(true);
            }
            let Some(parent) = class.parent.as_deref() else {
                return Some(false);
            };
            candidate = normalize_class_name(parent);
        }
        None
    }
}

pub(crate) fn native_fixed_callable_plan(
    compiled: &crate::compiled_unit::CompiledUnit,
    function_id: php_ir::FunctionId,
    has_receiver: bool,
) -> Option<NativeFixedCallablePlan> {
    let function = compiled.unit().functions.get(function_id.index())?;
    let requires_argument_trace = function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            matches!(
                &instruction.kind,
                php_ir::InstructionKind::CallFunction { name, .. }
                    if matches!(
                        name.trim_start_matches('\\').to_ascii_lowercase().as_str(),
                        "func_get_arg" | "func_get_args" | "func_num_args"
                    )
            )
        })
    });
    let first_parameter_by_reference = function
        .params
        .first()
        .is_some_and(|parameter| parameter.by_ref);
    let supported_parameters = function
        .params
        .iter()
        .enumerate()
        .all(|(index, parameter)| !parameter.variadic && (!parameter.by_ref || index == 0));
    let admitted = !function.flags.is_generator
        && !function.returns_by_ref
        && !requires_argument_trace
        && function.params.len() <= u8::MAX as usize
        && supported_parameters;
    let visible_arity = u32::try_from(function.params.len()).ok()?;
    admitted.then(|| NativeFixedCallablePlan {
        function: function_id,
        visible_arity,
        has_receiver,
        first_parameter_by_reference,
        returns_int: matches!(
            function.return_type.as_ref(),
            Some(php_ir::IrReturnType::Int)
        ),
        returns_string: matches!(
            function.return_type.as_ref(),
            Some(php_ir::IrReturnType::String)
        ),
        returns_releasable_scalar: function
            .return_type
            .as_ref()
            .is_some_and(native_callback_return_type_is_releasable_scalar),
    })
}

fn native_callback_return_type_is_releasable_scalar(type_: &php_ir::IrReturnType) -> bool {
    use php_ir::IrReturnType as Type;
    match type_ {
        Type::Int
        | Type::Float
        | Type::String
        | Type::Bool
        | Type::Null
        | Type::False
        | Type::True
        | Type::Void
        | Type::Never => true,
        Type::Nullable { inner } => native_callback_return_type_is_releasable_scalar(inner),
        Type::Union { members } => {
            !members.is_empty()
                && members
                    .iter()
                    .all(native_callback_return_type_is_releasable_scalar)
        }
        Type::Array
        | Type::Callable
        | Type::Iterable
        | Type::Object
        | Type::Mixed
        | Type::Class { .. }
        | Type::Intersection { .. }
        | Type::Dnf { .. } => false,
    }
}

impl NativeRequestQueryCapability {
    fn published(context: &NativeRequestColdState<'_>) -> Self {
        Self {
            environment: std::ptr::from_ref(&context.environment),
            included_files: std::ptr::from_ref(&context.included_files),
            sapi_name: std::ptr::from_ref(&context.options.runtime_context.sapi_name),
        }
    }

    #[allow(unsafe_code)]
    fn environment(&self) -> Option<&[(String, String)]> {
        unsafe { self.environment.as_ref() }.map(|environment| environment.as_ref().as_slice())
    }

    #[allow(unsafe_code)]
    fn included_files(&self) -> Option<&std::collections::BTreeSet<std::path::PathBuf>> {
        unsafe { self.included_files.as_ref() }
    }

    #[allow(unsafe_code)]
    fn sapi_name(&self) -> Option<&str> {
        unsafe { self.sapi_name.as_ref() }.map(String::as_str)
    }
}

impl NativeConfigurationCapability {
    fn published(context: &NativeRequestColdState<'_>) -> Self {
        Self {
            ini_registry: std::ptr::from_ref(&context.ini_registry)
                as *mut php_runtime::api::IniRegistry,
            include_path: std::ptr::from_ref(&context.include_path)
                as *mut Arc<Vec<std::path::PathBuf>>,
            display_errors: std::ptr::from_ref(&context.display_errors) as *mut bool,
            default_timezone: std::ptr::from_ref(&context.default_timezone) as *mut String,
        }
    }

    /// Returns the request registry guaranteed by capability publication.
    ///
    /// Exact handlers never validate this engine invariant per invocation:
    /// `NativeRequestOwner` publishes the stable non-null owner before native
    /// execution can observe the fast state.
    #[allow(unsafe_code)]
    fn ini_registry(&self) -> &php_runtime::api::IniRegistry {
        unsafe { &*self.ini_registry }
    }

    #[allow(unsafe_code)]
    fn ini_registry_mut(&mut self) -> &mut php_runtime::api::IniRegistry {
        unsafe { &mut *self.ini_registry }
    }

    #[allow(unsafe_code)]
    fn include_path_mut(&mut self) -> &mut Arc<Vec<std::path::PathBuf>> {
        unsafe { &mut *self.include_path }
    }

    #[allow(unsafe_code)]
    fn include_path(&self) -> &Arc<Vec<std::path::PathBuf>> {
        unsafe { &*self.include_path }
    }

    #[allow(unsafe_code)]
    fn display_errors_mut(&mut self) -> &mut bool {
        unsafe { &mut *self.display_errors }
    }

    #[allow(unsafe_code)]
    fn default_timezone(&self) -> &str {
        unsafe { &*self.default_timezone }.as_str()
    }

    #[allow(unsafe_code)]
    fn default_timezone_mut(&mut self) -> &mut String {
        unsafe { &mut *self.default_timezone }
    }
}

impl NativeHttpResponseCapability {
    fn published(context: &NativeRequestColdState<'_>) -> Self {
        Self {
            response: std::ptr::from_ref(&context.http_response)
                as *mut php_runtime::api::RuntimeHttpResponseState,
        }
    }

    /// Publication guarantees the stable non-null owner; exact invocation
    /// therefore performs no repeated engine-integrity validation.
    #[allow(unsafe_code)]
    fn response(&self) -> &php_runtime::api::RuntimeHttpResponseState {
        unsafe { &*self.response }
    }

    #[allow(unsafe_code)]
    fn response_mut(&mut self) -> &mut php_runtime::api::RuntimeHttpResponseState {
        unsafe { &mut *self.response }
    }
}

impl NativeSessionCapability {
    #[allow(unsafe_code)]
    fn control(&self) -> &php_runtime::api::NativeSessionControlState {
        unsafe { &*self.control }
    }

    #[allow(unsafe_code)]
    fn control_mut(&mut self) -> &mut php_runtime::api::NativeSessionControlState {
        unsafe { &mut *self.control }
    }

    const fn has_loader(&self) -> bool {
        self.has_loader != 0
    }

    const fn has_id_generator(&self) -> bool {
        self.has_id_generator != 0
    }
}

impl<'a> NativeRequestOwner<'a> {
    pub(super) fn new(
        compiled: &'a crate::compiled_unit::CompiledUnit,
        unit_identity: u64,
        options: &'a super::VmOptions,
        worker_state: &'a super::VmWorkerState,
        output: php_runtime::api::OutputBuffer,
        native_entries: std::sync::Arc<
            std::collections::BTreeMap<php_ir::FunctionId, php_jit::JitFunctionHandle>,
        >,
    ) -> Self {
        let mut cold = Box::new(NativeRequestColdState::new(
            compiled,
            unit_identity,
            options,
            worker_state,
            output,
            native_entries,
        ));
        cold.promote_cold_dynamic_constants()
            .expect("request constants must fit the authoritative native arena");
        cold.promote_pending_registered_callbacks()
            .expect("registered callbacks must fit the authoritative native arena");
        let mut fast = Box::<NativeRequestFastState>::default();
        let fast_ptr = std::ptr::from_mut(fast.as_mut());
        cold.fast_state = fast_ptr;
        fast.output = std::ptr::from_mut(&mut cold.output);
        fast.json_state = std::ptr::from_mut(cold.builtin_request_state.json_mut());
        fast.pcre_state = std::ptr::from_mut(cold.builtin_request_state.pcre_mut());
        fast.gc_state = std::ptr::from_mut(cold.builtin_request_state.gc_mut());
        fast.cwd = std::ptr::from_mut(&mut cold.cwd);
        fast.filesystem_capabilities = std::ptr::from_ref(&cold.options.runtime_context.filesystem);
        fast.filesystem_state = cold.registered_extensions.filesystem_ptr();
        let default_stream_context = cold
            .publish_owned_direct_array_entries(Vec::new())
            .expect("default stream context must fit the native array arena");
        cold.native_stream_context.default_options = default_stream_context;
        fast.stream_context = std::ptr::from_mut(&mut cold.native_stream_context);
        fast.stdin = std::ptr::from_ref(&cold.options.runtime_context.stdin);
        fast.resources = std::ptr::from_mut(&mut cold.resources);
        fast.upload_registry = std::ptr::from_mut(&mut cold.upload_registry);
        fast.last_error = std::ptr::from_mut(&mut cold.last_error);
        fast.direct_resource_handles = std::ptr::from_mut(&mut cold.direct_resource_handles);
        fast.direct_closure_handles = std::ptr::from_mut(&mut cold.direct_closure_handles);
        fast.callback_handlers = std::ptr::from_mut(&mut cold.registered_callbacks);
        fast.callback_transient_export = u8::from(cold.include_child);
        fast.symbol_query = NativeSymbolQueryCapability::published(cold.as_ref());
        fast.configuration = NativeConfigurationCapability::published(cold.as_ref());
        fast.http_response = NativeHttpResponseCapability::published(cold.as_ref());
        fast.request_query = NativeRequestQueryCapability::published(cold.as_ref());
        fast.mbstring = NativeMbstringCapability {
            internal_encoding: cold.registered_extensions.mb_internal_encoding_ptr(),
            substitute_character: cold.registered_extensions.mb_substitute_character_ptr(),
        };
        fast.bcmath = NativeBcmathCapability {
            scale: cold.registered_extensions.bcmath_scale_ptr(),
        };
        fast.random = NativeRandomCapability {
            fill: Some(php_runtime::api::native_random_fill),
        };
        let (filter_roots, filter_present) = cold
            .publish_native_filter_input_roots()
            .expect("request filter inputs must fit the native value arena");
        fast.filter = NativeFilterCapability {
            roots: filter_roots,
            present: filter_present,
        };
        fast.frame_arena = NativeFrameArenaCapability::published(cold.as_mut());
        cold.trusted_globals_proxy = cold
            .encode_globals_proxy()
            .expect("request globals proxy must fit the native value arena");
        // Every request owner, including the separately owned context used
        // while executing an include/eval unit, publishes its active unit's
        // literal table before any native entry can observe the runtime view.
        cold.prepare_trusted_literal_slots();
        cold.prepare_trusted_closure_plans();
        cold.prepare_trusted_exception_plans();
        cold.prepare_trusted_constant_fetches();
        cold.prepare_trusted_request_locals();
        cold.prepare_trusted_global_references()
            .expect("trusted global references must publish before native entry");
        let session_reference = cold
            .native_global_reference_handle("_SESSION")
            .expect("session global must publish in the native plane")
            .expect("session global must have one canonical reference");
        let committed = cold
            .encode_native_array_owner(cold.session.committed_data())
            .expect("committed session payload must fit the native arena");
        fast.session = NativeSessionCapability {
            control: std::ptr::from_mut(cold.session.native_control_mut()),
            global_reference: session_reference,
            committed,
            has_loader: u8::from(cold.options.runtime_context.session_loader.is_some()),
            has_id_generator: u8::from(cold.options.runtime_context.session_id_generator.is_some()),
        };
        cold.prepare_trusted_static_locals();
        cold.prepare_trusted_static_properties();
        cold.prepare_trusted_class_plans();
        cold.prepare_trusted_declared_properties();
        cold.prepare_trusted_instanceof_plans();
        cold.prepare_trusted_exception_routes();
        if cold.include_child {
            cold.republish_transferred_dynamic_units()
                .expect("transferred native units must publish before include execution");
        }
        Self { cold, _fast: fast }
    }
}

impl<'a> std::ops::Deref for NativeRequestOwner<'a> {
    type Target = NativeRequestColdState<'a>;

    fn deref(&self) -> &Self::Target {
        self.cold.as_ref()
    }
}

impl<'a> std::ops::DerefMut for NativeRequestOwner<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cold.as_mut()
    }
}

/// Value family observed directly from an encoded native value.  This is a
/// classification of the authoritative slot, not a second value
/// representation: it owns no payload and cannot outlive the query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeEncodedValueKind {
    Null,
    Uninitialized,
    Bool(bool),
    Int,
    Float,
    String,
    Array,
    Object,
    Callable,
    Resource,
    Generator,
    Fiber,
    Reference,
}

#[derive(Clone, Copy)]
enum NativeComparisonValue<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(&'a [u8]),
    Array {
        identity: usize,
        entries: &'a [php_jit::JitNativeDirectArrayEntry],
    },
    Object(NativeComparisonObject<'a>),
    OpaqueIdentity(u64),
    Resource(u64),
}

#[derive(Clone, Copy)]
struct NativeComparisonObject<'a> {
    identity: u64,
    layout_id: Option<u64>,
    owner: &'a php_runtime::api::ObjectRef,
}

#[derive(Default)]
struct NativeComparisonTraversal {
    arrays: Vec<(usize, usize)>,
    objects: Vec<(u64, u64)>,
    unordered: bool,
}

#[derive(Clone, Copy)]
enum NativeComparisonNumber {
    Int(i64),
    Float(f64),
}

impl NativeComparisonNumber {
    fn as_f64(self) -> f64 {
        match self {
            Self::Int(value) => value as f64,
            Self::Float(value) => value,
        }
    }
}

fn native_comparison_truthy(value: NativeComparisonValue<'_>) -> bool {
    match value {
        NativeComparisonValue::Null | NativeComparisonValue::Bool(false) => false,
        NativeComparisonValue::Bool(true) => true,
        NativeComparisonValue::Int(value) => value != 0,
        NativeComparisonValue::Float(value) => value != 0.0,
        NativeComparisonValue::String(value) => !value.is_empty() && value != b"0",
        NativeComparisonValue::Array { entries, .. } => !entries.is_empty(),
        NativeComparisonValue::Object(_)
        | NativeComparisonValue::OpaqueIdentity(_)
        | NativeComparisonValue::Resource(_) => true,
    }
}

fn native_comparison_numeric_string(bytes: &[u8]) -> Option<NativeComparisonNumber> {
    use php_runtime::experimental::numeric_string::{NumericStringKind, NumericStringValue};
    let classified = php_runtime::experimental::numeric_string::classify(bytes);
    match (classified.kind, classified.value) {
        (
            NumericStringKind::IntString | NumericStringKind::FloatString,
            Some(NumericStringValue::Int(value)),
        ) => Some(NativeComparisonNumber::Int(value)),
        (
            NumericStringKind::IntString | NumericStringKind::FloatString,
            Some(NumericStringValue::Float(value)),
        ) => Some(NativeComparisonNumber::Float(value)),
        _ => None,
    }
}

fn native_comparison_numbers_order(
    left: NativeComparisonNumber,
    right: NativeComparisonNumber,
) -> std::cmp::Ordering {
    if let (NativeComparisonNumber::Int(left), NativeComparisonNumber::Int(right)) = (left, right) {
        return left.cmp(&right);
    }
    let left = left.as_f64();
    let right = right.as_f64();
    if left.is_nan() || right.is_nan() {
        return std::cmp::Ordering::Greater;
    }
    left.partial_cmp(&right)
        .unwrap_or(std::cmp::Ordering::Greater)
}

fn native_comparison_number_bytes(number: NativeComparisonNumber) -> Vec<u8> {
    match number {
        NativeComparisonNumber::Int(value) => value.to_string().into_bytes(),
        NativeComparisonNumber::Float(value) => {
            php_runtime::api::float_to_php_string(value).into_bytes()
        }
    }
}

fn native_comparison_values_order(
    left: NativeComparisonValue<'_>,
    right: NativeComparisonValue<'_>,
) -> Option<std::cmp::Ordering> {
    if matches!(left, NativeComparisonValue::Bool(_))
        || matches!(right, NativeComparisonValue::Bool(_))
    {
        return Some(native_comparison_truthy(left).cmp(&native_comparison_truthy(right)));
    }
    match (left, right) {
        (NativeComparisonValue::Null, NativeComparisonValue::String(right)) => {
            return Some([].as_slice().cmp(right));
        }
        (NativeComparisonValue::String(left), NativeComparisonValue::Null) => {
            return Some(left.cmp([].as_slice()));
        }
        (NativeComparisonValue::Null, _) | (_, NativeComparisonValue::Null) => {
            return Some(native_comparison_truthy(left).cmp(&native_comparison_truthy(right)));
        }
        _ => {}
    }
    match (left, right) {
        (NativeComparisonValue::Int(left), NativeComparisonValue::Int(right)) => {
            Some(left.cmp(&right))
        }
        (NativeComparisonValue::Int(left), NativeComparisonValue::Float(right)) => {
            Some(native_comparison_numbers_order(
                NativeComparisonNumber::Int(left),
                NativeComparisonNumber::Float(right),
            ))
        }
        (NativeComparisonValue::Float(left), NativeComparisonValue::Int(right)) => {
            Some(native_comparison_numbers_order(
                NativeComparisonNumber::Float(left),
                NativeComparisonNumber::Int(right),
            ))
        }
        (NativeComparisonValue::Float(left), NativeComparisonValue::Float(right)) => {
            Some(native_comparison_numbers_order(
                NativeComparisonNumber::Float(left),
                NativeComparisonNumber::Float(right),
            ))
        }
        (NativeComparisonValue::String(left), NativeComparisonValue::String(right)) => {
            match (
                native_comparison_numeric_string(left),
                native_comparison_numeric_string(right),
            ) {
                (Some(left), Some(right)) => Some(native_comparison_numbers_order(left, right)),
                _ => Some(left.cmp(right)),
            }
        }
        (NativeComparisonValue::String(string), NativeComparisonValue::Int(number)) => {
            if let Some(string) = native_comparison_numeric_string(string) {
                Some(native_comparison_numbers_order(
                    string,
                    NativeComparisonNumber::Int(number),
                ))
            } else {
                Some(string.cmp(
                    native_comparison_number_bytes(NativeComparisonNumber::Int(number)).as_slice(),
                ))
            }
        }
        (NativeComparisonValue::String(string), NativeComparisonValue::Float(number)) => {
            if let Some(string) = native_comparison_numeric_string(string) {
                Some(native_comparison_numbers_order(
                    string,
                    NativeComparisonNumber::Float(number),
                ))
            } else {
                Some(
                    string.cmp(
                        native_comparison_number_bytes(NativeComparisonNumber::Float(number))
                            .as_slice(),
                    ),
                )
            }
        }
        (NativeComparisonValue::Int(number), NativeComparisonValue::String(string)) => {
            if let Some(string) = native_comparison_numeric_string(string) {
                Some(native_comparison_numbers_order(
                    NativeComparisonNumber::Int(number),
                    string,
                ))
            } else {
                Some(
                    native_comparison_number_bytes(NativeComparisonNumber::Int(number))
                        .as_slice()
                        .cmp(string),
                )
            }
        }
        (NativeComparisonValue::Float(number), NativeComparisonValue::String(string)) => {
            if let Some(string) = native_comparison_numeric_string(string) {
                Some(native_comparison_numbers_order(
                    NativeComparisonNumber::Float(number),
                    string,
                ))
            } else {
                Some(
                    native_comparison_number_bytes(NativeComparisonNumber::Float(number))
                        .as_slice()
                        .cmp(string),
                )
            }
        }
        _ => None,
    }
}

const fn native_reference_state(state: u32) -> u32 {
    state & !php_jit::JIT_NATIVE_REFERENCE_TYPED_PROPERTY_GUARD
}

fn native_special_value_class_is_a(kind: NativeEncodedValueKind, target: &str) -> Option<bool> {
    let target = normalize_class_name(target);
    match kind {
        NativeEncodedValueKind::Callable => Some(target == "closure"),
        NativeEncodedValueKind::Fiber => Some(target == "fiber"),
        NativeEncodedValueKind::Generator => Some(matches!(
            target.as_str(),
            "generator" | "iterator" | "traversable"
        )),
        _ => None,
    }
}

struct NativePreparedClosure {
    /// Stable generated-code view. The capture allocation is boxed before
    /// this record is published, so both pointers remain request-stable.
    native_view: php_jit::JitNativePreparedClosureView,
    /// PHP closure metadata only. `captures` and `bound_this` are always
    /// empty here; their authoritative owners are the encoded fields below.
    closure: php_runtime::api::ClosurePayload,
    capture_descriptors: Arc<[(String, bool)]>,
    implicit_this: Option<i64>,
    captures: Box<[i64]>,
    /// Published only by the exact same-unit closure allocation boundary.
    /// Baseline materialization and rebinding deliberately leave this absent.
    fixed_visible_arity: Option<u32>,
    first_parameter_by_reference: bool,
    returns_int: bool,
    returns_string: bool,
    returns_releasable_scalar: bool,
}

impl NativePreparedClosure {
    fn new(
        closure: php_runtime::api::ClosurePayload,
        capture_descriptors: Arc<[(String, bool)]>,
        implicit_this: Option<i64>,
        captures: Box<[i64]>,
        fixed_visible_arity: Option<u32>,
        first_parameter_by_reference: bool,
        returns_int: bool,
        returns_string: bool,
        returns_releasable_scalar: bool,
    ) -> Self {
        let native_view = php_jit::JitNativePreparedClosureView {
            captures: captures.as_ptr() as usize as u64,
            capture_count: u32::try_from(captures.len()).unwrap_or(u32::MAX),
            flags: u32::from(implicit_this.is_some())
                * php_jit::JIT_NATIVE_PREPARED_CLOSURE_HAS_IMPLICIT_THIS,
            implicit_this: implicit_this
                .unwrap_or_else(|| php_jit::jit_encode_constant(php_jit::JIT_VALUE_UNINITIALIZED)),
        };
        Self {
            native_view,
            closure,
            capture_descriptors,
            implicit_this,
            captures,
            fixed_visible_arity,
            first_parameter_by_reference,
            returns_int,
            returns_string,
            returns_releasable_scalar,
        }
    }
}

/// Stable allocation shared by cold callable semantics and generated closure
/// calls. The complete C-layout view is first, so a prepared callable slot's
/// `aux` pointer exposes every stable callable shape without crossing into the
/// Rust compatibility sidecar. Its first 24 bytes remain the generated
/// closure-call prefix.
#[repr(C)]
struct NativePreparedCallableOwner {
    native_view: php_jit::JitNativePreparedCallableView,
    /// Closure debug/context metadata is consulted only after an explicit
    /// baseline/cold boundary. Captures and the bound receiver remain
    /// authoritative in `native_view`.
    cold_closure: Option<NativePreparedClosure>,
    /// Stable byte owners addressed by `native_view`. These buffers carry no
    /// independent kind or dispatch semantics.
    _name_bytes: Box<[u8]>,
    _method_bytes: Box<[u8]>,
    _class_bytes: Box<[u8]>,
}

impl NativePreparedCallableOwner {
    fn from_native_parts(
        mut native_view: php_jit::JitNativePreparedCallableView,
        cold_closure: Option<NativePreparedClosure>,
        name_bytes: Box<[u8]>,
        method_bytes: Box<[u8]>,
        class_bytes: Box<[u8]>,
    ) -> Self {
        fn byte_range(value: &[u8]) -> (u64, u32) {
            let length = u32::try_from(value.len())
                .expect("published callable names are bounded by the native ABI");
            let bytes = if length == 0 {
                0
            } else {
                value.as_ptr() as usize as u64
            };
            (bytes, length)
        }
        (native_view.name_bytes, native_view.name_length) = byte_range(&name_bytes);
        (native_view.method_bytes, native_view.method_length) = byte_range(&method_bytes);
        (native_view.class_bytes, native_view.class_length) = byte_range(&class_bytes);
        Self {
            native_view,
            cold_closure,
            _name_bytes: name_bytes,
            _method_bytes: method_bytes,
            _class_bytes: class_bytes,
        }
    }

    fn install_fixed_plan(&mut self, plan: NativeFixedCallablePlan) {
        self.native_view.flags &= !(php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIXED_BINDING
            | php_jit::JIT_NATIVE_PREPARED_CALLABLE_HAS_RECEIVER
            | php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_INT
            | php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_STRING
            | php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIRST_PARAMETER_BY_REFERENCE
            | php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_RELEASABLE_SCALAR);
        self.native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIXED_BINDING;
        if plan.has_receiver {
            self.native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_HAS_RECEIVER;
        }
        if plan.first_parameter_by_reference {
            self.native_view.flags |=
                php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIRST_PARAMETER_BY_REFERENCE;
        }
        if plan.returns_int {
            self.native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_INT;
        }
        if plan.returns_string {
            self.native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_STRING;
        }
        if plan.returns_releasable_scalar {
            self.native_view.flags |=
                php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_RELEASABLE_SCALAR;
        }
        self.native_view.function_id = plan.function.raw();
        self.native_view.reserved = plan.visible_arity;
    }

    fn user_function(name: Box<[u8]>, resolved_function: Option<NativeFixedCallablePlan>) -> Self {
        let mut flags = 0;
        let mut visible_arity = 0;
        if let Some(plan) = resolved_function {
            flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIXED_BINDING;
            if plan.first_parameter_by_reference {
                flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIRST_PARAMETER_BY_REFERENCE;
            }
            if plan.returns_int {
                flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_INT;
            }
            if plan.returns_string {
                flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_STRING;
            }
            if plan.returns_releasable_scalar {
                flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_RELEASABLE_SCALAR;
            }
            visible_arity = plan.visible_arity;
        }
        Self::from_native_parts(
            php_jit::JitNativePreparedCallableView {
                kind: php_jit::JIT_NATIVE_CALLABLE_KIND_USER_FUNCTION,
                function_id: resolved_function.map_or(u32::MAX, |plan| plan.function.raw()),
                flags,
                reserved: visible_arity,
                ..php_jit::JitNativePreparedCallableView::default()
            },
            None,
            name,
            Box::default(),
            Box::default(),
        )
    }

    fn internal_builtin(name: Box<[u8]>) -> Self {
        Self::from_native_parts(
            php_jit::JitNativePreparedCallableView {
                kind: php_jit::JIT_NATIVE_CALLABLE_KIND_INTERNAL_BUILTIN,
                function_id: u32::MAX,
                ..php_jit::JitNativePreparedCallableView::default()
            },
            None,
            name,
            Box::default(),
            Box::default(),
        )
    }

    fn closure(closure: NativePreparedClosure) -> Self {
        let closure_view = closure.native_view;
        let mut flags = closure_view.flags;
        let mut visible_arity = 0;
        if let Some(arity) = closure.fixed_visible_arity {
            flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIXED_BINDING;
            visible_arity = arity;
        }
        if closure.first_parameter_by_reference {
            flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIRST_PARAMETER_BY_REFERENCE;
        }
        if closure.returns_int {
            flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_INT;
        }
        if closure.returns_string {
            flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_STRING;
        }
        if closure.returns_releasable_scalar {
            flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_RELEASABLE_SCALAR;
        }
        Self::from_native_parts(
            php_jit::JitNativePreparedCallableView {
                captures: closure_view.captures,
                capture_count: closure_view.capture_count,
                flags,
                implicit_this: closure_view.implicit_this,
                kind: php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE,
                function_id: closure.closure.function,
                reserved: visible_arity,
                ..php_jit::JitNativePreparedCallableView::default()
            },
            Some(closure),
            Box::default(),
            Box::default(),
            Box::default(),
        )
    }

    fn bound_object(
        receiver: i64,
        method: Box<[u8]>,
        scope: Option<Box<[u8]>>,
        resolved_function: Option<NativeFixedCallablePlan>,
    ) -> Self {
        let mut native_view = php_jit::JitNativePreparedCallableView {
            kind: php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD,
            receiver,
            function_id: resolved_function.map_or(u32::MAX, |plan| plan.function.raw()),
            reserved: resolved_function.map_or(0, |plan| plan.visible_arity),
            ..php_jit::JitNativePreparedCallableView::default()
        };
        if let Some(plan) = resolved_function {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIXED_BINDING;
            if plan.has_receiver {
                native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_HAS_RECEIVER;
            }
            if plan.first_parameter_by_reference {
                native_view.flags |=
                    php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIRST_PARAMETER_BY_REFERENCE;
            }
            if plan.returns_int {
                native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_INT;
            }
            if plan.returns_string {
                native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_STRING;
            }
            if plan.returns_releasable_scalar {
                native_view.flags |=
                    php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_RELEASABLE_SCALAR;
            }
        }
        if scope.is_some() {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_HAS_SCOPE;
        }
        Self::from_native_parts(
            native_view,
            None,
            scope.unwrap_or_default(),
            method,
            Box::default(),
        )
    }

    fn bound_class(
        class: Box<[u8]>,
        method: Box<[u8]>,
        scope: Option<Box<[u8]>>,
        resolved_function: Option<NativeFixedCallablePlan>,
    ) -> Self {
        let mut native_view = php_jit::JitNativePreparedCallableView {
            kind: php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_CLASS_METHOD,
            function_id: resolved_function.map_or(u32::MAX, |plan| plan.function.raw()),
            reserved: resolved_function.map_or(0, |plan| plan.visible_arity),
            ..php_jit::JitNativePreparedCallableView::default()
        };
        if resolved_function.is_some() {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIXED_BINDING;
        }
        if resolved_function.is_some_and(|plan| plan.first_parameter_by_reference) {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_FIRST_PARAMETER_BY_REFERENCE;
        }
        if resolved_function.is_some_and(|plan| plan.returns_int) {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_INT;
        }
        if resolved_function.is_some_and(|plan| plan.returns_string) {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_STRING;
        }
        if resolved_function.is_some_and(|plan| plan.returns_releasable_scalar) {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_RETURNS_RELEASABLE_SCALAR;
        }
        if scope.is_some() {
            native_view.flags |= php_jit::JIT_NATIVE_PREPARED_CALLABLE_HAS_SCOPE;
        }
        Self::from_native_parts(native_view, None, scope.unwrap_or_default(), method, class)
    }

    fn method_placeholder(target: Box<[u8]>) -> Self {
        Self::from_native_parts(
            php_jit::JitNativePreparedCallableView {
                kind: php_jit::JIT_NATIVE_CALLABLE_KIND_METHOD_PLACEHOLDER,
                function_id: u32::MAX,
                ..php_jit::JitNativePreparedCallableView::default()
            },
            None,
            target,
            Box::default(),
            Box::default(),
        )
    }

    fn unresolved_dynamic(target: Box<[u8]>) -> Self {
        Self::from_native_parts(
            php_jit::JitNativePreparedCallableView {
                kind: php_jit::JIT_NATIVE_CALLABLE_KIND_UNRESOLVED_DYNAMIC,
                function_id: u32::MAX,
                ..php_jit::JitNativePreparedCallableView::default()
            },
            None,
            target,
            Box::default(),
            Box::default(),
        )
    }
}

enum NativePreparedCallableDispatch {
    Closure,
    Named(String),
    BoundMethod {
        target: php_runtime::api::CallableMethodTarget,
        method: String,
    },
    Invalid(String),
}

struct NativeDirectFiber {
    state: php_runtime::api::FiberState,
    callable: i64,
    return_value: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeExecutionScope {
    unit: Option<usize>,
    called_class: Option<Arc<str>>,
    scope_class: Option<Arc<str>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeExecutionTarget {
    unit: Option<usize>,
    function: php_ir::FunctionId,
    called_class: Option<Arc<str>>,
    scope_class: Option<Arc<str>>,
}

impl NativeExecutionTarget {
    fn scope(&self) -> NativeExecutionScope {
        NativeExecutionScope {
            unit: self.unit,
            called_class: self.called_class.clone(),
            scope_class: self.scope_class.clone(),
        }
    }
}

struct NativeDirectGenerator {
    target: NativeExecutionTarget,
    /// These owners transfer into the generated activation on first entry.
    /// Thereafter the suspension snapshot or generated epilogue owns them.
    arguments: Vec<i64>,
    /// Suspension resume validates the original callable ABI even though the
    /// argument owners have moved into the saved native frame.
    argument_count: usize,
    handle: Option<php_jit::JitFunctionHandle>,
    state: Option<php_jit::JitDeoptState>,
    lifecycle: php_runtime::api::GeneratorState,
    current_key: Option<i64>,
    current_value: Option<i64>,
    return_value: Option<i64>,
    next_auto_key: i64,
    delegation: Option<NativeGeneratorDelegation>,
    yields_seen: u64,
}

enum NativeFiberReceiver {
    Direct(i64),
    Materialized(php_runtime::api::FiberRef),
}

/// Reusable allocations whose contents never survive a request boundary.
///
/// PHP-visible owners are released before this record is returned to the
/// worker. The pool retains only raw native arenas, frame mappings, and
/// numeric scratch capacity; it never retains values, globals, callbacks,
/// exceptions, extension state, or other request semantics.
pub(super) struct NativeRequestBuffers {
    direct_value_slots: php_runtime::api::StableNativeArena<php_jit::JitNativeValueSlot>,
    direct_value_next: Box<u32>,
    direct_object_owners: php_runtime::api::StableNativeArena<u64>,
    direct_array_states: php_runtime::api::StableNativeArena<php_jit::JitNativeDirectArrayState>,
    direct_array_entries: php_runtime::api::StableNativeArena<php_jit::JitNativeDirectArrayEntry>,
    direct_array_next: Box<u32>,
    direct_value_free_head: Box<u32>,
    direct_value_reused_bytes: Box<u64>,
    direct_array_free_heads: Box<[u32; php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_BUCKETS]>,
    direct_array_reused_bytes: Box<u64>,
    direct_string_bytes: php_runtime::api::StableNativeArena<u8>,
    direct_string_next: Box<u32>,
    direct_string_free_heads: Box<[u32; php_jit::JIT_NATIVE_DIRECT_STRING_FREE_BUCKETS]>,
    direct_string_reused_bytes: Box<u64>,
    fiber_suspension_states: php_runtime::api::StableNativeArena<php_jit::JitDeoptState>,
    fiber_suspension_next: Box<u32>,
    static_property_slots:
        php_runtime::api::StableNativeArena<php_jit::JitNativeStaticPropertySlot>,
    static_property_next: Box<u32>,
    native_call_encoded_scratch: Vec<i64>,
    native_frame_arena: NativeFrameArena,
    direct_resource_handles: std::collections::HashMap<u64, u32>,
    direct_closure_handles: std::collections::HashMap<u64, u32>,
    class_constant_cache: NativeClassConstantCache,
    diagnostic_telemetry: NativeRuntimeTelemetry,
}

impl Default for NativeRequestBuffers {
    fn default() -> Self {
        Self {
            direct_value_slots: php_runtime::api::StableNativeArena::new(
                php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY,
            ),
            direct_value_next: Box::new(0),
            direct_object_owners: php_runtime::api::StableNativeArena::new(
                php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY,
            ),
            direct_array_states: php_runtime::api::StableNativeArena::new(
                php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY,
            ),
            direct_array_entries: php_runtime::api::StableNativeArena::new(
                php_jit::JIT_NATIVE_DIRECT_ARRAY_ENTRY_CAPACITY,
            ),
            direct_array_next: Box::new(0),
            direct_value_free_head: Box::new(php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE),
            direct_value_reused_bytes: Box::new(0),
            direct_array_free_heads: Box::new(
                [php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE;
                    php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_BUCKETS],
            ),
            direct_array_reused_bytes: Box::new(0),
            direct_string_bytes: php_runtime::api::StableNativeArena::new(
                php_jit::JIT_NATIVE_DIRECT_STRING_BYTE_CAPACITY,
            ),
            direct_string_next: Box::new(0),
            direct_string_free_heads: Box::new(
                [php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE;
                    php_jit::JIT_NATIVE_DIRECT_STRING_FREE_BUCKETS],
            ),
            direct_string_reused_bytes: Box::new(0),
            fiber_suspension_states: php_runtime::api::StableNativeArena::new(
                php_jit::JIT_NATIVE_FIBER_SUSPENSION_CAPACITY,
            ),
            fiber_suspension_next: Box::new(0),
            static_property_slots: php_runtime::api::StableNativeArena::new(
                php_jit::JIT_NATIVE_STATIC_PROPERTY_CAPACITY,
            ),
            static_property_next: Box::new(0),
            native_call_encoded_scratch: Vec::new(),
            native_frame_arena: NativeFrameArena::default(),
            direct_resource_handles: std::collections::HashMap::new(),
            direct_closure_handles: std::collections::HashMap::new(),
            class_constant_cache: std::collections::HashMap::new(),
            diagnostic_telemetry: NativeRuntimeTelemetry::default(),
        }
    }
}

impl std::fmt::Debug for NativeRequestBuffers {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeRequestBuffers")
            .field("slot_capacity", &self.direct_value_slots.capacity())
            .field(
                "argument_scratch_capacity",
                &self.native_call_encoded_scratch.capacity(),
            )
            .field(
                "frame_capacity_bytes",
                &self.native_frame_arena.capacity_bytes(),
            )
            .finish()
    }
}

/// One explicit worker-owned pool for reusable native request allocations.
///
/// A worker may be cloned into nested/baseline execution, so checkout is
/// synchronized. Checked-out buffers are exclusively owned by their request.
#[derive(Debug, Default)]
pub(super) struct NativeRequestPool {
    available: Vec<NativeRequestBuffers>,
}

impl NativeRequestPool {
    pub(super) fn checkout(&mut self, argument_capacity: usize) -> NativeRequestBuffers {
        let mut buffers = self.available.pop().unwrap_or_default();
        buffers.native_call_encoded_scratch.clear();
        if buffers.native_call_encoded_scratch.capacity() < argument_capacity {
            buffers
                .native_call_encoded_scratch
                .reserve(argument_capacity);
        }
        buffers
    }

    pub(super) fn recycle(&mut self, mut buffers: NativeRequestBuffers) {
        debug_assert_eq!(*buffers.direct_value_next, 0);
        debug_assert_eq!(*buffers.direct_array_next, 0);
        debug_assert_eq!(*buffers.direct_string_next, 0);
        debug_assert_eq!(*buffers.fiber_suspension_next, 0);
        debug_assert_eq!(*buffers.static_property_next, 0);
        debug_assert!(buffers.direct_resource_handles.is_empty());
        debug_assert!(buffers.direct_closure_handles.is_empty());
        debug_assert!(buffers.class_constant_cache.is_empty());
        buffers.native_call_encoded_scratch.clear();
        buffers.native_frame_arena.reset_for_pool();
        buffers.diagnostic_telemetry.reset_for_pool();
        const MAX_RETAINED_NATIVE_REQUESTS: usize = 1;
        if self.available.len() < MAX_RETAINED_NATIVE_REQUESTS {
            self.available.push(buffers);
        }
    }
}

fn trusted_continuation_storage(unit: &php_ir::IrUnit) -> (Vec<u32>, usize) {
    let mut offsets = Vec::with_capacity(unit.functions.len());
    let mut count = 0_usize;
    for function_index in 0..unit.functions.len() {
        offsets.push(
            u32::try_from(count)
                .expect("native continuation publication offset must fit the runtime ABI"),
        );
        let function = php_ir::FunctionId::new(
            u32::try_from(function_index).expect("native function index must fit the runtime ABI"),
        );
        let capacity = php_jit::region_ir::native_continuation_capacity_upper_bound(unit, function)
            .expect("native continuation publication function must exist");
        count = count
            .checked_add(capacity)
            .expect("native continuation publication capacity overflow");
    }
    u32::try_from(count)
        .expect("native continuation publication capacity must fit the runtime ABI");
    (offsets, count)
}

fn trusted_request_local_storage(
    unit: &php_ir::IrUnit,
) -> (
    Vec<u32>,
    php_runtime::api::StableNativeArena<php_jit::JitNativeRequestLocalSlot>,
) {
    let mut offsets = Vec::with_capacity(unit.functions.len());
    let mut count = 0_usize;
    for function in &unit.functions {
        offsets.push(u32::try_from(count).unwrap_or(u32::MAX));
        count = count.saturating_add(function.locals.len());
    }
    (offsets, php_runtime::api::StableNativeArena::new(count))
}

fn native_request_local_name(function: &php_ir::IrFunction, local: usize) -> Option<&str> {
    const SUPERGLOBALS: &[&str] = &[
        "_GET", "_POST", "_COOKIE", "_REQUEST", "_SERVER", "_ENV", "_FILES", "_SESSION",
    ];
    let name = function.locals.get(local)?.as_str();
    ((function.flags.is_top_level
        && name != "GLOBALS"
        && !php_ir::is_compiler_generated_local_name(name))
        || SUPERGLOBALS.contains(&name))
    .then_some(name)
}

struct PreparedNativeRuntimeClass {
    entry: php_runtime::api::ClassEntry,
    display_name: String,
    layout_id: u64,
    /// One request-owned native owner per initialized default. Each object
    /// instance retains these encoded values into its cloned slot vector.
    default_native_slots: Box<[php_runtime::api::NativeDeclaredPropertySlot]>,
}

enum NativeGeneratorDelegation {
    Array { source: i64, cursor: usize },
    Generator { generator: i64 },
}

// `control_reserved` is otherwise zero for generated native call states. The
// marker lets the Fiber suspension stack distinguish an opaque Generator
// continuation from an ordinary compiled caller without publishing a second
// value representation or ABI entry point.
const NATIVE_FIBER_GENERATOR_FOREACH_CONTINUATION: u32 = 0x4746_4f52;

enum NativeGeneratorAdvance {
    Yielded {
        key: i64,
        value: i64,
    },
    Complete,
    FiberSuspended {
        value: i64,
        active: i64,
        /// Direct Generators waiting for `active`, ordered from the immediate
        /// delegating parent out to the iterator exposed to foreach.
        parents: Vec<i64>,
    },
}

#[derive(Clone)]
struct NativeGeneratorFiberFrame {
    active: i64,
    parents: Vec<i64>,
}

struct NativeFiberExecution {
    target: NativeExecutionTarget,
    handle: php_jit::JitFunctionHandle,
    arguments: Vec<i64>,
    state: php_jit::JitDeoptState,
    nested: Option<Box<NativeFiberExecution>>,
    generator: Option<NativeGeneratorFiberFrame>,
}

impl NativeFiberExecution {
    fn resume_target(&self) -> &NativeExecutionTarget {
        self.nested
            .as_deref()
            .map_or(&self.target, NativeFiberExecution::resume_target)
    }
}

impl<'a> NativeRequestColdState<'a> {
    fn all_published_native_functions(&self) -> Vec<php_ir::FunctionId> {
        let mut functions = self
            .native_entries
            .keys()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        functions.extend(
            self.compiled
                .prepared_deployment_image()
                .preferred_function_entries
                .iter()
                .enumerate()
                .filter_map(|(function, entry)| {
                    (entry.load(std::sync::atomic::Ordering::Acquire) != 0)
                        .then(|| u32::try_from(function).ok())
                        .flatten()
                        .map(php_ir::FunctionId::new)
                }),
        );
        functions.into_iter().collect()
    }

    fn published_native_functions(&self) -> Vec<php_ir::FunctionId> {
        self.native_metadata_preparation_scope
            .clone()
            .unwrap_or_else(|| self.all_published_native_functions())
    }

    fn prepared_continuation_instructions(
        &self,
        function: php_ir::FunctionId,
    ) -> Option<std::sync::Arc<[Option<std::sync::Arc<php_ir::Instruction>>]>> {
        self.compiled.prepared_continuation_instructions(function)
    }

    fn published_continuation_ranges(&self) -> Vec<std::ops::Range<usize>> {
        self.published_native_functions()
            .into_iter()
            .filter_map(|function| {
                let instructions = self.prepared_continuation_instructions(function)?;
                let base = self
                    .trusted_property_function_offsets
                    .get(function.index())
                    .copied()
                    .and_then(|base| usize::try_from(base).ok())?;
                base.checked_add(instructions.len()).map(|end| base..end)
            })
            .collect()
    }

    fn published_request_local_ranges(&self) -> Vec<std::ops::Range<usize>> {
        self.published_native_functions()
            .into_iter()
            .filter_map(|function| {
                let local_count = self.unit.functions.get(function.index())?.locals.len();
                let base = self
                    .trusted_request_local_function_offsets
                    .get(function.index())
                    .copied()
                    .and_then(|base| usize::try_from(base).ok())?;
                base.checked_add(local_count).map(|end| base..end)
            })
            .collect()
    }

    /// Materialize immutable metadata and request-owned plans only for entries
    /// already published in the active unit. Calling this after an on-demand
    /// compilation is the single publication boundary for the newly reached
    /// function; dormant declarations are never traversed.
    pub(super) fn prepare_published_native_metadata(&mut self) -> Result<(), String> {
        let pending = self
            .all_published_native_functions()
            .into_iter()
            .filter(|function| !self.prepared_native_metadata_functions.contains(function))
            .collect::<Vec<_>>();
        if pending.is_empty() {
            if self.trusted_exception_route_symbol_epoch != self.external_signature_epoch {
                self.prepare_trusted_exception_routes();
            }
            return Ok(());
        }
        let rebuild_exception_routes = self.trusted_exception_route_symbol_epoch
            != self.external_signature_epoch
            || pending.iter().any(|function| {
                self.compiled
                    .preferred_function_metadata(*function)
                    .is_some_and(|metadata| {
                        metadata
                            .exception_handlers
                            .iter()
                            .any(|handler| handler.function == *function)
                    })
            });
        let rebuild_instanceof = pending.iter().any(|function| {
            self.prepared_continuation_instructions(*function)
                .is_some_and(|instructions| {
                    instructions.iter().flatten().any(|instruction| {
                        matches!(
                            &instruction.kind,
                            php_ir::InstructionKind::InstanceOf { class_name, .. }
                                if !class_name.eq_ignore_ascii_case("static")
                        )
                    })
                })
        });
        self.native_metadata_preparation_scope = Some(pending.clone());
        self.prepare_trusted_closure_plans();
        self.prepare_trusted_exception_plans();
        self.prepare_trusted_constant_fetches();
        self.prepare_trusted_request_locals();
        if let Err(error) = self.prepare_trusted_global_references() {
            self.native_metadata_preparation_scope = None;
            return Err(error);
        }
        self.prepare_trusted_static_locals();
        self.prepare_trusted_static_properties();
        self.prepare_trusted_class_plans();
        self.prepare_trusted_declared_properties();
        self.native_metadata_preparation_scope = None;
        if rebuild_instanceof {
            // The entry table is shared by every plan in this unit. Rebuild
            // it unit-wide only when a newly published function adds an
            // instanceof site; ordinary function publication remains
            // strictly incremental.
            self.prepare_trusted_instanceof_plans();
        }
        if rebuild_exception_routes {
            self.prepare_trusted_exception_routes();
        }
        self.prepared_native_metadata_functions.extend(pending);
        Ok(())
    }

    pub(super) fn native_runtime_ptr(&mut self) -> *mut std::ffi::c_void {
        self.fast_state.cast()
    }

    fn publish_active_call_argument_view(&mut self) {
        let (arguments, count, fixed_count) = self.call_frames.last().map_or((0, 0, 0), |frame| {
            (
                frame.arguments.as_ptr() as usize as u64,
                u32::try_from(frame.arguments.len()).unwrap_or(u32::MAX),
                frame.fixed_argument_count,
            )
        });
        // SAFETY: the separately allocated fast state is request-stable. A
        // selected linked runtime view is likewise request-owned and mutable
        // for the duration of this synchronous activation.
        #[allow(unsafe_code)]
        unsafe {
            let fast = &mut *self.fast_state;
            let view = if fast.header.runtime_view_pointer == 0 {
                &mut fast.header.runtime_view
            } else {
                &mut *(fast.header.runtime_view_pointer as usize
                    as *mut php_jit::JitNativeRuntimeView)
            };
            view.active_call_arguments = arguments;
            view.active_call_argument_count = count;
            view.active_call_fixed_argument_count = fixed_count;
            view.active_call_fixed_arguments = 0;
            view.active_call_tail_arguments = 0;
        }
    }

    fn take_native_fiber_suspension_state(
        &mut self,
        handle: u64,
    ) -> Result<Option<php_jit::JitDeoptState>, String> {
        if handle == 0 {
            return Ok(None);
        }
        let next = usize::try_from(*self.fiber_suspension_next)
            .map_err(|_| "native Fiber suspension stack is invalid".to_owned())?;
        let index = usize::try_from(handle - 1)
            .map_err(|_| "native Fiber suspension handle is invalid".to_owned())?;
        if index >= self.fiber_suspension_states.capacity() || index + 1 != next {
            return Err(format!(
                "native Fiber suspension stack is not LIFO: handle={handle} depth={next}"
            ));
        }
        *self.fiber_suspension_next = u32::try_from(index).unwrap_or(0);
        Ok(Some(self.fiber_suspension_states[index]))
    }

    fn discard_native_fiber_suspension_states(&mut self) {
        // Stack entries are snapshots of owners already carried by generated
        // activation state; the arena itself owns no encoded values. Native
        // code updates only the current stack depth, so a fully popped stack
        // does not retain a separate high-water mark. Discarding the reserved
        // range decommits every page touched by this request without moving
        // the worker-stable mapping.
        self.fiber_suspension_states
            .discard_prefix(self.fiber_suspension_states.capacity());
        *self.fiber_suspension_next = 0;
    }

    /// Releases the owners captured in a suspended native activation when no
    /// generated continuation will ever resume it. Normal return/unwind runs
    /// the generated epilogue and must not pass through this path.
    fn abandon_native_fiber_execution(
        &mut self,
        execution: NativeFiberExecution,
    ) -> Result<(), String> {
        let NativeFiberExecution {
            target: _,
            handle,
            arguments: _,
            state,
            nested,
            generator,
        } = execution;
        if let Some(nested) = nested {
            self.abandon_native_fiber_execution(*nested)?;
        }
        self.release_native_suspension_owners(&handle, &state)?;
        if let Some(generator) = generator
            && let Some(index) = self.direct_generator_index(generator.active)
            && let Some(activation) = self.direct_generator_mut(index)
        {
            activation.state = None;
            activation.lifecycle = php_runtime::api::GeneratorState::Errored;
        }
        Ok(())
    }

    fn release_native_suspension_owners(
        &mut self,
        handle: &php_jit::JitFunctionHandle,
        state: &php_jit::JitDeoptState,
    ) -> Result<(), String> {
        let metadata = handle
            .region_state_metadata()
            .ok_or_else(|| "suspended native activation has no state metadata".to_owned())?;
        let (owned_locals, owned_registers) = metadata
            .suspensions
            .iter()
            .find(|entry| {
                entry.function.raw() == state.function_id
                    && entry.continuation_id == state.continuation_id
            })
            .map(|entry| (&entry.owned_locals, &entry.owned_registers))
            .or_else(|| {
                metadata
                    .native_transitions
                    .iter()
                    .find(|entry| {
                        entry.function.raw() == state.function_id
                            && entry.continuation_id == state.continuation_id
                    })
                    .map(|entry| (&entry.owned_locals, &entry.owned_registers))
            })
            .ok_or_else(|| {
                format!(
                    "suspended native activation state {}:{} has no ownership metadata",
                    state.function_id, state.continuation_id
                )
            })?;

        let mut owners = owned_locals
            .iter()
            .filter(|local| state.local_initialized(**local))
            .map(|local| state.slots[local.index()])
            .collect::<Vec<_>>();
        for snapshot in 0..php_jit::JIT_DEOPT_MAX_REGISTERS {
            let initialized = state.initialized_register_mask
                & 1_u64
                    .checked_shl(u32::try_from(snapshot).unwrap_or(u32::MAX))
                    .unwrap_or(0)
                != 0;
            if initialized
                && owned_registers
                    .iter()
                    .any(|register| register.raw() == state.register_ids[snapshot])
            {
                owners.push(state.registers[snapshot]);
            }
        }
        if self.completed_nested_fiber_call.as_ref().is_some_and(
            |(function, continuation, _, _)| {
                *function == state.function_id && *continuation == state.continuation_id
            },
        ) && let Some((_, _, _, value)) = self.completed_nested_fiber_call.take()
        {
            owners.push(value);
        }
        for owner in owners {
            self.release_if_live(owner)?;
        }
        Ok(())
    }

    pub(super) const fn process_exit_terminates_process(&self) -> bool {
        self.registered_extensions.is_fork_child()
    }

    /// Publish every immutable source literal into the request-wide native
    /// value plane once per compiled unit. Generated storage operations borrow
    /// these slots and retain only when the value actually acquires an owner.
    ///
    /// Named and class constants are deliberately excluded: their PHP-visible
    /// resolution remains a cold/exact operation and cannot be frozen as a
    /// source literal.
    fn prepare_trusted_literal_slots(&mut self) {
        let identity = self.unit_identity;
        if self.trusted_literal_slots.contains_key(&identity) {
            return;
        }
        let constants = self.unit.constants.clone();
        // Slot zero is an unreadable-state sentinel for branchless generated
        // lookup when a dynamic value is not a unit literal. It must exist
        // even for a constant-free unit.
        let mut slots =
            vec![php_jit::JitNativeTrustedLiteralSlot::default(); constants.len().max(1)];
        for (index, constant) in constants.iter().enumerate() {
            if matches!(
                constant,
                php_ir::IrConstant::NamedConstant(_) | php_ir::IrConstant::ClassConstant { .. }
            ) {
                continue;
            }
            let Ok(value) = self.encode_native_ir_constant_owned(constant) else {
                continue;
            };
            slots[index] = php_jit::JitNativeTrustedLiteralSlot {
                value,
                state: php_jit::JIT_NATIVE_TRUSTED_LITERAL_PUBLISHED,
                reserved: 0,
            };
        }
        self.trusted_literal_slots
            .insert(identity, slots.into_boxed_slice());
    }

    fn clear_trusted_literal_slots(&mut self) {
        let values = std::mem::take(&mut self.trusted_literal_slots)
            .into_values()
            .flat_map(|slots| {
                slots
                    .into_vec()
                    .into_iter()
                    .filter_map(|slot| {
                        (slot.state == php_jit::JIT_NATIVE_TRUSTED_LITERAL_PUBLISHED)
                            .then_some(slot.value)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for value in values {
            let _ = self.release_if_live(value);
        }
    }

    /// Cold symbol-mutation hook for one newly visible constant. Resolution
    /// and encoding occur once here; every exact callsite receives an owned
    /// handle and generated code subsequently performs only a numeric load.
    fn publish_trusted_constant_encoding(&mut self, name: &str, encoded: i64) {
        for function in self.published_native_functions() {
            let Some(instructions) = self.prepared_continuation_instructions(function) else {
                continue;
            };
            let function = function.raw();
            for (continuation, instruction) in instructions.iter().enumerate() {
                let Some(instruction) = instruction.as_ref() else {
                    continue;
                };
                if !matches!(
                    &instruction.kind,
                    php_ir::InstructionKind::FetchConst { name: candidate, .. }
                        if candidate == name
                ) {
                    continue;
                }
                let Ok(continuation) = u32::try_from(continuation) else {
                    continue;
                };
                let _ = self.publish_trusted_constant_fetch(function, continuation, encoded);
            }
        }
    }

    /// Publish immutable closure allocation descriptors only for functions
    /// whose native entries are callable in this unit. Dormant declarations
    /// retain neither a RegionGraph-derived index nor resident plan pages.
    fn prepare_trusted_closure_plans(&mut self) {
        for function in self.published_native_functions() {
            let Some(sites) = self.compiled.prepared_native_closure_sites(function) else {
                continue;
            };
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function.index())
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                continue;
            };
            for (continuation, site) in sites.iter().enumerate() {
                let Some(site) = site.as_ref() else {
                    continue;
                };
                let Some(plan) = self
                    .trusted_closure_plans
                    .get_mut(base.saturating_add(continuation))
                else {
                    continue;
                };
                *plan = Arc::as_ptr(site) as usize as u64;
            }
        }
    }

    /// Resolves internal throwable class and source metadata once per
    /// published `MakeException` continuation. The optimizing allocator
    /// consumes only these stable opaque plans and the native message value.
    fn prepare_trusted_exception_plans(&mut self) {
        let mut sites = Vec::new();
        for function in self.published_native_functions() {
            let Some((function_name, include_function_frame)) = self
                .unit
                .functions
                .get(function.index())
                .map(|function| (function.name.clone(), !function.flags.is_top_level))
            else {
                continue;
            };
            let Some(instructions) = self.prepared_continuation_instructions(function) else {
                continue;
            };
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function.index())
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                continue;
            };
            for (continuation, instruction) in instructions.iter().enumerate() {
                let Some(instruction) = instruction.as_ref() else {
                    continue;
                };
                let php_ir::InstructionKind::MakeException { class_name, .. } = &instruction.kind
                else {
                    continue;
                };
                sites.push((
                    base.saturating_add(continuation),
                    class_name.clone(),
                    function_name.clone(),
                    include_function_frame,
                    instruction.span,
                ));
            }
        }
        for (index, class_name, function_name, include_function_frame, span) in sites {
            if self
                .trusted_exception_plans
                .get(index)
                .is_some_and(|plan| *plan != 0)
            {
                continue;
            }
            let prepared = Box::new(prepare_native_throwable_site(
                self,
                &class_name,
                &function_name,
                include_function_frame,
                span,
            ));
            let pointer = std::ptr::from_ref(prepared.as_ref()) as usize as u64;
            self.trusted_exception_plan_owners.insert(index, prepared);
            if let Some(plan) = self.trusted_exception_plans.get_mut(index) {
                *plan = pointer;
            }
        }
    }

    /// Publish exact declared-property slots for statically proven object
    /// classes. Visibility, hooks, readonly/type constraints, layout identity,
    /// and numeric storage location are resolved once before native entry.
    fn prepare_trusted_declared_properties(&mut self) {
        let owner = self.current_dynamic_unit;
        for function in self.published_native_functions() {
            let Some(instructions) = self.compiled.prepared_native_property_sites(function) else {
                continue;
            };
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function.index())
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                continue;
            };
            for (continuation, site) in instructions.iter().enumerate() {
                let Some(site) = site.as_ref() else {
                    continue;
                };
                let Some(class) = self.unit.classes.get(site.class_index as usize) else {
                    continue;
                };
                let prepared = {
                    self.runtime_class_cache
                        .borrow()
                        .get(&(owner, class.name.clone()))
                        .cloned()
                };
                let Some(prepared) = prepared else {
                    continue;
                };
                let Some(declaration) = native_instance_property_declaration(
                    self,
                    &class.name,
                    &site.property,
                    function.raw(),
                ) else {
                    continue;
                };
                let property = &declaration.entry;
                // The statically prepared receiver class and the compiling
                // method are resolved to the declaration owner once. This
                // preserves inherited protected access without repeating
                // hierarchy or visibility checks in generated code.
                let readable =
                    native_instance_property_readable(self, &declaration, function.raw())
                        && property.hooks.get.is_none();
                let setter_visible = (!property.flags.set_is_private
                    && !property.flags.set_is_protected)
                    || native_instance_property_writable(self, &declaration, function.raw());
                let writable = readable
                    && !prepared.entry.flags.is_readonly
                    && !property.flags.is_readonly
                    && setter_visible
                    && !property.flags.is_typed
                    && property.type_.is_none()
                    && property.hooks.set.is_none();
                let referenceable = writable && property.hooks.get.is_none();
                let dimension_writable = readable
                    && !prepared.entry.flags.is_readonly
                    && !property.flags.is_readonly
                    && setter_visible
                    && property.hooks.set.is_none();
                let admitted = match site.required_state {
                    php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_PUBLISHED => readable,
                    php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_WRITABLE => writable,
                    php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_REFERENCEABLE => referenceable,
                    php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_DIMENSION_WRITABLE => {
                        dimension_writable
                    }
                    _ => false,
                };
                if !admitted {
                    continue;
                }
                let Some(slot_index) = php_runtime::api::ObjectRef::prepared_declared_slot_index(
                    &prepared.entry,
                    &prepared.display_name,
                    &site.property,
                ) else {
                    continue;
                };
                let Some(plan) = self
                    .trusted_property_slots
                    .get_mut(base.saturating_add(continuation))
                else {
                    continue;
                };
                *plan = php_jit::JitNativeTrustedPropertySlot {
                    state: site.required_state,
                    slot_index,
                    // A non-final instance method can receive any subclass.
                    // Runtime class assembly appends inherited backed slots in
                    // lineage order, so the declaring slot is a stable prefix.
                    // Zero publishes that class-family contract; exact/final
                    // layouts retain the ordinary identity guard.
                    layout_id: if prepared.entry.flags.is_final {
                        prepared.layout_id
                    } else {
                        0
                    },
                };
            }
        }
    }

    /// Resolve fixed `instanceof C` sites into immutable layout-id hash
    /// tables. Every class whose object layout is currently visible receives
    /// an exact boolean result. A class loaded later has a new unknown layout
    /// and therefore takes the site's single baseline continuation.
    fn prepare_trusted_instanceof_plans(&mut self) {
        let published_functions = self.published_native_functions();
        let has_instanceof_site = published_functions.iter().any(|function| {
            self.prepared_continuation_instructions(*function)
                .is_some_and(|instructions| {
                    instructions.iter().flatten().any(|instruction| {
                        matches!(
                            &instruction.kind,
                            php_ir::InstructionKind::InstanceOf { class_name, .. }
                                if !class_name.eq_ignore_ascii_case("static")
                        )
                    })
                })
        });
        if !has_instanceof_site {
            return;
        }
        for function in &published_functions {
            let Some(instructions) = self.prepared_continuation_instructions(*function) else {
                continue;
            };
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function.index())
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                continue;
            };
            let end = base.saturating_add(instructions.len());
            if let Some(plans) = self.trusted_instanceof_plans.get_mut(base..end) {
                plans.fill(Default::default());
            }
        }
        self.trusted_instanceof_entries.clear();

        let (known_names, layouts) = {
            let mut seen = std::collections::BTreeSet::new();
            let mut declarations = Vec::new();
            for class in &self.unit.classes {
                if class.flags.is_conditional && !self.class_is_visible(&class.name) {
                    continue;
                }
                if seen.insert(class.name.clone()) {
                    declarations.push((self.current_dynamic_unit, class));
                }
            }
            for (name, unit) in &self.external_class_units {
                if self.current_dynamic_unit == Some(*unit) || !seen.insert(name.clone()) {
                    continue;
                }
                let Some(class) = self
                    .dynamic_units
                    .get(*unit)
                    .and_then(|package| package.compiled.lookup_unit_class(name))
                else {
                    continue;
                };
                declarations.push((Some(*unit), class));
            }

            let known_names = declarations
                .iter()
                .map(|(_, class)| class.name.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let layouts = declarations
                .iter()
                .filter(|(_, class)| {
                    !class.flags.is_abstract && !class.flags.is_interface && !class.flags.is_trait
                })
                .filter_map(|(owner, class)| {
                    self.prepared_runtime_class_layout_id(*owner, class)
                        .map(|layout_id| (class.name.clone(), layout_id))
                })
                .collect::<Vec<_>>();
            (known_names, layouts)
        };

        for function in published_functions {
            let Some(instructions) = self.prepared_continuation_instructions(function) else {
                continue;
            };
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function.index())
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                continue;
            };
            let caller_function = function.raw();
            for (continuation, instruction) in instructions.iter().enumerate() {
                let Some(instruction) = instruction.as_ref() else {
                    continue;
                };
                let php_ir::InstructionKind::InstanceOf { class_name, .. } = &instruction.kind
                else {
                    continue;
                };
                if class_name.eq_ignore_ascii_case("static") {
                    continue;
                }
                let Ok(target) =
                    native_resolve_scoped_class_name(self, class_name, caller_function)
                else {
                    continue;
                };
                let target = normalize_class_name(&target);
                if self.class_aliases.contains_key(&target) || !known_names.contains(&target) {
                    continue;
                }

                let capacity = layouts.len().saturating_mul(2).max(2).next_power_of_two();
                let Ok(mask) = u32::try_from(capacity - 1) else {
                    continue;
                };
                let Ok(entry_offset) = u32::try_from(self.trusted_instanceof_entries.len()) else {
                    continue;
                };
                self.trusted_instanceof_entries.resize(
                    self.trusted_instanceof_entries
                        .len()
                        .saturating_add(capacity),
                    php_jit::JitNativeInstanceOfEntry::default(),
                );
                for (candidate, layout_id) in &layouts {
                    let result = native_internal_instanceof(candidate, &target)
                        .unwrap_or_else(|| native_class_is_a(self, candidate, &target));
                    let mut bucket = php_jit::jit_native_instanceof_index(*layout_id, mask);
                    loop {
                        let index = entry_offset as usize + bucket as usize;
                        let entry = &mut self.trusted_instanceof_entries[index];
                        if entry.layout_id == 0 || entry.layout_id == *layout_id {
                            *entry = php_jit::JitNativeInstanceOfEntry {
                                layout_id: *layout_id,
                                result: u32::from(result),
                                reserved: 0,
                            };
                            break;
                        }
                        bucket = bucket.wrapping_add(1) & mask;
                    }
                }
                let Some(plan) = self
                    .trusted_instanceof_plans
                    .get_mut(base.saturating_add(continuation))
                else {
                    continue;
                };
                *plan = php_jit::JitNativeInstanceOfPlan {
                    entry_offset,
                    mask,
                    state: php_jit::JIT_NATIVE_INSTANCEOF_PLAN_PUBLISHED,
                    reserved: 0,
                };
            }
        }
    }

    /// Resolve one immutable class layout while keeping every `RefCell`
    /// borrow confined to its cache probe. Chaining `borrow().get().or_else`
    /// with a later `borrow_mut()` keeps the temporary immutable guard alive
    /// for the complete expression and panics precisely on a cache miss.
    fn prepared_runtime_class_layout_id(
        &self,
        owner: Option<usize>,
        class: &php_ir::module::ClassEntry,
    ) -> Option<u64> {
        let key = (owner, class.name.clone());
        if let Some(layout_id) = { self.runtime_class_layout_cache.borrow().get(&key).copied() } {
            return Some(layout_id);
        }
        if let Some(layout_id) = {
            self.runtime_class_cache
                .borrow()
                .get(&key)
                .map(|prepared| prepared.layout_id)
        } {
            return Some(layout_id);
        }
        let runtime = native_runtime_class_with_owner(self, owner, class).ok()?;
        let layout_id =
            php_runtime::api::ObjectRef::prepared_layout_id(&runtime, &class.display_name);
        self.runtime_class_layout_cache
            .borrow_mut()
            .insert(key, layout_id);
        Some(layout_id)
    }

    /// Resolve every currently published compiled catch/finally edge into an
    /// immutable throwable-layout table. A direct compiled caller consumes
    /// this table while its own machine activation is still live, then
    /// re-enters the same fixed callee entry at the selected handler block.
    fn prepare_trusted_exception_routes(&mut self) {
        let published_functions = self.all_published_native_functions();
        for function in &published_functions {
            let Some(instructions) = self.prepared_continuation_instructions(*function) else {
                continue;
            };
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function.index())
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                continue;
            };
            let end = base.saturating_add(instructions.len());
            if let Some(plans) = self.trusted_exception_route_plans.get_mut(base..end) {
                plans.fill(Default::default());
            }
        }
        self.trusted_exception_route_entries.clear();

        let mut layouts = Vec::<(String, u64)>::new();
        let mut seen_layouts = std::collections::BTreeSet::new();
        let mut seen_classes = std::collections::BTreeSet::new();
        for class in &self.unit.classes {
            if class.flags.is_conditional && !self.class_is_visible(&class.name) {
                continue;
            }
            if class.flags.is_abstract || class.flags.is_interface || class.flags.is_trait {
                continue;
            }
            let layout_id = self.prepared_runtime_class_layout_id(self.current_dynamic_unit, class);
            if let Some(layout_id) = layout_id
                && seen_layouts.insert(layout_id)
            {
                seen_classes.insert(normalize_class_name(&class.name));
                layouts.push((class.name.clone(), layout_id));
            }
        }
        for (name, unit) in &self.external_class_units {
            if self.current_dynamic_unit == Some(*unit)
                || seen_classes.contains(&normalize_class_name(name))
            {
                continue;
            }
            let Some(class) = self
                .dynamic_units
                .get(*unit)
                .and_then(|package| package.compiled.lookup_unit_class(name))
            else {
                continue;
            };
            if class.flags.is_abstract || class.flags.is_interface || class.flags.is_trait {
                continue;
            }
            let layout_id = self.prepared_runtime_class_layout_id(Some(*unit), class);
            if let Some(layout_id) = layout_id
                && seen_layouts.insert(layout_id)
            {
                seen_classes.insert(normalize_class_name(&class.name));
                layouts.push((class.name.clone(), layout_id));
            }
        }
        let registry = php_std::ExtensionRegistry::standard_library();
        for class in registry
            .extensions()
            .filter(|extension| registry.is_extension_enabled(extension.name()))
            .flat_map(|extension| extension.classes())
        {
            let candidate = normalize_class_name(class.name());
            if seen_classes.contains(&candidate)
                || !native_class_is_a(self, &candidate, "throwable")
            {
                continue;
            }
            let (runtime, display_name) = cold_diagnostics::native_throwable_class(class.name());
            let layout_id =
                php_runtime::api::ObjectRef::prepared_layout_id(&runtime, &display_name);
            if seen_layouts.insert(layout_id) {
                seen_classes.insert(candidate);
                layouts.push((class.name().to_owned(), layout_id));
            }
        }
        for class in [
            "Exception",
            "ErrorException",
            "Error",
            "TypeError",
            "ValueError",
            "ArgumentCountError",
            "ArithmeticError",
            "DivisionByZeroError",
            "CompileError",
            "ParseError",
            "FiberError",
            "UnhandledMatchError",
            "RuntimeException",
            "LogicException",
        ] {
            let candidate = normalize_class_name(class);
            if seen_classes.contains(&candidate) {
                continue;
            }
            let (runtime, display_name) = cold_diagnostics::native_throwable_class(class);
            let layout_id =
                php_runtime::api::ObjectRef::prepared_layout_id(&runtime, &display_name);
            if seen_layouts.insert(layout_id) {
                seen_classes.insert(candidate);
                layouts.push((class.to_owned(), layout_id));
            }
        }

        for function in published_functions {
            let Some(metadata) = self.compiled.preferred_function_metadata(function) else {
                continue;
            };
            let exception_handlers = metadata
                .exception_handlers
                .iter()
                .filter(|handler| handler.function == function)
                .collect::<Vec<_>>();
            if exception_handlers.is_empty() {
                continue;
            }
            let continuations = metadata
                .continuations
                .iter()
                .filter(|continuation| continuation.function == function);
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function.index())
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                continue;
            };
            for continuation in continuations {
                let handlers = exception_handlers
                    .iter()
                    .copied()
                    .filter(|handler| handler.protected_blocks.contains(&continuation.block))
                    .collect::<Vec<_>>();
                if handlers.is_empty() {
                    continue;
                }
                let decisions = layouts
                    .iter()
                    .filter_map(|(candidate, layout_id)| {
                        let decision = handlers.iter().rev().find_map(|handler| {
                            if let Some(catch) = handler.catch {
                                let matches = handler.catch_types.iter().any(|target| {
                                    let target = native_resolve_scoped_class_name(
                                        self,
                                        target,
                                        function.raw(),
                                    )
                                    .unwrap_or_else(|_| target.clone());
                                    native_internal_instanceof(candidate, &target).unwrap_or_else(
                                        || native_class_is_a(self, candidate, &target),
                                    )
                                });
                                if matches {
                                    return Some((
                                        php_jit::jit_native_handler_resume_id(catch),
                                        php_jit::JitCallStatus::CONTINUE.0,
                                    ));
                                }
                            }
                            handler.finally.map(|finally| {
                                (
                                    php_jit::jit_native_handler_resume_id(finally),
                                    php_jit::JitCallStatus::THROW.0,
                                )
                            })
                        })?;
                        Some((*layout_id, decision.0, decision.1))
                    })
                    .collect::<Vec<_>>();
                if decisions.is_empty() {
                    continue;
                }
                let capacity = decisions.len().saturating_mul(2).max(2).next_power_of_two();
                let Ok(mask) = u32::try_from(capacity - 1) else {
                    continue;
                };
                let Ok(entry_offset) = u32::try_from(self.trusted_exception_route_entries.len())
                else {
                    continue;
                };
                self.trusted_exception_route_entries.resize(
                    self.trusted_exception_route_entries
                        .len()
                        .saturating_add(capacity),
                    php_jit::JitNativeExceptionRouteEntry::default(),
                );
                for (layout_id, resume_id, pending_status) in decisions {
                    let mut bucket = php_jit::jit_native_instanceof_index(layout_id, mask);
                    loop {
                        let index = entry_offset as usize + bucket as usize;
                        let entry = &mut self.trusted_exception_route_entries[index];
                        if entry.layout_id == 0 || entry.layout_id == layout_id {
                            *entry = php_jit::JitNativeExceptionRouteEntry {
                                layout_id,
                                resume_id,
                                pending_status,
                            };
                            break;
                        }
                        bucket = bucket.wrapping_add(1) & mask;
                    }
                }
                let Some(plan) = self
                    .trusted_exception_route_plans
                    .get_mut(base.saturating_add(continuation.id as usize))
                else {
                    continue;
                };
                *plan = php_jit::JitNativeExceptionRoutePlan {
                    entry_offset,
                    mask,
                    state: php_jit::JIT_NATIVE_EXCEPTION_ROUTE_PUBLISHED,
                    reserved: 0,
                };
            }
        }
        self.trusted_exception_route_symbol_epoch = self.external_signature_epoch;
    }

    fn direct_static_property_encoded(&self, key: &(String, String)) -> Option<i64> {
        let index = usize::try_from(*self.static_property_indices.get(key)?).ok()?;
        let slot = self.static_property_slots.get(index)?;
        (slot.initialized != 0).then_some(slot.value)
    }

    /// Publish the immutable result of one exact constant continuation.
    /// The plan retains its own owner; the caller keeps the owner returned by
    /// the baseline operation for the current SSA result.
    fn publish_trusted_constant_fetch(
        &mut self,
        function: u32,
        continuation: u32,
        encoded: i64,
    ) -> Result<(), String> {
        let base = self
            .trusted_property_function_offsets
            .get(function as usize)
            .copied()
            .and_then(|base| usize::try_from(base).ok())
            .ok_or_else(|| "trusted constant function index is missing".to_owned())?;
        let index = base
            .checked_add(continuation as usize)
            .ok_or_else(|| "trusted constant continuation index overflow".to_owned())?;
        let plan = self
            .trusted_constant_slots
            .get(index)
            .copied()
            .ok_or_else(|| "trusted constant continuation is missing".to_owned())?;
        if plan.state == php_jit::JIT_NATIVE_TRUSTED_CONSTANT_PUBLISHED {
            return Ok(());
        }
        self.retain(encoded)?;
        self.trusted_constant_slots[index] = php_jit::JitNativeTrustedConstantSlot {
            value: encoded,
            state: php_jit::JIT_NATIVE_TRUSTED_CONSTANT_PUBLISHED,
            reserved: 0,
        };
        Ok(())
    }

    fn clear_trusted_constant_fetches(&mut self) {
        let mut values = Vec::new();
        for range in self.published_continuation_ranges() {
            values.extend(
                self.trusted_constant_slots[range]
                    .iter_mut()
                    .filter_map(|slot| {
                        (slot.state == php_jit::JIT_NATIVE_TRUSTED_CONSTANT_PUBLISHED).then(|| {
                            let value = slot.value;
                            *slot = php_jit::JitNativeTrustedConstantSlot::default();
                            value
                        })
                    }),
            );
        }
        for value in values {
            let _ = self.release_if_live(value);
        }
    }

    pub(super) fn recycle_native_request_buffers(&mut self) {
        cold_dynamic_units::schedule_hot_native_functions(self);
        self.clear_trusted_constant_fetches();
        self.clear_trusted_literal_slots();
        self.clear_trusted_request_locals();
        self.clear_trusted_global_references();
        self.clear_trusted_static_locals();
        let suspended_fibers = std::mem::take(&mut self.fiber_executions);
        for (_, execution) in suspended_fibers {
            let _ = self.abandon_native_fiber_execution(execution);
        }
        if let Some(value) = self.pending_fiber_suspension_value.take() {
            let _ = self.release_if_live(value);
        }
        if let Some((_, _, _, value)) = self.completed_nested_fiber_call.take() {
            let _ = self.release_if_live(value);
        }
        self.discard_native_fiber_suspension_states();
        self.native_execution_scopes.truncate(1);
        self.current_native_execution_scope = 1;
        self.active_fiber = None;
        let registered_callbacks = std::mem::take(&mut self.registered_callbacks);
        let _ = self.release_registered_callback_state(registered_callbacks);
        // ObjectRef identities may escape an include/nested VM through
        // globals or returned symbols. Their native property cells point into
        // this request arena, so restore every such object before the arena is
        // force-recycled. Doing this after individual slots were reclaimed
        // made graph order observable and could leave an escaped empty shell.
        let _ = self.demote_all_direct_objects();
        let stream_context = std::mem::take(&mut self.native_stream_context);
        for options in stream_context.resource_options.into_values() {
            let _ = self.release_if_live(options);
        }
        let _ = self.release_if_live(stream_context.default_options);
        let direct_value_used = usize::try_from(*self.direct_value_next).unwrap_or(0);
        let direct_array_used = usize::try_from(*self.direct_array_next).unwrap_or(0);
        let direct_string_used = usize::try_from(*self.direct_string_next).unwrap_or(0);
        let static_property_used = usize::try_from(*self.static_property_next).unwrap_or(0);
        let static_values = self
            .static_property_slots
            .get(..static_property_used)
            .unwrap_or_default()
            .iter()
            .filter(|slot| slot.initialized != 0)
            .map(|slot| slot.value)
            .collect::<Vec<_>>();
        self.static_property_slots
            .discard_prefix(static_property_used);
        *self.static_property_next = 0;
        self.static_property_indices.clear();
        for value in static_values {
            let _ = self.release_if_live(value);
        }
        for index in (0..direct_value_used).rev() {
            while self.direct_value_slots[index].refcount != 0 {
                if self.release_direct_value_index(index).is_err() {
                    break;
                }
            }
        }
        self.direct_value_slots.discard_prefix(direct_value_used);
        self.direct_object_owners.discard_prefix(direct_value_used);
        self.direct_array_states.discard_prefix(direct_value_used);
        self.direct_array_entries.discard_prefix(direct_array_used);
        *self.direct_value_next = 0;
        *self.direct_array_next = 0;
        *self.direct_value_free_head = php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE;
        *self.direct_value_reused_bytes = 0;
        self.direct_array_free_heads
            .fill(php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE);
        *self.direct_array_reused_bytes = 0;
        self.direct_string_free_heads
            .fill(php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE);
        *self.direct_string_reused_bytes = 0;
        self.direct_string_bytes.discard_prefix(direct_string_used);
        *self.direct_string_next = 0;
        self.baseline_values.direct_reference_cells.clear();
        self.baseline_values.materialized_direct_references.clear();
        self.native_global_reference_handles.clear();
        self.baseline_values.direct_object_handles.clear();
        debug_assert!(self.direct_resource_handles.is_empty());
        self.direct_resource_handles.clear();
        debug_assert!(self.direct_closure_handles.is_empty());
        self.direct_closure_handles.clear();
        self.baseline_values.direct_fiber_handles.clear();
        self.baseline_values.direct_fiber_cells.clear();
        self.baseline_values.direct_generator_handles.clear();
        self.baseline_values.direct_generator_cells.clear();
        // This content index owns neither slots nor bytes. Generated native
        // releases can retire a string without crossing the cold HashMap, so
        // dead indices are expected and are filtered by lookup. Request reset
        // discards the complete non-owning index with the recycled arenas.
        self.direct_string_interned_slots.clear();
        self.baseline_values.direct_array_handles.clear();
        self.baseline_values.direct_array_storage_ids.clear();
        self.baseline_values.direct_array_encode_depth = 0;
        self.class_constant_cache.clear();
        let diagnostic_telemetry = std::mem::replace(
            &mut self.runtime_telemetry,
            Rc::new(RefCell::new(NativeRuntimeTelemetry::default())),
        );
        let mut diagnostic_telemetry = Rc::try_unwrap(diagnostic_telemetry)
            .map(RefCell::into_inner)
            .unwrap_or_default();
        diagnostic_telemetry.reset_for_pool();
        self.worker_state
            .recycle_native_request_buffers(NativeRequestBuffers {
                direct_value_slots: std::mem::take(&mut self.direct_value_slots),
                direct_value_next: std::mem::take(&mut self.direct_value_next),
                direct_object_owners: std::mem::take(&mut self.direct_object_owners),
                direct_array_states: std::mem::take(&mut self.direct_array_states),
                direct_array_entries: std::mem::take(&mut self.direct_array_entries),
                direct_array_next: std::mem::take(&mut self.direct_array_next),
                direct_value_free_head: std::mem::take(&mut self.direct_value_free_head),
                direct_value_reused_bytes: std::mem::take(&mut self.direct_value_reused_bytes),
                direct_array_free_heads: std::mem::take(&mut self.direct_array_free_heads),
                direct_array_reused_bytes: std::mem::take(&mut self.direct_array_reused_bytes),
                direct_string_bytes: std::mem::take(&mut self.direct_string_bytes),
                direct_string_next: std::mem::take(&mut self.direct_string_next),
                direct_string_free_heads: std::mem::take(&mut self.direct_string_free_heads),
                direct_string_reused_bytes: std::mem::take(&mut self.direct_string_reused_bytes),
                fiber_suspension_states: std::mem::take(&mut self.fiber_suspension_states),
                fiber_suspension_next: std::mem::take(&mut self.fiber_suspension_next),
                static_property_slots: std::mem::take(&mut self.static_property_slots),
                static_property_next: std::mem::take(&mut self.static_property_next),
                native_call_encoded_scratch: std::mem::take(&mut self.native_call_encoded_scratch),
                native_frame_arena: std::mem::take(&mut self.native_frame_arena),
                direct_resource_handles: std::mem::take(&mut self.direct_resource_handles),
                direct_closure_handles: std::mem::take(&mut self.direct_closure_handles),
                class_constant_cache: std::mem::take(&mut self.class_constant_cache),
                diagnostic_telemetry,
            });
    }

    fn reset_execution_deadline_seconds(&mut self, seconds: u64) {
        if !self.execution_deadline_mutable {
            return;
        }
        self.execution_deadline_at = if seconds == 0 {
            None
        } else {
            std::time::Instant::now().checked_add(std::time::Duration::from_secs(seconds))
        };
    }

    fn publish_native_entry_address(&self, function: php_ir::FunctionId, address: usize) {
        let deployment = self.compiled.prepared_deployment_image();
        if let Some(cell) = deployment.native_function_entries.get(function.index()) {
            cell.store(address, std::sync::atomic::Ordering::Release);
        }
        if let Some(cell) = deployment.preferred_function_entries.get(function.index()) {
            let _ = cell.compare_exchange(
                0,
                address,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            );
        }
    }

    pub(super) fn attach_root_deployment_image(
        &mut self,
        compiled: crate::compiled_unit::CompiledUnit,
    ) {
        if self.current_dynamic_unit.is_some() {
            return;
        }
        let unit = self.dynamic_units.len();
        let deployment = compiled.prepared_deployment_image();
        for (function, handle) in self.native_entries.iter() {
            if !handle.region_state_metadata().is_some_and(|metadata| {
                metadata.compiler_tier == php_jit::region_ir::NativeCompilerTier::Baseline
            }) {
                continue;
            }
            if let (Some(cell), Some(preferred), Some(address)) = (
                deployment.native_function_entries.get(function.index()),
                deployment.preferred_function_entries.get(function.index()),
                handle.native_entry_address(),
            ) {
                cell.store(address, std::sync::atomic::Ordering::Release);
                let _ = preferred.compare_exchange(
                    0,
                    address,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                );
            }
        }
        let native_entry_signature_hashes = if self.include_child {
            self.native_entries
                .keys()
                .copied()
                .map(|function| {
                    let signatures = cold_dynamic_units::visible_external_function_signatures(
                        self, &compiled, function,
                    );
                    (
                        function,
                        super::external_function_signatures_hash(&signatures),
                    )
                })
                .collect()
        } else {
            // Before the root image is attached there are no runtime
            // declaration overlays. Each compiled entry therefore hashes its
            // immutable late-link placeholder set.
            self.native_entries
                .keys()
                .copied()
                .map(|function| {
                    let signatures =
                        super::linked_external_function_signatures(&compiled, function, &[]);
                    (
                        function,
                        super::external_function_signatures_hash(&signatures),
                    )
                })
                .collect()
        };
        if !self.include_child && !deployment.function_exports.is_empty() {
            self.external_signature_epoch = self.external_signature_epoch.saturating_add(1);
        }
        let native_entry_signature_epochs = self
            .native_entries
            .keys()
            .copied()
            .map(|function| (function, self.external_signature_epoch))
            .collect();
        let runtime_state = NativeUnitRuntimeState::for_compiled(&compiled);
        let linked_functions = vec![
            php_jit::JitNativeLinkedFunction::default();
            compiled.prepared_linked_function_count()
        ]
        .into_boxed_slice();
        self.dynamic_units.push(NativeDynamicUnit {
            compiled: compiled.clone(),
            cross_unit_global_names: cold_dynamic_units::dynamic_unit_cross_unit_global_names(
                &compiled,
                self.native_entries.keys().copied(),
            ),
            native_entries: self.native_entries.clone(),
            native_entry_signature_hashes,
            native_entry_signature_epochs,
            runtime_state,
            linked_functions,
            published_runtime_view: Box::default(),
        });
        self.prepare_trusted_literal_slots();
        self.current_dynamic_unit = Some(unit);
        debug_assert_eq!(
            self.current_native_execution_scope, 1,
            "root deployment attachment must precede nested native execution"
        );
        self.native_execution_scopes
            .first_mut()
            .expect("every native request publishes its root execution scope")
            .unit = Some(unit);
        if self.include_child {
            cold_dynamic_units::refresh_linked_function_records(self);
            return;
        }
        debug_assert_eq!(unit, 0, "immutable deployment must be the root native unit");
        self.deployment_functions = std::sync::Arc::clone(&deployment.function_exports);
        self.deployment_classes = std::sync::Arc::clone(&deployment.exported_classes);
    }

    fn class_is_visible(&self, normalized: &str) -> bool {
        self.deployment_classes.contains(normalized) || self.dynamic_classes.contains(normalized)
    }

    fn rebind_native_request_local_reference(
        &mut self,
        name: &str,
        encoded: i64,
    ) -> Result<(), String> {
        if self.native_reference_identity(encoded).is_none() {
            return Err(format!(
                "native request local ${name} was rebound to a non-reference value"
            ));
        }
        let published_functions = self.published_native_functions();
        let slot_indices = published_functions
            .into_iter()
            .filter_map(|function| {
                self.unit
                    .functions
                    .get(function.index())
                    .map(|definition| (function.index(), definition))
            })
            .flat_map(|(function, definition)| {
                definition
                    .locals
                    .iter()
                    .enumerate()
                    .filter_map(move |(local, _)| {
                        (native_request_local_name(definition, local) == Some(name))
                            .then_some((function, local))
                    })
            })
            .filter_map(|(function, local)| {
                self.trusted_request_local_function_offsets
                    .get(function)
                    .copied()
                    .and_then(|base| usize::try_from(base).ok())
                    .and_then(|base| base.checked_add(local))
                    .filter(|index| {
                        self.trusted_request_local_slots
                            .get(*index)
                            .is_some_and(|slot| slot.encoded != encoded)
                    })
            })
            .collect::<Vec<_>>();
        let map_changed = self.native_global_reference_handles.get(name).copied() != Some(encoded);
        let owner_count = slot_indices.len().saturating_add(usize::from(map_changed));
        let mut retained = 0_usize;
        for _ in 0..owner_count {
            if let Err(error) = self.retain(encoded) {
                for _ in 0..retained {
                    let _ = self.release(encoded);
                }
                return Err(error);
            }
            retained = retained.saturating_add(1);
        }

        let mut replaced = Vec::with_capacity(owner_count);
        if map_changed
            && let Some(previous) = self
                .native_global_reference_handles
                .insert(name.to_owned(), encoded)
        {
            replaced.push(previous);
        }
        for index in slot_indices {
            let previous = self.trusted_request_local_slots[index];
            self.trusted_request_local_slots[index] = php_jit::JitNativeRequestLocalSlot {
                encoded,
                state: php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED,
                reserved: 0,
            };
            if previous.state == php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED {
                replaced.push(previous.encoded);
            }
        }
        for previous in replaced {
            self.release(previous)?;
        }
        self.mark_roots_dirty(RootMutationReason::GlobalOrStatic);
        Ok(())
    }

    fn prepare_trusted_request_locals(&mut self) {
        self.ensure_native_global_references();
        let mut sites = Vec::new();
        for function in self.published_native_functions() {
            let Some(definition) = self.unit.functions.get(function.index()) else {
                continue;
            };
            sites.extend(
                definition
                    .locals
                    .iter()
                    .enumerate()
                    .filter_map(|(local, _)| {
                        native_request_local_name(definition, local)
                            .map(|name| (function.index(), local, name.to_owned()))
                    }),
            );
        }
        for (function, local, name) in sites {
            let Ok(encoded) = self.native_request_local_handle(&name) else {
                continue;
            };
            let Some(index) = self
                .trusted_request_local_function_offsets
                .get(function)
                .copied()
                .and_then(|base| usize::try_from(base).ok())
                .and_then(|base| base.checked_add(local))
            else {
                continue;
            };
            let Some(previous) = self.trusted_request_local_slots.get(index).copied() else {
                continue;
            };
            if previous.state == php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED
                && previous.encoded == encoded
            {
                continue;
            }
            if self.retain(encoded).is_err() {
                continue;
            }
            self.trusted_request_local_slots[index] = php_jit::JitNativeRequestLocalSlot {
                encoded,
                state: php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED,
                reserved: 0,
            };
            if previous.state == php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED {
                let _ = self.release(previous.encoded);
            }
        }
    }

    fn clear_trusted_request_locals(&mut self) {
        let mut values = Vec::new();
        for range in self.published_request_local_ranges() {
            values.extend(
                self.trusted_request_local_slots[range]
                    .iter_mut()
                    .filter_map(|slot| {
                        (slot.state == php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED).then(|| {
                            let encoded = slot.encoded;
                            *slot = php_jit::JitNativeRequestLocalSlot::default();
                            encoded
                        })
                    }),
            );
        }
        for encoded in values {
            let _ = self.release_if_live(encoded);
        }
    }

    fn publish_trusted_static_local_reference(
        &mut self,
        function: u32,
        local: u32,
        encoded: i64,
    ) -> Result<(), String> {
        if encoded as u64 & php_jit::JIT_VALUE_RUNTIME_KIND_MASK
            != php_jit::JIT_VALUE_RUNTIME_REFERENCE_TAG
            || Self::direct_value_index(encoded).is_none()
        {
            return Err("native static local did not produce a direct reference".to_owned());
        }
        let Some(base) = self
            .trusted_property_function_offsets
            .get(function as usize)
            .copied()
            .and_then(|base| usize::try_from(base).ok())
        else {
            return Err("native static-local function index is missing".to_owned());
        };
        let instructions = self
            .prepared_continuation_instructions(php_ir::FunctionId::new(function))
            .ok_or_else(|| "native static-local function metadata is missing".to_owned())?;
        let sites = instructions
            .iter()
            .enumerate()
            .filter_map(|(continuation, instruction)| {
                matches!(
                    instruction.as_ref().map(|instruction| &instruction.kind),
                    Some(php_ir::InstructionKind::InitStaticLocal { local: candidate, .. })
                        if candidate.raw() == local
                )
                .then_some(base.saturating_add(continuation))
            })
            .collect::<Vec<_>>();
        for index in sites {
            let previous = self
                .trusted_static_local_slots
                .get(index)
                .copied()
                .ok_or_else(|| "native static-local continuation is missing".to_owned())?;
            if previous.state == php_jit::JIT_NATIVE_TRUSTED_STATIC_LOCAL_PUBLISHED
                && previous.encoded == encoded
            {
                continue;
            }
            self.retain(encoded)?;
            self.trusted_static_local_slots[index] = php_jit::JitNativeTrustedStaticLocalSlot {
                encoded,
                state: php_jit::JIT_NATIVE_TRUSTED_STATIC_LOCAL_PUBLISHED,
                reserved: 0,
            };
            if previous.state == php_jit::JIT_NATIVE_TRUSTED_STATIC_LOCAL_PUBLISHED {
                self.release(previous.encoded)?;
            }
        }
        Ok(())
    }

    fn direct_value_index(encoded: i64) -> Option<usize> {
        let index = php_jit::jit_decode_runtime_value(encoded)?;
        let index = index.checked_sub(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE)? as usize;
        (index < php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY).then_some(index)
    }

    /// Clones the stable backing owner of one live direct object. The
    /// slot-parallel pointer arena is authoritative; no object-value HashMap
    /// participates in ordinary lookup.
    #[allow(unsafe_code)]
    fn direct_object(&self, index: usize) -> Option<php_runtime::api::ObjectRef> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0 || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT {
            return None;
        }
        self.direct_object_owner(index)
    }

    /// Clone the backing owner while a direct object is being retired.  At
    /// that point its native refcount is already zero, but the parallel owner
    /// remains valid until the descriptor and owner are reclaimed together.
    #[allow(unsafe_code)]
    fn direct_object_owner(&self, index: usize) -> Option<php_runtime::api::ObjectRef> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT {
            return None;
        }
        let owner =
            *self.direct_object_owners.get(index)? as usize as *const php_runtime::api::ObjectRef;
        // SAFETY: encode/publish stores a Box<ObjectRef> before exposing the
        // descriptor, and release clears the pointer only after refcount zero.
        unsafe { owner.as_ref().cloned() }
    }

    /// Clones the stable resource capability owned directly by one live
    /// native slot. No `Value` or cold handle lookup participates.
    #[allow(unsafe_code)]
    fn direct_resource(&self, index: usize) -> Option<php_runtime::api::ResourceRef> {
        let slot = *self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_RESOURCE
            || slot.flags != php_jit::JIT_NATIVE_DIRECT_RESOURCE_ABI_VERSION
            || slot.aux == 0
        {
            return None;
        }
        let owner = slot.aux as usize as *const php_runtime::api::ResourceRef;
        // SAFETY: publication installs exactly one boxed ResourceRef before
        // exposing the slot, and final release reclaims it exactly once.
        unsafe { owner.as_ref().cloned() }
    }

    /// Resolves a resource operand without crossing the Rust `Value` plane.
    /// Direct references are transparent to by-value builtin parameters.
    fn native_resource(&self, encoded: i64) -> Option<php_runtime::api::ResourceRef> {
        let encoded = self.dereference_direct_encoding(encoded);
        if let Some(index) = Self::direct_value_index(encoded) {
            return self.direct_resource(index);
        }
        None
    }

    /// Borrows the authoritative native callable view. The pointer is stable
    /// until the final encoded owner is released; no Rust dispatch enum or
    /// parallel runtime-value mirror participates.
    #[allow(unsafe_code)]
    fn direct_prepared_callable_view(
        &self,
        index: usize,
    ) -> Option<&php_jit::JitNativePreparedCallableView> {
        let slot = *self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE
            || slot.flags != php_jit::JIT_NATIVE_PREPARED_CALLABLE_ABI_VERSION
        {
            return None;
        }
        let owner = slot.aux as usize as *const NativePreparedCallableOwner;
        // SAFETY: publication installs exactly one boxed record before the
        // descriptor becomes visible, and final release reclaims both.
        unsafe { owner.as_ref().map(|owner| &owner.native_view) }
    }

    /// Baseline-only Closure metadata. Generated and exact code consumes the
    /// native view and never reaches this compatibility payload.
    #[allow(unsafe_code)]
    fn direct_prepared_closure(&self, index: usize) -> Option<&NativePreparedClosure> {
        let slot = *self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE
            || slot.flags != php_jit::JIT_NATIVE_PREPARED_CALLABLE_ABI_VERSION
        {
            return None;
        }
        let owner = slot.aux as usize as *const NativePreparedCallableOwner;
        // SAFETY: the validated owner remains live for this shared request
        // borrow. Only Closure-kind records populate the cold payload.
        unsafe { owner.as_ref()?.cold_closure.as_ref() }
    }

    #[allow(unsafe_code)]
    fn direct_prepared_closure_mut(&mut self, index: usize) -> Option<&mut NativePreparedClosure> {
        let slot = *self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE
            || slot.flags != php_jit::JIT_NATIVE_PREPARED_CALLABLE_ABI_VERSION
        {
            return None;
        }
        let owner = slot.aux as usize as *mut NativePreparedCallableOwner;
        // SAFETY: mutation requires `&mut self`, so no competing owner borrow
        // can exist on this request thread.
        unsafe { owner.as_mut()?.cold_closure.as_mut() }
    }

    #[allow(unsafe_code)]
    fn native_callable_string(&self, bytes: u64, length: u32) -> Option<String> {
        if length == 0 {
            return Some(String::new());
        }
        let bytes = usize::try_from(bytes).ok()? as *const u8;
        // SAFETY: every non-empty range is backed by one immutable boxed byte
        // owner adjacent to the validated native view.
        let bytes = unsafe { std::slice::from_raw_parts(bytes, length as usize) };
        std::str::from_utf8(bytes).ok().map(str::to_owned)
    }

    #[allow(unsafe_code)]
    fn fiber_record(&self, index: usize) -> Option<&NativeDirectFiber> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || !matches!(
                slot.kind,
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FIBER
                    | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_FIBER
            )
            || slot.flags != php_jit::JIT_NATIVE_DIRECT_FIBER_ABI_VERSION
        {
            return None;
        }
        let owner = slot.aux as usize as *const NativeDirectFiber;
        // SAFETY: direct Fiber publication owns one boxed record until the
        // slot's final encoded owner is released.
        unsafe { owner.as_ref() }
    }

    fn direct_fiber(&self, index: usize) -> Option<&NativeDirectFiber> {
        (self.direct_value_slots.get(index)?.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FIBER)
            .then(|| self.fiber_record(index))
            .flatten()
    }

    #[allow(unsafe_code)]
    fn direct_fiber_mut(&mut self, index: usize) -> Option<&mut NativeDirectFiber> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FIBER
            || slot.flags != php_jit::JIT_NATIVE_DIRECT_FIBER_ABI_VERSION
        {
            return None;
        }
        let owner = slot.aux as usize as *mut NativeDirectFiber;
        // SAFETY: `&mut self` excludes a competing record borrow on the
        // request thread.
        unsafe { owner.as_mut() }
    }

    fn direct_fiber_index(&self, encoded: i64) -> Option<usize> {
        let index = Self::direct_value_index(encoded)?;
        self.direct_fiber(index).map(|_| index)
    }

    #[allow(unsafe_code)]
    fn direct_generator(&self, index: usize) -> Option<&NativeDirectGenerator> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || !matches!(
                slot.kind,
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_GENERATOR
                    | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_GENERATOR
            )
            || slot.flags != php_jit::JIT_NATIVE_DIRECT_GENERATOR_ABI_VERSION
        {
            return None;
        }
        let owner = slot.aux as usize as *const NativeDirectGenerator;
        // SAFETY: publication installs one boxed activation before exposing
        // the slot, and final release reclaims both on the request thread.
        unsafe { owner.as_ref() }
    }

    #[allow(unsafe_code)]
    fn direct_generator_mut(&mut self, index: usize) -> Option<&mut NativeDirectGenerator> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || !matches!(
                slot.kind,
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_GENERATOR
                    | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_GENERATOR
            )
            || slot.flags != php_jit::JIT_NATIVE_DIRECT_GENERATOR_ABI_VERSION
        {
            return None;
        }
        let owner = slot.aux as usize as *mut NativeDirectGenerator;
        // SAFETY: `&mut self` excludes a competing activation borrow.
        unsafe { owner.as_mut() }
    }

    fn direct_generator_index(&self, encoded: i64) -> Option<usize> {
        let index = Self::direct_value_index(encoded)?;
        self.direct_generator(index).map(|_| index)
    }

    fn native_fiber_state(&self, encoded: i64) -> Option<php_runtime::api::FiberState> {
        let index = self.direct_fiber_index(encoded)?;
        self.direct_fiber(index).map(|fiber| fiber.state)
    }

    fn native_fiber_callable(&self, encoded: i64) -> Option<i64> {
        let index = self.direct_fiber_index(encoded)?;
        self.direct_fiber(index).map(|fiber| fiber.callable)
    }

    fn native_fiber_return_value(&self, encoded: i64) -> Option<Option<i64>> {
        let index = self.direct_fiber_index(encoded)?;
        self.direct_fiber(index).map(|fiber| fiber.return_value)
    }

    fn set_native_fiber_state(
        &mut self,
        encoded: i64,
        state: php_runtime::api::FiberState,
    ) -> Result<(), String> {
        let index = self
            .direct_fiber_index(encoded)
            .ok_or_else(|| "native Fiber has no direct record".to_owned())?;
        self.direct_fiber_mut(index)
            .ok_or_else(|| "native Fiber record disappeared".to_owned())?
            .state = state;
        Ok(())
    }

    fn terminate_native_fiber(
        &mut self,
        encoded: i64,
        return_value: Option<i64>,
    ) -> Result<(), String> {
        let index = self
            .direct_fiber_index(encoded)
            .ok_or_else(|| "native Fiber has no direct record".to_owned())?;
        let previous = {
            let fiber = self
                .direct_fiber_mut(index)
                .ok_or_else(|| "native Fiber record disappeared".to_owned())?;
            fiber.state = php_runtime::api::FiberState::Terminated;
            std::mem::replace(&mut fiber.return_value, return_value)
        };
        if let Some(previous) = previous {
            self.release(previous)?;
        }
        Ok(())
    }

    fn reserve_direct_value_slot(&mut self) -> Result<usize, String> {
        if *self.direct_value_free_head != php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE {
            let index = usize::try_from(*self.direct_value_free_head)
                .map_err(|_| "direct native free-list index overflow".to_owned())?;
            let slot = self
                .direct_value_slots
                .get(index)
                .ok_or_else(|| format!("direct native free-list slot {index} is missing"))?;
            *self.direct_value_free_head = u32::try_from(slot.payload)
                .map_err(|_| format!("direct native free-list link {} is invalid", slot.payload))?;
            *self.direct_value_reused_bytes = self
                .direct_value_reused_bytes
                .saturating_add(std::mem::size_of::<php_jit::JitNativeValueSlot>() as u64);
            self.cross_unit_stable_values.remove(&index);
            return Ok(index);
        }
        let index = usize::try_from(*self.direct_value_next)
            .map_err(|_| "direct native value index overflow".to_owned())?;
        if index >= self.direct_value_slots.len() {
            let mut live_by_kind = std::collections::BTreeMap::<u32, (usize, u64, u32)>::new();
            let mut dead = 0usize;
            for slot in self.direct_value_slots.get(..index).unwrap_or_default() {
                if slot.refcount == 0 {
                    dead = dead.saturating_add(1);
                    continue;
                }
                let entry = live_by_kind.entry(slot.kind).or_default();
                entry.0 = entry.0.saturating_add(1);
                entry.1 = entry.1.saturating_add(u64::from(slot.refcount));
                entry.2 = entry.2.max(slot.refcount);
            }
            return Err(format!(
                "direct native value arena exhausted at {} slots (dead={dead}, live_by_kind={live_by_kind:?})",
                index.saturating_add(1),
            ));
        }
        *self.direct_value_next = u32::try_from(index + 1)
            .map_err(|_| "direct native value index overflow".to_owned())?;
        self.cross_unit_stable_values.remove(&index);
        Ok(index)
    }

    fn encode_direct_slot_index(index: usize, tag: u64) -> Result<i64, String> {
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .ok_or_else(|| "direct native value handle overflow".to_owned())?;
        Ok(php_jit::jit_encode_typed_runtime_value(runtime_index, tag))
    }

    fn publish_cold_iterator(&mut self, iterator: NativeColdIterator) -> Result<i64, String> {
        let index = self.reserve_direct_value_slot()?;
        let direct_view = match &iterator {
            NativeColdIterator::Array(iterator) => iterator
                .direct
                .as_ref()
                .map(|direct| std::ptr::from_ref(direct.view.as_ref()) as usize as u64),
            NativeColdIterator::Object(_)
            | NativeColdIterator::Snapshot(_)
            | NativeColdIterator::LiveArray(_)
            | NativeColdIterator::User(_)
            | NativeColdIterator::Generator(_) => None,
        };
        let owner = Box::into_raw(Box::new(iterator));
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: if direct_view.is_some() {
                php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
            } else {
                php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR
            },
            flags: if direct_view.is_some() {
                php_jit::JIT_NATIVE_FOREACH_VIEW_ABI_VERSION
            } else {
                php_jit::JIT_NATIVE_COLD_ITERATOR_ABI_VERSION
            },
            payload: direct_view.unwrap_or(0),
            aux: owner as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
        Self::encode_direct_slot_index(index, php_jit::JIT_VALUE_RUNTIME_ITERATOR_TAG)
    }

    fn cold_iterator(&self, index: usize) -> Option<&NativeColdIterator> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || !matches!(
                slot.kind,
                php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
                    | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR
            )
            || slot.aux == 0
        {
            return None;
        }
        // SAFETY: publication owns exactly one boxed iterator through `aux`;
        // final direct-slot release reclaims it after all users have stopped.
        #[allow(unsafe_code)]
        unsafe {
            (slot.aux as usize as *const NativeColdIterator).as_ref()
        }
    }

    fn cold_iterator_mut(&mut self, index: usize) -> Option<&mut NativeColdIterator> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || !matches!(
                slot.kind,
                php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
                    | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR
            )
            || slot.aux == 0
        {
            return None;
        }
        // SAFETY: request execution is synchronous and this mutable borrow is
        // the only access to the request-owned iterator record.
        #[allow(unsafe_code)]
        unsafe {
            (slot.aux as usize as *mut NativeColdIterator).as_mut()
        }
    }

    fn publish_cold_generator(
        &mut self,
        generator: php_runtime::api::GeneratorRef,
    ) -> Result<i64, String> {
        let index = self.reserve_direct_value_slot()?;
        let id = generator.id();
        let owner = Box::into_raw(Box::new(generator));
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_COLD_GENERATOR,
            flags: php_jit::JIT_NATIVE_COLD_GENERATOR_ABI_VERSION,
            payload: id,
            aux: owner as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
        self.baseline_values
            .direct_generator_handles
            .insert(id, index as u32);
        Self::encode_direct_slot_index(index, php_jit::JIT_VALUE_RUNTIME_GENERATOR_TAG)
    }

    fn cold_generator(&self, index: usize) -> Option<&php_runtime::api::GeneratorRef> {
        let slot = self.direct_value_slots.get(index)?;
        if slot.refcount == 0
            || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_COLD_GENERATOR
            || slot.flags != php_jit::JIT_NATIVE_COLD_GENERATOR_ABI_VERSION
            || slot.aux == 0
        {
            return None;
        }
        // SAFETY: the direct slot owns this boxed GeneratorRef until final
        // release and request execution is synchronous.
        #[allow(unsafe_code)]
        unsafe {
            (slot.aux as usize as *const php_runtime::api::GeneratorRef).as_ref()
        }
    }

    /// Publishes a PHP string directly into the authoritative request-owned
    /// native byte/value plane. The Rust `PhpString` is consumed at this
    /// boundary and is not mirrored or retained in a second identity table.
    ///
    /// PHP strings have value semantics, so a cold `PhpString` owner does not
    /// need request-wide identity preservation. Equal immutable bytes may
    /// Publishes borrowed bytes directly as one native PHP string owner.
    /// Metadata/introspection producers already own stable byte slices and
    /// must not construct an intermediate `PhpString` merely to enter the
    /// authoritative string arena.
    #[track_caller]
    fn encode_native_string_bytes_owner(&mut self, bytes: &[u8]) -> Result<i64, String> {
        let hash = native_direct_string_hash(bytes);
        let existing = self
            .direct_string_interned_slots
            .get(&hash)
            .and_then(|indices| {
                indices.iter().copied().find(|index| {
                    let index = *index as usize;
                    self.direct_value_slots.get(index).is_some_and(|slot| {
                        slot.refcount != 0 && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_STRING
                    }) && self
                        .native_string_bytes(
                            (php_jit::JIT_VALUE_RUNTIME_STRING_TAG
                                | u64::from(
                                    index as u32 + php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE,
                                )) as i64,
                        )
                        .is_some_and(|candidate| candidate == bytes)
                })
            });
        if let Some(index) = existing {
            let runtime_index = index
                .checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE)
                .ok_or_else(|| "direct native string handle overflow".to_owned())?;
            let encoded = (php_jit::JIT_VALUE_RUNTIME_STRING_TAG | u64::from(runtime_index)) as i64;
            self.retain(encoded)?;
            return Ok(encoded);
        }
        let encoded = self.encode_direct_string_bytes(bytes)?;
        let index = Self::direct_value_index(encoded)
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| "direct native string index is invalid".to_owned())?;
        self.direct_string_interned_slots
            .entry(hash)
            .or_default()
            .push(index);
        Ok(encoded)
    }

    fn direct_string_capacity(length: usize) -> Result<usize, String> {
        length
            .max(php_jit::JIT_NATIVE_DIRECT_STRING_MIN_CAPACITY as usize)
            .checked_next_power_of_two()
            .ok_or_else(|| "direct native string capacity overflow".to_owned())
    }

    fn reserve_direct_string_bytes(&mut self, length: usize) -> Result<(usize, usize), String> {
        let capacity = Self::direct_string_capacity(length)?;
        let bucket = capacity.trailing_zeros() as usize;
        let head = self.direct_string_free_heads[bucket];
        if head != php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE {
            let start = head as usize;
            let next_bytes: [u8; 4] = self
                .direct_string_bytes
                .get(start..start + 4)
                .ok_or_else(|| "direct native string free-list entry is missing".to_owned())?
                .try_into()
                .expect("four-byte string free-list header");
            self.direct_string_free_heads[bucket] = u32::from_ne_bytes(next_bytes);
            *self.direct_string_reused_bytes = self
                .direct_string_reused_bytes
                .saturating_add(capacity as u64);
            return Ok((start, capacity));
        }
        let start = usize::try_from(*self.direct_string_next)
            .map_err(|_| "direct native string offset overflow".to_owned())?;
        let end = start
            .checked_add(capacity)
            .ok_or_else(|| "direct native string range overflow".to_owned())?;
        if end > self.direct_string_bytes.len() {
            return Err(format!(
                "direct native string arena exhausted at {end} bytes (next={start}, requested={capacity})"
            ));
        }
        *self.direct_string_next =
            u32::try_from(end).map_err(|_| "direct native string offset overflow".to_owned())?;
        Ok((start, capacity))
    }

    fn free_direct_string_bytes(&mut self, start: usize, capacity: usize) {
        if capacity < php_jit::JIT_NATIVE_DIRECT_STRING_MIN_CAPACITY as usize
            || !capacity.is_power_of_two()
        {
            return;
        }
        let bucket = capacity.trailing_zeros() as usize;
        let Some(head) = self.direct_string_free_heads.get_mut(bucket) else {
            return;
        };
        let Some(bytes) = self.direct_string_bytes.get_mut(start..start + 4) else {
            return;
        };
        bytes.copy_from_slice(&head.to_ne_bytes());
        *head = u32::try_from(start).unwrap_or(php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE);
    }

    fn encode_direct_string_bytes(&mut self, bytes: &[u8]) -> Result<i64, String> {
        let (start, capacity) = self.reserve_direct_string_bytes(bytes.len())?;
        let end = start + bytes.len();
        let index = match self.reserve_direct_value_slot() {
            Ok(index) => index,
            Err(error) => {
                self.free_direct_string_bytes(start, capacity);
                return Err(error);
            }
        };
        let runtime_index = match u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
        {
            Some(runtime_index) => runtime_index,
            None => {
                self.direct_value_slots[index] = php_jit::JitNativeValueSlot::default();
                self.free_direct_string_bytes(start, capacity);
                return Err("direct native value handle overflow".to_owned());
            }
        };
        self.direct_string_bytes[start..end].copy_from_slice(bytes);
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
            flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
            reserved: php_jit::jit_native_direct_string_reserved(
                u32::try_from(capacity).unwrap_or(u32::MAX),
                bytes == b"0",
            ),
            payload: bytes.len() as u64,
            aux: self.direct_string_bytes[start..].as_ptr() as usize as u64,
        };
        Ok((php_jit::JIT_VALUE_RUNTIME_STRING_TAG | u64::from(runtime_index)) as i64)
    }

    /// Convert a unit-scoped literal to its request-wide native encoding at a
    /// cross-unit call boundary without reconstructing a Rust `Value`.
    fn stabilize_active_unit_constant(&mut self, index: u32) -> Result<i64, String> {
        let constant = self
            .unit
            .constants
            .get(index as usize)
            .cloned()
            .ok_or_else(|| format!("native constant {index} is missing from the active unit"))?;
        match constant {
            php_ir::IrConstant::Null => Ok(php_jit::jit_encode_constant(u32::MAX)),
            php_ir::IrConstant::Bool(false) => {
                Ok(php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE))
            }
            php_ir::IrConstant::Bool(true) => {
                Ok(php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE))
            }
            php_ir::IrConstant::Int(value) => self.encode_native_int(value),
            php_ir::IrConstant::Float(value) => {
                self.encode_native_float_owner(php_runtime::api::FloatValue::from_f64(value))
            }
            php_ir::IrConstant::String(value) => self.encode_direct_string_bytes(value.as_bytes()),
            php_ir::IrConstant::StringBytes(value) => self.encode_direct_string_bytes(&value),
            constant @ php_ir::IrConstant::Array(_) => {
                self.encode_native_ir_constant_owned(&constant)
            }
            constant @ (php_ir::IrConstant::NamedConstant(_)
            | php_ir::IrConstant::ClassConstant { .. }) => {
                self.encode_native_ir_constant_owned(&constant)
            }
        }
    }

    /// Publishes a parameter/default constant directly into the native value
    /// plane.  Scalar and array defaults are common call-frame data and must
    /// not be constructed as a temporary Rust `Value` merely because the
    /// caller omitted an argument.
    fn encode_native_ir_constant_owned(
        &mut self,
        constant: &php_ir::IrConstant,
    ) -> Result<i64, String> {
        self.encode_native_ir_constant_owned_at_depth(constant, 0)
    }

    fn encode_native_ir_constant_owned_at_depth(
        &mut self,
        constant: &php_ir::IrConstant,
        depth: usize,
    ) -> Result<i64, String> {
        if depth > 32 {
            return Err("native constant resolution exceeded its recursion limit".to_owned());
        }
        match constant {
            php_ir::IrConstant::Null => Ok(php_jit::jit_encode_constant(u32::MAX)),
            php_ir::IrConstant::Bool(false) => {
                Ok(php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE))
            }
            php_ir::IrConstant::Bool(true) => {
                Ok(php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE))
            }
            php_ir::IrConstant::Int(value) => self.encode_native_int(*value),
            php_ir::IrConstant::Float(value) => {
                self.encode_native_float_owner(php_runtime::api::FloatValue::from_f64(*value))
            }
            php_ir::IrConstant::String(value) => self.encode_direct_string_bytes(value.as_bytes()),
            php_ir::IrConstant::StringBytes(value) => self.encode_direct_string_bytes(value),
            php_ir::IrConstant::Array(source) => {
                let mut entries =
                    Vec::<php_jit::JitNativeDirectArrayEntry>::with_capacity(source.len());
                let mut next_index = None;
                for source_entry in source {
                    let value = match self
                        .encode_native_ir_constant_owned_at_depth(&source_entry.value, depth + 1)
                    {
                        Ok(value) => value,
                        Err(error) => {
                            for entry in entries {
                                let _ = self.release(entry.key);
                                let _ = self.release(entry.value);
                            }
                            return Err(error);
                        }
                    };
                    let key = match source_entry.key.as_ref() {
                        Some(key) => {
                            self.encode_native_constant_array_key_owned_at_depth(key, depth + 1)
                        }
                        None => {
                            let next = next_index.unwrap_or(0);
                            if next == i64::MAX
                                && entries.iter().any(|entry| {
                                    self.native_encoded_int(entry.key) == Some(i64::MAX)
                                })
                            {
                                Err(php_runtime::api::PHP_ARRAY_APPEND_OVERFLOW_MESSAGE.to_owned())
                            } else {
                                self.encode_native_int(next)
                            }
                        }
                    };
                    let key = match key {
                        Ok(key) => key,
                        Err(error) => {
                            let _ = self.release(value);
                            for entry in entries {
                                let _ = self.release(entry.key);
                                let _ = self.release(entry.value);
                            }
                            return Err(error);
                        }
                    };
                    if let Some(key_value) = self.native_encoded_int(key) {
                        let next = key_value.saturating_add(1);
                        if next_index.is_none_or(|current| next > current) {
                            next_index = Some(next);
                        }
                    }
                    if let Some(existing) = entries
                        .iter_mut()
                        .find(|entry| self.native_encoded_array_keys_equal(entry.key, key))
                    {
                        let _ = self.release(key);
                        let previous = std::mem::replace(&mut existing.value, value);
                        self.release(previous)?;
                    } else {
                        entries.push(php_jit::JitNativeDirectArrayEntry { key, value });
                    }
                }
                self.publish_owned_direct_array_entries(entries)
            }
            php_ir::IrConstant::NamedConstant(name) => {
                self.encode_named_runtime_constant_owned(name, depth + 1)
            }
            php_ir::IrConstant::ClassConstant {
                class_name,
                constant_name,
                ..
            } => self.encode_class_runtime_constant_owned(class_name, constant_name, depth + 1),
        }
    }

    /// Follows local and linked class declarations while retaining their
    /// encoded native representation. Visibility and autoload diagnostics
    /// remain on the explicit `FetchClassConstant` continuation.
    fn encode_class_runtime_constant_owned(
        &mut self,
        class_name: &str,
        constant_name: &str,
        depth: usize,
    ) -> Result<i64, String> {
        if depth > 32 {
            return Err("native constant resolution exceeded its recursion limit".to_owned());
        }
        let normalized = normalize_class_name(class_name);
        let local = self
            .unit
            .classes
            .iter()
            .find(|class| class.name == normalized)
            .and_then(|class| {
                class
                    .constants
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(constant_name))
            })
            .cloned();
        if let Some(entry) = local {
            if let Some(constant) = entry
                .value
                .and_then(|id| self.unit.constants.get(id.index()))
                .cloned()
            {
                return self.encode_native_ir_constant_owned_at_depth(&constant, depth + 1);
            }
            if let Some(reference) = entry.value_named_constant {
                for name in reference.names {
                    if let Ok(value) = self.encode_named_runtime_constant_owned(&name, depth + 1) {
                        return Ok(value);
                    }
                }
            }
            if let Some(reference) = entry.value_class_constant {
                return self.encode_class_runtime_constant_owned(
                    &reference.class_name,
                    &reference.constant_name,
                    depth + 1,
                );
            }
        }

        if let Some((unit, class)) = native_external_class_handle(self, &normalized) {
            let entry = class
                .constants
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(constant_name))
                .cloned();
            if let Some(entry) = entry {
                if let Some(constant) = entry
                    .value
                    .and_then(|id| {
                        self.dynamic_units
                            .get(unit)
                            .and_then(|package| package.compiled.unit().constants.get(id.index()))
                    })
                    .cloned()
                {
                    return self.encode_native_ir_constant_owned_at_depth(&constant, depth + 1);
                }
                if let Some(reference) = entry.value_named_constant {
                    for name in reference.names {
                        if let Ok(value) =
                            self.encode_named_runtime_constant_owned(&name, depth + 1)
                        {
                            return Ok(value);
                        }
                    }
                }
                if let Some(reference) = entry.value_class_constant {
                    return self.encode_class_runtime_constant_owned(
                        &reference.class_name,
                        &reference.constant_name,
                        depth + 1,
                    );
                }
            }
        }
        Err(format!("Undefined constant {class_name}::{constant_name}"))
    }

    fn encode_native_constant_array_key_owned_at_depth(
        &mut self,
        constant: &php_ir::IrConstant,
        depth: usize,
    ) -> Result<i64, String> {
        match constant {
            php_ir::IrConstant::Null => self.encode_direct_string_bytes(&[]),
            php_ir::IrConstant::Bool(value) => Ok(i64::from(*value)),
            php_ir::IrConstant::Int(value) => self.encode_native_int(*value),
            php_ir::IrConstant::Float(value) => Ok(*value as i64),
            php_ir::IrConstant::String(value) => {
                if let Some(key) = php_runtime::api::array_key_integer_bytes(value.as_bytes()) {
                    self.encode_native_int(key)
                } else {
                    self.encode_direct_string_bytes(value.as_bytes())
                }
            }
            php_ir::IrConstant::StringBytes(value) => {
                if let Some(key) = php_runtime::api::array_key_integer_bytes(value) {
                    self.encode_native_int(key)
                } else {
                    self.encode_direct_string_bytes(value)
                }
            }
            php_ir::IrConstant::Array(_) => Err("native constant array key is invalid".to_owned()),
            php_ir::IrConstant::NamedConstant(_) | php_ir::IrConstant::ClassConstant { .. } => {
                let encoded = self.encode_native_ir_constant_owned_at_depth(constant, depth + 1)?;
                match self.native_encoded_value_kind(encoded) {
                    Some(NativeEncodedValueKind::Null) => {
                        self.release(encoded)?;
                        self.encode_direct_string_bytes(&[])
                    }
                    Some(NativeEncodedValueKind::Bool(value)) => {
                        self.release(encoded)?;
                        self.encode_native_int(i64::from(value))
                    }
                    Some(NativeEncodedValueKind::Int) => Ok(encoded),
                    Some(NativeEncodedValueKind::Float) => {
                        let value = self.native_encoded_float(encoded).ok_or_else(|| {
                            "native constant float key lost its payload".to_owned()
                        })?;
                        self.release(encoded)?;
                        self.encode_native_int(php_runtime::api::php_float_to_int(value))
                    }
                    Some(NativeEncodedValueKind::String) => {
                        let bytes = self.native_string_name_bytes(encoded).ok_or_else(|| {
                            "native constant string key lost its bytes".to_owned()
                        })?;
                        let integer_key = php_runtime::api::array_key_integer_bytes(&bytes);
                        self.release(encoded)?;
                        if let Some(key) = integer_key {
                            self.encode_native_int(key)
                        } else {
                            self.encode_direct_string_bytes(&bytes)
                        }
                    }
                    _ => {
                        self.release(encoded)?;
                        Err("native constant array key is invalid".to_owned())
                    }
                }
            }
        }
    }

    fn native_encoded_array_keys_equal(&self, left: i64, right: i64) -> bool {
        let left_int = self.native_encoded_int(left).or_else(|| {
            self.native_string_bytes(left)
                .and_then(php_runtime::api::array_key_integer_bytes)
        });
        let right_int = self.native_encoded_int(right).or_else(|| {
            self.native_string_bytes(right)
                .and_then(php_runtime::api::array_key_integer_bytes)
        });
        match (left_int, right_int) {
            (Some(left), Some(right)) => left == right,
            (None, None) => self.native_string_bytes(left) == self.native_string_bytes(right),
            _ => false,
        }
    }

    fn native_encoded_matches_array_key(
        &self,
        encoded: i64,
        key: &php_runtime::api::ArrayKey,
    ) -> bool {
        match key {
            php_runtime::api::ArrayKey::Int(key) => {
                self.native_encoded_int(encoded).or_else(|| {
                    self.native_string_bytes(encoded)
                        .and_then(php_runtime::api::array_key_integer_bytes)
                }) == Some(*key)
            }
            php_runtime::api::ArrayKey::String(key) => {
                if let Some(key) = php_runtime::api::array_key_integer_bytes(key.as_bytes()) {
                    self.native_encoded_int(encoded) == Some(key)
                } else {
                    self.native_string_bytes(encoded)
                        .is_some_and(|bytes| bytes == key.as_bytes())
                }
            }
        }
    }

    fn encode_native_array_key_owned(
        &mut self,
        key: &php_runtime::api::ArrayKey,
    ) -> Result<i64, String> {
        match key {
            php_runtime::api::ArrayKey::Int(key) => self.encode_native_int(*key),
            php_runtime::api::ArrayKey::String(key) => {
                if let Some(key) = php_runtime::api::array_key_integer_bytes(key.as_bytes()) {
                    self.encode_native_int(key)
                } else {
                    self.encode_native_string_bytes_owner(key.as_bytes())
                }
            }
        }
    }

    /// Converts the two diagnostic-free PHP array-key families directly.
    /// Float/bool/null/object conversions remain at the semantic boundary
    /// because they may emit PHP-visible diagnostics.
    fn native_encoded_plain_array_key(&self, encoded: i64) -> Option<php_runtime::api::ArrayKey> {
        let encoded = self.dereference_direct_encoding(encoded);
        match self.native_encoded_value_kind(encoded)? {
            NativeEncodedValueKind::Int => self
                .native_encoded_int(encoded)
                .map(php_runtime::api::ArrayKey::Int),
            NativeEncodedValueKind::String => self
                .native_string_name_bytes(encoded)
                .map(php_runtime::api::ArrayKey::from_bytes),
            _ => None,
        }
    }

    /// Publishes one IEEE-754 scalar directly. The payload is authoritative
    /// and no cold value mirror is retained.
    fn encode_native_float_owner(
        &mut self,
        value: php_runtime::api::FloatValue,
    ) -> Result<i64, String> {
        let index = self.reserve_direct_value_slot()?;
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_FLOAT,
            payload: value.to_f64().to_bits(),
            ..php_jit::JitNativeValueSlot::default()
        };
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .ok_or_else(|| "direct native value handle overflow".to_owned())?;
        Ok((php_jit::JIT_VALUE_RUNTIME_FLOAT_TAG | u64::from(runtime_index)) as i64)
    }

    /// Keeps the full PHP integer domain on the authoritative native plane.
    /// Most integers remain immediate; only bit patterns overlapping a native
    /// handle namespace consume a direct slot.
    fn encode_native_int(&mut self, value: i64) -> Result<i64, String> {
        if php_jit::jit_decode_runtime_value(value).is_none()
            && php_jit::jit_decode_constant(value).is_none()
        {
            return Ok(value);
        }
        let index = self.reserve_direct_value_slot()?;
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_INT,
            flags: php_jit::JIT_NATIVE_DIRECT_INT_ABI_VERSION,
            payload: value as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .ok_or_else(|| "direct native integer handle overflow".to_owned())?;
        Ok(php_jit::jit_encode_runtime_value(runtime_index))
    }

    /// Publishes one opaque PHP resource capability directly. The slot owns
    /// ResourceRef identity and lifetime; ordinary calls never wrap it in a
    /// Rust `Value` or allocate a compatibility handle.
    fn encode_native_resource_owner(
        &mut self,
        resource: php_runtime::api::ResourceRef,
    ) -> Result<i64, String> {
        let resource_id = resource.id().get();
        if let Some(index) = self.direct_resource_handles.get(&resource_id).copied() {
            let slot = self
                .direct_value_slots
                .get_mut(index as usize)
                .filter(|slot| {
                    slot.refcount != 0
                        && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_RESOURCE
                        && slot.flags == php_jit::JIT_NATIVE_DIRECT_RESOURCE_ABI_VERSION
                        && slot.payload == resource_id
                })
                .ok_or_else(|| {
                    "direct native resource identity points at a dead slot".to_owned()
                })?;
            slot.refcount = slot
                .refcount
                .checked_add(1)
                .ok_or_else(|| "direct native resource refcount overflow".to_owned())?;
            let runtime_index = index
                .checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE)
                .ok_or_else(|| "direct native resource handle overflow".to_owned())?;
            return Ok(php_jit::jit_encode_typed_runtime_value(
                runtime_index,
                php_jit::JIT_VALUE_RUNTIME_RESOURCE_TAG,
            ));
        }

        let index = self.reserve_direct_value_slot()?;
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .ok_or_else(|| "direct native resource handle overflow".to_owned())?;
        let owner = Box::into_raw(Box::new(resource));
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            // The identity table owns one request-lifetime native reference
            // in addition to the encoded owner returned to the caller.
            refcount: 2,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_RESOURCE,
            flags: php_jit::JIT_NATIVE_DIRECT_RESOURCE_ABI_VERSION,
            payload: resource_id,
            aux: owner as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
        self.direct_resource_handles.insert(
            resource_id,
            u32::try_from(index).map_err(|_| "direct native resource index overflow".to_owned())?,
        );
        Ok(php_jit::jit_encode_typed_runtime_value(
            runtime_index,
            php_jit::JIT_VALUE_RUNTIME_RESOURCE_TAG,
        ))
    }

    /// Publishes object identity and PHP ownership in the direct plane. The
    /// slot-parallel stable owner supplies the backing identity needed at a
    /// cold boundary; declared values move into native slots immediately.
    #[track_caller]
    fn encode_native_object_owner(
        &mut self,
        object: php_runtime::api::ObjectRef,
    ) -> Result<i64, String> {
        let object_id = object.id();
        let existing = self
            .baseline_values
            .direct_object_handles
            .get(&object_id)
            .copied()
            .or_else(|| {
                let used = usize::try_from(*self.direct_value_next).ok()?;
                (0..used)
                    .find(|index| {
                        self.direct_object(*index)
                            .is_some_and(|candidate| candidate.id() == object_id)
                    })
                    .and_then(|index| u32::try_from(index).ok())
            });
        if let Some(index) = existing {
            let slot = self
                .direct_value_slots
                .get_mut(index as usize)
                .filter(|slot| {
                    slot.refcount != 0 && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
                })
                .ok_or_else(|| "direct native object identity points at a dead slot".to_owned())?;
            slot.refcount = slot
                .refcount
                .checked_add(1)
                .ok_or_else(|| "direct native object refcount overflow".to_owned())?;
            self.baseline_values
                .direct_object_handles
                .insert(object_id, index);
            let runtime_index = index
                .checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE)
                .ok_or_else(|| "direct native object handle overflow".to_owned())?;
            if !php_jit::jit_native_object_property_view_is_published(
                self.direct_value_slots[index as usize].flags,
            ) && let Err(error) = self.promote_direct_object_property_slots(index as usize)
            {
                let _ = self.release_direct_value_index(index as usize);
                return Err(error);
            }
            return Ok((php_jit::JIT_VALUE_RUNTIME_OBJECT_TAG | u64::from(runtime_index)) as i64);
        }
        let index = self.reserve_direct_value_slot()?;
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .ok_or_else(|| "direct native value handle overflow".to_owned())?;
        let shutdown_handle = self
            .shutdown_destructor_queue
            .is_some()
            .then(|| object.weak_handle());
        let owner = Box::into_raw(Box::new(object));
        self.direct_object_owners[index] = owner as usize as u64;
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT,
            payload: object_id,
            ..php_jit::JitNativeValueSlot::default()
        };
        self.baseline_values.direct_object_handles.insert(
            object_id,
            u32::try_from(index).map_err(|_| "direct native object index overflow".to_owned())?,
        );
        if let Err(error) = self.promote_direct_object_property_slots(index) {
            let _ = self.release_direct_value_index(index);
            return Err(error);
        }
        if let Some(handle) = shutdown_handle {
            self.shutdown_destructor_queue
                .as_mut()
                .expect("shutdown destructor queue disappeared during object publication")
                .push(handle);
        }
        Ok((php_jit::JIT_VALUE_RUNTIME_OBJECT_TAG | u64::from(runtime_index)) as i64)
    }

    /// Removes authoritative native properties from an object that is about
    /// to die without running user code. The encoded children are returned to
    /// the central direct release walk; no Rust `Value` is reconstructed.
    fn take_direct_object_children(&mut self, index: usize) -> Result<Vec<i64>, String> {
        let object = self
            .direct_object_owner(index)
            .ok_or_else(|| format!("direct native object {index} has no stable owner"))?;
        let descriptor = *self
            .direct_value_slots
            .get(index)
            .ok_or_else(|| format!("direct native object {index} slot is missing"))?;
        if !php_jit::jit_native_object_property_view_is_published(descriptor.flags) {
            return Ok(Vec::new());
        }
        let (slots, dynamic) = object
            .take_native_property_slots(descriptor.payload)
            .ok_or_else(|| format!("direct native object {index} lost its property slots"))?;
        let mut children: Vec<i64> = slots
            .iter()
            .filter(|slot| slot.initialized != 0)
            .map(|slot| slot.value)
            .collect();
        children.extend(
            dynamic
                .values()
                .filter(|cell| cell.slot.initialized != 0)
                .map(|cell| cell.slot.value),
        );
        self.direct_value_slots[index].flags = 0;
        self.direct_value_slots[index].reserved = 0;
        self.direct_value_slots[index].payload = object.id();
        self.direct_value_slots[index].aux = 0;
        Ok(children)
    }

    fn reserve_direct_array_entries(&mut self, length: usize) -> Result<(usize, usize), String> {
        // Rust-side publication normally installs a completed immutable/COW
        // snapshot. Reserving the CLIF construction headroom for every such
        // array made hundreds of thousands of one- and two-element values each
        // pin eight entries. Keep one cell so a freed empty range can carry
        // its intrusive free-list link; mutation grows the range on demand.
        // Newly constructed CLIF arrays still use
        // `JIT_NATIVE_DIRECT_ARRAY_INITIAL_CAPACITY` directly in generated
        // code and therefore retain their append headroom.
        let capacity = length.max(1).next_power_of_two();
        let bucket = capacity.trailing_zeros() as usize;
        let head = self.direct_array_free_heads[bucket];
        if head != php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE {
            let start = head as usize;
            let next = self
                .direct_array_entries
                .get(start)
                .map(|entry| entry.key as u32)
                .ok_or_else(|| "direct native array free-list entry is missing".to_owned())?;
            self.direct_array_free_heads[bucket] = next;
            *self.direct_array_reused_bytes = self.direct_array_reused_bytes.saturating_add(
                capacity.saturating_mul(std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>())
                    as u64,
            );
            return Ok((start, capacity));
        }
        let start = usize::try_from(*self.direct_array_next)
            .map_err(|_| "direct native array entry index overflow".to_owned())?;
        let end = start
            .checked_add(capacity)
            .ok_or_else(|| "direct native array entry range overflow".to_owned())?;
        if end > self.direct_array_entries.len() {
            let reusable = self
                .direct_array_free_heads
                .iter()
                .filter(|head| **head != php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE)
                .count();
            let (live_arrays, live_entries, live_capacity, live_refs) = self
                .direct_value_slots
                .get(..usize::try_from(*self.direct_value_next).unwrap_or(0))
                .unwrap_or_default()
                .iter()
                .filter(|slot| {
                    slot.refcount != 0 && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY
                })
                .fold((0usize, 0u64, 0u64, 0u64), |totals, slot| {
                    (
                        totals.0.saturating_add(1),
                        totals.1.saturating_add(slot.payload),
                        totals.2.saturating_add(u64::from(slot.reserved)),
                        totals.3.saturating_add(u64::from(slot.refcount)),
                    )
                });
            let direct_used = usize::try_from(*self.direct_value_next).unwrap_or(0);
            let mut referenced = vec![false; direct_used];
            let direct_base = self.direct_array_entries.as_ptr() as usize;
            let entry_size = std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>();
            for slot in self
                .direct_value_slots
                .get(..direct_used)
                .unwrap_or_default()
                .iter()
                .filter(|slot| {
                    slot.refcount != 0 && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY
                })
            {
                let start = usize::try_from(slot.aux)
                    .unwrap_or(direct_base)
                    .saturating_sub(direct_base)
                    / entry_size;
                let length = usize::try_from(slot.payload).unwrap_or(0);
                for entry in self
                    .direct_array_entries
                    .get(start..start.saturating_add(length))
                    .unwrap_or_default()
                {
                    for encoded in [entry.key, entry.value] {
                        if let Some(index) = Self::direct_value_index(encoded)
                            && index < referenced.len()
                        {
                            referenced[index] = true;
                        }
                    }
                }
            }
            let unreferenced_arrays = self
                .direct_value_slots
                .get(..direct_used)
                .unwrap_or_default()
                .iter()
                .enumerate()
                .filter(|(index, slot)| {
                    slot.refcount != 0
                        && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY
                        && !referenced[*index]
                })
                .count();
            return Err(format!(
                "direct native array arena exhausted at {end} entries (next={start}, requested={capacity}, reusable_buckets={reusable}, live_arrays={live_arrays}, live_entries={live_entries}, live_capacity={live_capacity}, live_refs={live_refs}, unreferenced_arrays={unreferenced_arrays})"
            ));
        }
        *self.direct_array_next = u32::try_from(end)
            .map_err(|_| "direct native array entry index overflow".to_owned())?;
        Ok((start, capacity))
    }

    fn free_direct_array_entries(&mut self, start: usize, capacity: usize) {
        if capacity == 0 {
            return;
        }
        if !capacity.is_power_of_two() {
            return;
        }
        let Ok(start_u32) = u32::try_from(start) else {
            return;
        };
        let bucket = capacity.trailing_zeros() as usize;
        if bucket >= self.direct_array_free_heads.len() || start >= self.direct_array_entries.len()
        {
            return;
        }
        let previous = self.direct_array_free_heads[bucket];
        self.direct_array_entries[start].key = i64::from(previous);
        self.direct_array_entries[start].value = 0;
        self.direct_array_free_heads[bucket] = start_u32;
    }

    fn encode_prepared_callable(
        &mut self,
        callable: Box<php_runtime::api::CallableValue>,
    ) -> Result<i64, String> {
        if matches!(
            callable.as_ref(),
            php_runtime::api::CallableValue::Closure(_)
        ) {
            return self.encode_prepared_closure(*callable);
        }
        let owner = match *callable {
            php_runtime::api::CallableValue::UserFunction { name } => {
                let normalized = name.trim_start_matches('\\').to_ascii_lowercase();
                let resolved_function = self.compiled.lookup_function(&normalized).or_else(|| {
                    normalized
                        .rsplit_once('\\')
                        .and_then(|(_, basename)| self.compiled.lookup_function(basename))
                });
                let resolved_function = resolved_function.and_then(|function| {
                    native_fixed_callable_plan(&self.compiled, function, false)
                });
                NativePreparedCallableOwner::user_function(
                    name.into_bytes().into_boxed_slice(),
                    resolved_function,
                )
            }
            php_runtime::api::CallableValue::InternalBuiltin { name } => {
                NativePreparedCallableOwner::internal_builtin(name.into_bytes().into_boxed_slice())
            }
            php_runtime::api::CallableValue::BoundMethod {
                target,
                method,
                scope,
            } => {
                let method = method.into_bytes().into_boxed_slice();
                let scope = scope.map(|scope| scope.into_bytes().into_boxed_slice());
                match target {
                    php_runtime::api::CallableMethodTarget::Object(object) => {
                        NativePreparedCallableOwner::bound_object(
                            self.encode_native_object_owner(object)?,
                            method,
                            scope,
                            None,
                        )
                    }
                    php_runtime::api::CallableMethodTarget::Class(class) => {
                        NativePreparedCallableOwner::bound_class(
                            class.into_bytes().into_boxed_slice(),
                            method,
                            scope,
                            None,
                        )
                    }
                }
            }
            php_runtime::api::CallableValue::MethodPlaceholder { target } => {
                NativePreparedCallableOwner::method_placeholder(
                    target.into_bytes().into_boxed_slice(),
                )
            }
            php_runtime::api::CallableValue::UnresolvedDynamic { target } => {
                NativePreparedCallableOwner::unresolved_dynamic(
                    target.into_bytes().into_boxed_slice(),
                )
            }
            php_runtime::api::CallableValue::Closure(_) => unreachable!(),
        };
        let index = match self.reserve_direct_value_slot() {
            Ok(index) => index,
            Err(error) => {
                if owner.native_view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD {
                    let _ = self.release(owner.native_view.receiver);
                }
                return Err(error);
            }
        };
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .expect("direct callable index is bounded by the native value arena");
        let owner = Box::into_raw(Box::new(owner));
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE,
            flags: php_jit::JIT_NATIVE_PREPARED_CALLABLE_ABI_VERSION,
            aux: owner as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
        Ok(php_jit::jit_encode_typed_runtime_value(
            runtime_index,
            php_jit::JIT_VALUE_RUNTIME_CALLABLE_TAG,
        ))
    }

    /// Gives an encoded value one additional request-arena owner without
    /// decoding or reconstructing it. Direct values are authoritative;
    /// `None` is reserved for proxies/iterators whose cold semantics require
    /// an explicit operation.
    fn duplicate_authoritative_native_value(
        &mut self,
        encoded: i64,
    ) -> Result<Option<i64>, String> {
        if self.is_globals_proxy(encoded) {
            return Ok(None);
        }
        if let Some(index) = Self::direct_value_index(encoded) {
            if self.direct_value_slots.get(index).is_some_and(|slot| {
                matches!(
                    slot.kind,
                    php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
                        | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR
                )
            }) {
                return Ok(None);
            }
            self.retain(encoded)?;
            return Ok(Some(encoded));
        }
        if let Some(index) = php_jit::jit_decode_runtime_value(encoded) {
            return Err(format!(
                "native runtime value {index} is outside the authoritative direct slot plane"
            ));
        }
        if let Some(constant) = php_jit::jit_decode_constant(encoded)
            && !matches!(
                constant,
                u32::MAX
                    | php_jit::JIT_VALUE_UNINITIALIZED
                    | php_jit::JIT_VALUE_FALSE
                    | php_jit::JIT_VALUE_TRUE
            )
        {
            return self.stabilize_active_unit_constant(constant).map(Some);
        }
        Ok(Some(encoded))
    }

    fn prepared_closure_invocation(
        &self,
        encoded: i64,
    ) -> Option<(
        php_runtime::api::ClosurePayload,
        Option<i64>,
        smallvec::SmallVec<[i64; 8]>,
    )> {
        let index = Self::direct_value_index(encoded)?;
        let view = self.direct_prepared_callable_view(index)?;
        if view.kind != php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE {
            return None;
        }
        let prepared = self.direct_prepared_closure(index)?;
        if prepared.closure.id != self.direct_value_slots.get(index)?.payload
            || prepared.capture_descriptors.len() != prepared.captures.len()
        {
            return None;
        }
        Some((
            prepared.closure.clone(),
            prepared.implicit_this,
            smallvec::SmallVec::from_slice(&prepared.captures),
        ))
    }

    fn rebind_prepared_closure(
        &mut self,
        encoded: i64,
        new_this: Option<i64>,
        new_scope: Option<i64>,
        api: &str,
    ) -> Option<Result<i64, String>> {
        let index = Self::direct_value_index(self.dereference_direct_encoding(encoded))?;
        let (closure, capture_descriptors, captures) = {
            if self.direct_prepared_callable_view(index)?.kind
                != php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE
            {
                return None;
            }
            let prepared = self.direct_prepared_closure(index)?;
            (
                prepared.closure.clone(),
                prepared.capture_descriptors.clone(),
                prepared.captures.clone(),
            )
        };
        let mut retained = Vec::new();
        let result = (|| {
            let new_this = match new_this {
                Some(value) => Some(
                    self.duplicate_authoritative_dereferenced_native_value(value)?
                        .ok_or_else(|| {
                            format!("{api}() new object crossed from baseline storage")
                        })?,
                ),
                None => None,
            };
            let bound_this = match new_this {
                None => None,
                Some(value)
                    if self.native_encoded_value_kind(value)
                        == Some(NativeEncodedValueKind::Null) =>
                {
                    self.release(value)?;
                    None
                }
                Some(value)
                    if self.native_encoded_value_kind(value)
                        == Some(NativeEncodedValueKind::Object)
                        && self.native_query_object(value).is_some() =>
                {
                    Some(value)
                }
                Some(value) => {
                    let actual = self.native_encoded_type_name(value);
                    self.release(value)?;
                    return Err(format!(
                        "{api}(): Argument #2 ($newThis) must be of type ?object, {} given",
                        actual
                    ));
                }
            };
            retained.extend(bound_this);

            let scope_value = match new_scope {
                Some(value) => Some(
                    self.duplicate_authoritative_dereferenced_native_value(value)?
                        .ok_or_else(|| format!("{api}() scope crossed from baseline storage"))?,
                ),
                None => None,
            };
            let scope_result: Result<Option<Arc<str>>, String> = match scope_value {
                Some(value)
                    if self.native_encoded_value_kind(value)
                        == Some(NativeEncodedValueKind::Null) =>
                {
                    Ok(None)
                }
                Some(value)
                    if self.native_encoded_value_kind(value)
                        == Some(NativeEncodedValueKind::Object) =>
                {
                    match self.native_query_object(value) {
                        Some(object) => Ok(Some(object.display_name().into())),
                        None => Err(format!(
                            "{api}() scope object crossed from baseline storage"
                        )),
                    }
                }
                Some(value)
                    if self.native_encoded_value_kind(value)
                        == Some(NativeEncodedValueKind::String) =>
                {
                    match self.native_string_name_bytes(value) {
                        Some(bytes) => {
                            let scope = String::from_utf8_lossy(&bytes).into_owned();
                            Ok((scope != "static").then(|| Arc::from(scope.as_str())))
                        }
                        None => Err(format!("{api}() scope string has no native bytes")),
                    }
                }
                Some(value) => Err(format!(
                    "{api}(): Argument #3 ($newScope) must be of type object|string|null, {} given",
                    self.native_encoded_type_name(value)
                )),
                None => Ok(bound_this
                    .and_then(|value| self.native_query_object(value))
                    .map(|object| Arc::from(object.display_name()))),
            };
            if let Some(scope_value) = scope_value {
                self.release(scope_value)?;
            }
            let scope = scope_result?;

            let mut context = closure.context.clone();
            if let Some(scope) = scope {
                context.scope_class = Some(scope.clone());
                context.called_class = Some(scope.clone());
                context.declaring_class = Some(scope);
            }
            let mut rebound = php_runtime::api::ClosurePayload::new(closure.function, Vec::new());
            rebound.debug = closure.debug.clone();
            rebound.context = context;

            let mut rebound_captures = Vec::with_capacity(captures.len());
            for capture in captures.iter().copied() {
                match self.duplicate_authoritative_native_value(capture)? {
                    Some(capture) => {
                        retained.push(capture);
                        rebound_captures.push(capture);
                    }
                    None => {
                        return Err(format!("{api}() capture crossed from baseline storage"));
                    }
                }
            }
            let prepared = NativePreparedClosure::new(
                rebound,
                capture_descriptors,
                bound_this,
                rebound_captures.into_boxed_slice(),
                None,
                false,
                false,
                false,
                false,
            );
            retained.clear();
            self.publish_prepared_closure_owned(prepared)
        })();
        if result.is_err() {
            for value in retained {
                let _ = self.release(value);
            }
        }
        Some(result)
    }

    fn prepared_callable_dispatch(&self, encoded: i64) -> Option<NativePreparedCallableDispatch> {
        let index = Self::direct_value_index(encoded)?;
        let view = self.direct_prepared_callable_view(index)?;
        match view.kind {
            php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE => {
                Some(NativePreparedCallableDispatch::Closure)
            }
            php_jit::JIT_NATIVE_CALLABLE_KIND_USER_FUNCTION
            | php_jit::JIT_NATIVE_CALLABLE_KIND_INTERNAL_BUILTIN => {
                Some(NativePreparedCallableDispatch::Named(
                    self.native_callable_string(view.name_bytes, view.name_length)?,
                ))
            }
            php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD
            | php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_CLASS_METHOD => {
                let target = if view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD {
                    php_runtime::api::CallableMethodTarget::Object(
                        self.native_query_object(view.receiver)?,
                    )
                } else {
                    php_runtime::api::CallableMethodTarget::Class(
                        self.native_callable_string(view.class_bytes, view.class_length)?,
                    )
                };
                Some(NativePreparedCallableDispatch::BoundMethod {
                    target,
                    method: self.native_callable_string(view.method_bytes, view.method_length)?,
                })
            }
            php_jit::JIT_NATIVE_CALLABLE_KIND_METHOD_PLACEHOLDER
            | php_jit::JIT_NATIVE_CALLABLE_KIND_UNRESOLVED_DYNAMIC => {
                Some(NativePreparedCallableDispatch::Invalid(
                    self.native_callable_string(view.name_bytes, view.name_length)?,
                ))
            }
            _ => None,
        }
    }

    /// Move an owned result from the active external unit back to its caller.
    /// Runtime handles already belong to the request-wide arena and need no
    /// clone or replacement slot. Only unit-indexed constants and an unowned
    /// closure require translation.
    fn transfer_external_return(&mut self, encoded: i64, owner_unit: usize) -> Result<i64, String> {
        if let Some(index) = Self::direct_value_index(encoded) {
            if let Some(prepared) = self.direct_prepared_closure_mut(index)
                && prepared.closure.context.owner_unit.is_none()
            {
                prepared.closure.context.owner_unit = Some(owner_unit);
                return Ok(encoded);
            }
            // Direct arrays may still contain constants indexed by the
            // callee's IrUnit. Rewrite only those embedded constants while
            // the callee unit is active; otherwise the caller can interpret
            // the same numeric index as an unrelated value. The native
            // array slots remain authoritative and no Rust `PhpArray` is
            // reconstructed at this boundary.
            self.stabilize_direct_array_for_cross_unit(encoded)?;
            return Ok(encoded);
        }
        if php_jit::jit_decode_runtime_value(encoded).is_some() {
            return Ok(encoded);
        }
        if let Some(constant) = php_jit::jit_decode_constant(encoded)
            && !matches!(
                constant,
                u32::MAX
                    | php_jit::JIT_VALUE_UNINITIALIZED
                    | php_jit::JIT_VALUE_FALSE
                    | php_jit::JIT_VALUE_TRUE
            )
        {
            return self.stabilize_active_unit_constant(constant);
        }
        Ok(encoded)
    }

    fn retain(&mut self, encoded: i64) -> Result<(), String> {
        if let Some(index) = Self::direct_value_index(encoded) {
            let refcount = &mut self
                .direct_value_slots
                .get_mut(index)
                .ok_or_else(|| format!("direct native value {index} is missing"))?
                .refcount;
            *refcount = refcount
                .checked_add(1)
                .ok_or_else(|| format!("direct native value {index} refcount overflow"))?;
            return Ok(());
        }
        let Some(index) = php_jit::jit_decode_runtime_value(encoded) else {
            return Ok(());
        };
        Err(format!(
            "native runtime value {index} is outside the authoritative direct slot plane"
        ))
    }

    /// Classify an encoded PHP value without cloning it out of the request
    /// arena. Immediates and direct records are authoritative; cold iterator
    /// and generator records are never references.
    fn php_handle_is_reference(&self, encoded: i64) -> Option<bool> {
        if let Some(index) = Self::direct_value_index(encoded) {
            return self.direct_value_slots.get(index).and_then(|slot| {
                (slot.refcount != 0).then_some(matches!(
                    slot.kind,
                    php_jit::JIT_NATIVE_VALUE_VIEW_REFERENCE_SCALAR
                        | php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
                ))
            });
        }
        php_jit::jit_decode_runtime_value(encoded)
            .is_none()
            .then_some(false)
    }

    /// Borrows one authoritative native string without materializing or
    /// copying it. Direct string slots and immutable unit literals share this
    /// read plane; consumers that must outlive the borrow explicitly copy at
    /// their cold capability boundary.
    fn native_string_bytes(&self, encoded: i64) -> Option<&[u8]> {
        if let Some(index) = Self::direct_value_index(encoded) {
            let slot = self.direct_value_slots.get(index)?;
            if slot.refcount == 0 || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_STRING {
                return None;
            }
            let length = usize::try_from(slot.payload).ok()?;
            let base = self.direct_string_bytes.as_ptr() as usize;
            let address = usize::try_from(slot.aux).ok()?;
            let start = address.checked_sub(base)?;
            return self
                .direct_string_bytes
                .get(start..start.checked_add(length)?);
        }
        if php_jit::jit_decode_runtime_value(encoded).is_some() {
            return None;
        }
        let constant = php_jit::jit_decode_constant(encoded)?;
        match self.unit.constants.get(constant as usize)? {
            php_ir::IrConstant::String(value) => Some(value.as_bytes()),
            php_ir::IrConstant::StringBytes(value) => Some(value.as_slice()),
            _ => None,
        }
    }

    /// Copy one native string name for a cold capability lookup without
    /// materializing a PHP `Value`. Symbol tables own Rust strings, so this
    /// allocation is the exact query payload rather than a value-plane
    /// conversion.
    fn native_string_name_bytes(&self, encoded: i64) -> Option<Vec<u8>> {
        self.native_string_bytes(encoded).map(<[u8]>::to_vec)
    }

    /// Borrows the stable owner of a direct native object without demoting its
    /// authoritative property storage or constructing a Rust `Value`.
    fn native_query_object(&self, encoded: i64) -> Option<php_runtime::api::ObjectRef> {
        let encoded = self.dereference_direct_encoding(encoded);
        if let Some(index) = Self::direct_value_index(encoded) {
            return self.direct_object(index);
        }
        None
    }

    /// Returns the immutable, inheritance-complete class record already
    /// published for a native object layout.  Conditional definitions from a
    /// different unit are deliberately not searched by name: the active or
    /// deployment owner must identify the exact class allocation.
    fn prepared_native_runtime_class(&self, name: &str) -> Option<Rc<PreparedNativeRuntimeClass>> {
        let normalized = normalize_class_name(name);
        let cache = self.runtime_class_cache.borrow();
        if let Some(prepared) = cache.get(&(self.current_dynamic_unit, normalized.clone())) {
            return Some(Rc::clone(prepared));
        }
        if let Some(unit) = self.external_class_units.get(&normalized).copied()
            && let Some(prepared) = cache.get(&(Some(unit), normalized.clone()))
        {
            return Some(Rc::clone(prepared));
        }
        cache.get(&(None, normalized)).map(Rc::clone)
    }

    /// Reads one declared property cell from the authoritative native object
    /// representation without materializing the remaining object slots.
    #[allow(unsafe_code)]
    fn native_declared_property_slot(
        &mut self,
        encoded: i64,
        property: &str,
    ) -> Option<php_runtime::api::NativeDeclaredPropertySlot> {
        let location = self.native_declared_property_slot_location(encoded, property)?;
        // SAFETY: the native slot box is the authoritative immovable object
        // storage while the live direct descriptor publishes this layout.
        Some(unsafe { *location })
    }

    #[allow(unsafe_code)]
    fn native_declared_property_slot_location(
        &mut self,
        encoded: i64,
        property: &str,
    ) -> Option<*mut php_runtime::api::NativeDeclaredPropertySlot> {
        let encoded = self.dereference_direct_encoding(encoded);
        let index = Self::direct_value_index(encoded)?;
        let descriptor = *self.direct_value_slots.get(index)?;
        if descriptor.refcount == 0
            || descriptor.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
        {
            return None;
        }
        if !php_jit::jit_native_object_property_view_is_published(descriptor.flags)
            && !self.promote_direct_object_property_slots(index).ok()?
        {
            return None;
        }
        let descriptor = *self.direct_value_slots.get(index)?;
        let object = self.direct_object(index)?;
        let slot = object.declared_slot_index(property)?;
        let (base, count) = object.native_declared_slots_view(descriptor.payload)?;
        let slot = usize::try_from(slot).ok()?;
        if slot >= count {
            return None;
        }
        // SAFETY: the native slot box is the authoritative immovable object
        // storage while the live direct descriptor publishes this layout.
        Some(unsafe { base.add(slot) })
    }

    /// Reads one dynamic property from the same authoritative native value
    /// plane as declared slots. The outer option denotes a valid direct
    /// object representation; the inner option denotes property existence.
    fn native_dynamic_property_slot(
        &mut self,
        encoded: i64,
        property: &str,
    ) -> Option<Option<php_runtime::api::NativeDeclaredPropertySlot>> {
        let encoded = self.dereference_direct_encoding(encoded);
        let index = Self::direct_value_index(encoded)?;
        let descriptor = *self.direct_value_slots.get(index)?;
        if descriptor.refcount == 0
            || descriptor.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
        {
            return None;
        }
        if !php_jit::jit_native_object_property_view_is_published(descriptor.flags)
            && !self.promote_direct_object_property_slots(index).ok()?
        {
            return None;
        }
        let descriptor = *self.direct_value_slots.get(index)?;
        self.direct_object(index)?
            .native_dynamic_property_slot(descriptor.payload, property)
    }

    /// Borrows one existing object-property value from the authoritative
    /// native slot plane. Internal native classes use this for their private
    /// state instead of consulting the now-empty cold `ObjectRef` property
    /// map after promotion.
    fn native_object_property_value(&mut self, encoded: i64, property: &str) -> Option<i64> {
        if let Some(slot) = self.native_declared_property_slot(encoded, property) {
            return (slot.initialized != 0).then_some(slot.value);
        }
        self.native_dynamic_property_slot(encoded, property)?
            .and_then(|slot| (slot.initialized != 0).then_some(slot.value))
    }

    /// Moves a fresh encoded owner into one existing authoritative object
    /// property. This is the mutation counterpart of
    /// `native_object_property_value`; it neither reconstructs a Rust `Value`
    /// nor demotes sibling properties.
    #[allow(unsafe_code)]
    fn replace_native_object_property_owned(
        &mut self,
        object: i64,
        property: &str,
        value: i64,
    ) -> Result<bool, String> {
        if self.php_handle_is_reference(value) != Some(false) {
            self.release(value)?;
            return Ok(false);
        }
        if let Some(location) = self.native_declared_property_slot_location(object, property) {
            // SAFETY: `location` belongs to the request-stable authoritative
            // declared-slot box resolved above.
            let previous = unsafe { *location };
            if previous.initialized != 0
                && self.php_handle_is_reference(previous.value) != Some(false)
            {
                self.release(value)?;
                return Ok(false);
            }
            // SAFETY: the fresh owner moves into the stable property cell.
            unsafe {
                *location = php_runtime::api::NativeDeclaredPropertySlot {
                    initialized: 1,
                    reserved: 0,
                    value,
                };
            }
            self.mark_roots_dirty(RootMutationReason::RootedContainer);
            if previous.initialized != 0 {
                self.release(previous.value)?;
            }
            return Ok(true);
        }

        let object = self.dereference_direct_encoding(object);
        let Some(index) = Self::direct_value_index(object) else {
            self.release(value)?;
            return Ok(false);
        };
        let Some(descriptor) = self.direct_value_slots.get(index).copied() else {
            self.release(value)?;
            return Ok(false);
        };
        if descriptor.refcount == 0
            || descriptor.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
            || (!php_jit::jit_native_object_property_view_is_published(descriptor.flags)
                && !self.promote_direct_object_property_slots(index)?)
        {
            self.release(value)?;
            return Ok(false);
        }
        let descriptor = self.direct_value_slots[index];
        let Some(owner) = self.direct_object(index) else {
            self.release(value)?;
            return Ok(false);
        };
        let Some(Some(previous)) = owner.native_dynamic_property_slot(descriptor.payload, property)
        else {
            self.release(value)?;
            return Ok(false);
        };
        if self.php_handle_is_reference(previous.value) != Some(false) {
            self.release(value)?;
            return Ok(false);
        }
        let replacement = php_runtime::api::NativeDeclaredPropertySlot {
            initialized: 1,
            reserved: 0,
            value,
        };
        let previous = match owner.set_native_dynamic_property(
            descriptor.payload,
            property.to_owned(),
            replacement,
        ) {
            Ok(Some(previous)) => previous,
            Ok(None) => {
                return Err(format!(
                    "native internal property {property} disappeared during replacement"
                ));
            }
            Err(replacement) => {
                self.release(replacement.value)?;
                return Ok(false);
            }
        };
        self.mark_roots_dirty(RootMutationReason::RootedContainer);
        self.release(previous.value)?;
        Ok(true)
    }

    /// Replaces one ordinary declared-property owner without materializing
    /// either the object or the assigned value into the cold Rust plane.
    ///
    /// The property and assignment expression each need an independent
    /// owner unless executable ownership moves the input owner into the
    /// expression result. Reference-backed cells deliberately remain a cold
    /// semantic shape until their write-through path is native as well.
    #[allow(unsafe_code)]
    fn assign_plain_native_declared_property(
        &mut self,
        object: i64,
        value: i64,
        property: &str,
        move_result: bool,
    ) -> Result<Option<i64>, String> {
        let Some(location) = self.native_declared_property_slot_location(object, property) else {
            return Ok(None);
        };
        // SAFETY: `location` belongs to the request-stable authoritative
        // declared-slot box resolved above.
        let previous = unsafe { *location };
        if previous.initialized != 0 && self.php_handle_is_reference(previous.value) != Some(false)
        {
            return Ok(None);
        }
        if self.php_handle_is_reference(value) != Some(false) {
            return Ok(None);
        }
        let Some(property_owner) = self.duplicate_authoritative_native_value(value)? else {
            return Ok(None);
        };
        let result = if move_result {
            value
        } else {
            let Some(result) = self.duplicate_authoritative_native_value(value)? else {
                self.release(property_owner)?;
                return Ok(None);
            };
            result
        };
        // SAFETY: the old owner remains live until the replacement record has
        // been installed. The new record consumes `property_owner`.
        unsafe {
            *location = php_runtime::api::NativeDeclaredPropertySlot {
                initialized: 1,
                reserved: 0,
                value: property_owner,
            };
        }
        self.mark_roots_dirty(RootMutationReason::RootedContainer);
        if previous.initialized != 0
            && let Err(error) = self.release(previous.value)
        {
            if !move_result {
                let _ = self.release(result);
            }
            return Err(error);
        }
        Ok(Some(result))
    }

    /// Replaces one ordinary dynamic-property owner without decoding the
    /// receiver or assigned value. Magic access, declared-name visibility,
    /// references, and creation diagnostics are admitted by the caller.
    fn assign_plain_native_dynamic_property(
        &mut self,
        object: i64,
        value: i64,
        property: &str,
        move_result: bool,
    ) -> Result<Option<i64>, String> {
        let object = self.dereference_direct_encoding(object);
        let Some(index) = Self::direct_value_index(object) else {
            return Ok(None);
        };
        let Some(descriptor) = self.direct_value_slots.get(index).copied() else {
            return Ok(None);
        };
        if descriptor.refcount == 0
            || descriptor.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
            || (!php_jit::jit_native_object_property_view_is_published(descriptor.flags)
                && !self.promote_direct_object_property_slots(index)?)
        {
            return Ok(None);
        }
        let descriptor = self.direct_value_slots[index];
        let Some(owner) = self.direct_object(index) else {
            return Ok(None);
        };
        let Some(previous) = owner.native_dynamic_property_slot(descriptor.payload, property)
        else {
            return Ok(None);
        };
        if previous.is_some_and(|slot| self.php_handle_is_reference(slot.value) != Some(false))
            || self.php_handle_is_reference(value) != Some(false)
        {
            return Ok(None);
        }
        let Some(property_owner) = self.duplicate_authoritative_native_value(value)? else {
            return Ok(None);
        };
        let result = if move_result {
            value
        } else {
            let Some(result) = self.duplicate_authoritative_native_value(value)? else {
                self.release(property_owner)?;
                return Ok(None);
            };
            result
        };
        let replacement = php_runtime::api::NativeDeclaredPropertySlot {
            initialized: 1,
            reserved: 0,
            value: property_owner,
        };
        let replaced =
            owner.set_native_dynamic_property(descriptor.payload, property.to_owned(), replacement);
        let previous = match replaced {
            Ok(previous) => previous,
            Err(replacement) => {
                self.release(replacement.value)?;
                if !move_result {
                    self.release(result)?;
                }
                return Ok(None);
            }
        };
        self.mark_roots_dirty(RootMutationReason::RootedContainer);
        if let Some(previous) = previous
            && let Err(error) = self.release(previous.value)
        {
            if !move_result {
                let _ = self.release(result);
            }
            return Err(error);
        }
        Ok(Some(result))
    }

    /// Removes one existing dynamic-property owner directly. Missing
    /// properties are a successful no-op; an outer `None` requests baseline.
    fn unset_plain_native_dynamic_property(
        &mut self,
        object: i64,
        property: &str,
    ) -> Result<Option<()>, String> {
        let object = self.dereference_direct_encoding(object);
        let Some(index) = Self::direct_value_index(object) else {
            return Ok(None);
        };
        let Some(descriptor) = self.direct_value_slots.get(index).copied() else {
            return Ok(None);
        };
        if descriptor.refcount == 0
            || descriptor.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
            || (!php_jit::jit_native_object_property_view_is_published(descriptor.flags)
                && !self.promote_direct_object_property_slots(index)?)
        {
            return Ok(None);
        }
        let descriptor = self.direct_value_slots[index];
        let Some(owner) = self.direct_object(index) else {
            return Ok(None);
        };
        let Some(removed) = owner.unset_native_dynamic_property(descriptor.payload, property)
        else {
            return Ok(None);
        };
        if let Some(removed) = removed {
            self.mark_roots_dirty(RootMutationReason::RootedContainer);
            self.release(removed.value)?;
        }
        Ok(Some(()))
    }

    /// Creates a direct reference whose payload ownership is supplied by the
    /// caller. No `ReferenceCell` exists on this path; an explicit cold decode
    /// creates and publishes that compatibility identity only when required.
    fn encode_direct_reference_payload_owned(&mut self, payload: i64) -> Result<i64, String> {
        let index = self.reserve_direct_value_slot()?;
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR,
            flags: php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION,
            reserved: php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_PUBLISHED,
            payload: payload as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
        let Some(runtime_index) = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
        else {
            self.direct_value_slots[index] = php_jit::JitNativeValueSlot::default();
            return Err("direct native reference handle overflow".to_owned());
        };
        Ok((php_jit::JIT_VALUE_RUNTIME_REFERENCE_TAG | u64::from(runtime_index)) as i64)
    }

    /// Turns one authoritative declared-property cell into a direct reference
    /// without materializing the object or any sibling property. The property
    /// owns one reference handle and the returned handle is an independent
    /// owner for the callee frame.
    #[allow(unsafe_code)]
    fn bind_native_declared_property_reference(
        &mut self,
        object: i64,
        property: &str,
    ) -> Result<Option<i64>, String> {
        let Some(location) = self.native_declared_property_slot_location(object, property) else {
            return Ok(None);
        };
        // SAFETY: location belongs to the request-stable native declared slot
        // vector resolved above and remains live for this synchronous bind.
        let previous = unsafe { *location };
        if previous.initialized != 0 && self.php_handle_is_reference(previous.value) == Some(true) {
            self.retain(previous.value)?;
            return Ok(Some(previous.value));
        }
        let payload = if previous.initialized == 0 {
            php_jit::jit_encode_constant(u32::MAX)
        } else {
            previous.value
        };
        // Keep the existing property owner intact until both reference owners
        // have been established. This makes every error path recover without
        // reviving a released payload.
        self.retain(payload)?;
        let reference = match self.encode_direct_reference_payload_owned(payload) {
            Ok(reference) => reference,
            Err(error) => {
                self.release(payload)?;
                return Err(error);
            }
        };
        if let Err(error) = self.retain(reference) {
            self.release(reference)?;
            return Err(error);
        }
        let callee_owner = reference;
        // SAFETY: same stable slot location as above. Ownership of one
        // reference handle moves into the property cell.
        unsafe {
            *location = php_runtime::api::NativeDeclaredPropertySlot {
                initialized: 1,
                reserved: 0,
                value: reference,
            };
        }
        if previous.initialized != 0 {
            self.release(previous.value)?;
        }
        Ok(Some(callee_owner))
    }

    /// Gives an exact native call an independently owned dereferenced value
    /// without entering `ReferenceCell` or the Rust `Value` plane. `None`
    /// means the caller must take its one baseline continuation before any
    /// PHP-visible call binding effect.
    fn duplicate_authoritative_dereferenced_native_value(
        &mut self,
        mut encoded: i64,
    ) -> Result<Option<i64>, String> {
        for _ in 0..16 {
            let Some(index) = Self::direct_value_index(encoded) else {
                break;
            };
            let Some(slot) = self
                .direct_value_slots
                .get(index)
                .copied()
                .filter(|slot| slot.refcount != 0)
            else {
                return Ok(None);
            };
            match slot.kind {
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
                    if slot.flags == php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION
                        && native_reference_state(slot.reserved)
                            != php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_EMPTY =>
                {
                    encoded = slot.payload as i64;
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_REFERENCE_SCALAR => return Ok(None),
                _ => return self.duplicate_authoritative_native_value(encoded),
            }
        }
        if self.php_handle_is_reference(encoded) == Some(true) {
            return Ok(None);
        }
        if php_jit::jit_decode_runtime_value(encoded).is_some() {
            return Ok(None);
        }
        self.duplicate_authoritative_native_value(encoded)
    }

    fn direct_reference_payload(&self, encoded: i64) -> Option<i64> {
        let index = Self::direct_value_index(encoded)?;
        let slot = *self.direct_value_slots.get(index)?;
        (slot.refcount != 0
            && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
            && slot.flags == php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION
            && native_reference_state(slot.reserved)
                != php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_EMPTY)
            .then_some(slot.payload as i64)
    }

    fn dereference_direct_encoding(&self, mut encoded: i64) -> i64 {
        for _ in 0..16 {
            let Some(payload) = self.direct_reference_payload(encoded) else {
                break;
            };
            encoded = payload;
        }
        encoded
    }

    fn native_encoded_value_kind(&self, encoded: i64) -> Option<NativeEncodedValueKind> {
        let encoded = self.dereference_direct_encoding(encoded);
        if let Some(constant) = php_jit::jit_decode_constant(encoded) {
            return match constant {
                u32::MAX => Some(NativeEncodedValueKind::Null),
                php_jit::JIT_VALUE_UNINITIALIZED => Some(NativeEncodedValueKind::Uninitialized),
                php_jit::JIT_VALUE_FALSE => Some(NativeEncodedValueKind::Bool(false)),
                php_jit::JIT_VALUE_TRUE => Some(NativeEncodedValueKind::Bool(true)),
                constant => match self.unit.constants.get(constant as usize)? {
                    php_ir::IrConstant::Null => Some(NativeEncodedValueKind::Null),
                    php_ir::IrConstant::Bool(value) => Some(NativeEncodedValueKind::Bool(*value)),
                    php_ir::IrConstant::Int(_) => Some(NativeEncodedValueKind::Int),
                    php_ir::IrConstant::Float(_) => Some(NativeEncodedValueKind::Float),
                    php_ir::IrConstant::String(_) | php_ir::IrConstant::StringBytes(_) => {
                        Some(NativeEncodedValueKind::String)
                    }
                    php_ir::IrConstant::Array(_) => Some(NativeEncodedValueKind::Array),
                    php_ir::IrConstant::NamedConstant(_)
                    | php_ir::IrConstant::ClassConstant { .. } => None,
                },
            };
        }
        if php_jit::jit_decode_runtime_value(encoded).is_none() {
            return Some(NativeEncodedValueKind::Int);
        }
        if let Some(index) = Self::direct_value_index(encoded) {
            let slot = self.direct_value_slots.get(index)?;
            if slot.refcount == 0 {
                return None;
            }
            return match slot.kind {
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_INT
                    if slot.flags == php_jit::JIT_NATIVE_DIRECT_INT_ABI_VERSION =>
                {
                    Some(NativeEncodedValueKind::Int)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_STRING => Some(NativeEncodedValueKind::String),
                php_jit::JIT_NATIVE_VALUE_VIEW_ARRAY
                | php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY
                | php_jit::JIT_NATIVE_VALUE_VIEW_SHARED_ARRAY
                | php_jit::JIT_NATIVE_VALUE_VIEW_BORROWED_REFERENCE_ARRAY
                | php_jit::JIT_NATIVE_VALUE_VIEW_GLOBALS_PROXY => {
                    Some(NativeEncodedValueKind::Array)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_FLOAT => Some(NativeEncodedValueKind::Float),
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT => {
                    Some(NativeEncodedValueKind::Object)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_RESOURCE => {
                    Some(NativeEncodedValueKind::Resource)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE => {
                    Some(NativeEncodedValueKind::Callable)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FIBER
                | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_FIBER => {
                    Some(NativeEncodedValueKind::Fiber)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_GENERATOR
                | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_GENERATOR
                | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_GENERATOR => {
                    Some(NativeEncodedValueKind::Generator)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_REFERENCE_SCALAR
                | php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR => {
                    Some(NativeEncodedValueKind::Reference)
                }
                _ => None,
            };
        }
        let _ = php_jit::jit_decode_runtime_value(encoded)?;
        None
    }

    fn native_encoded_int(&self, encoded: i64) -> Option<i64> {
        let encoded = self.dereference_direct_encoding(encoded);
        if let Some(index) = Self::direct_value_index(encoded) {
            let slot = self.direct_value_slots.get(index)?;
            return (slot.refcount != 0
                && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_INT
                && slot.flags == php_jit::JIT_NATIVE_DIRECT_INT_ABI_VERSION)
                .then_some(slot.payload as i64);
        }
        if php_jit::jit_decode_runtime_value(encoded).is_none()
            && php_jit::jit_decode_constant(encoded).is_none()
        {
            return Some(encoded);
        }
        if let Some(constant) = php_jit::jit_decode_constant(encoded) {
            return match self.unit.constants.get(constant as usize)? {
                php_ir::IrConstant::Int(value) => Some(*value),
                _ => None,
            };
        }
        None
    }

    fn native_encoded_float(&self, encoded: i64) -> Option<f64> {
        let encoded = self.dereference_direct_encoding(encoded);
        if let Some(index) = Self::direct_value_index(encoded) {
            let slot = self.direct_value_slots.get(index)?;
            return (slot.refcount != 0 && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_FLOAT)
                .then(|| f64::from_bits(slot.payload));
        }
        if let Some(constant) = php_jit::jit_decode_constant(encoded) {
            return match self.unit.constants.get(constant as usize)? {
                php_ir::IrConstant::Float(value) => Some(*value),
                _ => None,
            };
        }
        None
    }

    fn native_encoded_bool(&self, encoded: i64) -> Option<bool> {
        match self.native_encoded_value_kind(encoded)? {
            NativeEncodedValueKind::Bool(value) => Some(value),
            _ => None,
        }
    }

    fn native_encoded_resource_id(&self, encoded: i64) -> Option<u64> {
        let encoded = self.dereference_direct_encoding(encoded);
        let index = Self::direct_value_index(encoded)?;
        let slot = self.direct_value_slots.get(index)?;
        (slot.refcount != 0
            && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_RESOURCE
            && slot.flags == php_jit::JIT_NATIVE_DIRECT_RESOURCE_ABI_VERSION)
            .then_some(slot.payload)
    }

    fn native_encoded_type_name(&self, encoded: i64) -> &'static str {
        match self.native_encoded_value_kind(encoded) {
            Some(NativeEncodedValueKind::Null) => "null",
            Some(NativeEncodedValueKind::Uninitialized) => "uninitialized",
            Some(NativeEncodedValueKind::Bool(_)) => "bool",
            Some(NativeEncodedValueKind::Int) => "int",
            Some(NativeEncodedValueKind::Float) => "float",
            Some(NativeEncodedValueKind::String) => "string",
            Some(NativeEncodedValueKind::Array) => "array",
            Some(NativeEncodedValueKind::Object | NativeEncodedValueKind::Callable) => "object",
            Some(NativeEncodedValueKind::Resource) => "resource",
            Some(NativeEncodedValueKind::Generator) => "Generator",
            Some(NativeEncodedValueKind::Fiber) => "Fiber",
            Some(NativeEncodedValueKind::Reference) => "reference",
            None => "unknown",
        }
    }

    fn native_encoded_is_callable(&self, encoded: i64) -> Option<bool> {
        let encoded = self.dereference_direct_encoding(encoded);
        match self.native_encoded_value_kind(encoded)? {
            NativeEncodedValueKind::Callable => Some(true),
            NativeEncodedValueKind::Object => {
                let object = self.native_query_object(encoded)?;
                let class = object.class_name();
                Some(
                    native_method_in_hierarchy(self, &class, "__invoke").is_some()
                        || native_external_method(self, &class, "__invoke").is_some(),
                )
            }
            NativeEncodedValueKind::String => {
                let name = String::from_utf8_lossy(self.native_string_bytes(encoded)?);
                Some(if let Some((class, method)) = name.split_once("::") {
                    native_method_in_hierarchy(self, class, method).is_some()
                        || native_external_method(self, class, method).is_some()
                } else {
                    self.function_id(&name).is_some()
                        || self.external_function(&name).is_some()
                        || php_extensions::BuiltinRegistry::new()
                            .contains(&name.to_ascii_lowercase())
                })
            }
            NativeEncodedValueKind::Array => {
                let entries = self.direct_array_entries_for(encoded)?;
                if entries.len() != 2 {
                    return Some(false);
                }
                let mut target = None;
                let mut method = None;
                for entry in entries {
                    match self.native_encoded_int(entry.key) {
                        Some(0) => target = Some(entry.value),
                        Some(1) => method = Some(entry.value),
                        _ => {}
                    }
                }
                let target = self.dereference_direct_encoding(target?);
                let method = self.dereference_direct_encoding(method?);
                let method = String::from_utf8_lossy(self.native_string_bytes(method)?);
                if let Some(object) = self.native_query_object(target) {
                    let class = object.class_name();
                    Some(
                        native_method_in_hierarchy(self, &class, &method).is_some()
                            || native_external_method(self, &class, &method).is_some(),
                    )
                } else {
                    let class = String::from_utf8_lossy(self.native_string_bytes(target)?);
                    Some(
                        native_method_in_hierarchy(self, &class, &method).is_some()
                            || native_external_method(self, &class, &method).is_some(),
                    )
                }
            }
            _ => Some(false),
        }
    }

    fn native_encoded_matches_ir_type(
        &self,
        encoded: i64,
        type_: &php_ir::IrReturnType,
    ) -> Option<bool> {
        use php_ir::IrReturnType as Ir;
        let encoded = self.dereference_direct_encoding(encoded);
        let kind = self.native_encoded_value_kind(encoded)?;
        match type_ {
            Ir::Int => Some(kind == NativeEncodedValueKind::Int),
            Ir::Float => Some(matches!(
                kind,
                NativeEncodedValueKind::Float | NativeEncodedValueKind::Int
            )),
            Ir::String => Some(kind == NativeEncodedValueKind::String),
            Ir::Array => Some(kind == NativeEncodedValueKind::Array),
            Ir::Callable => self.native_encoded_is_callable(encoded),
            Ir::Iterable => Some(match kind {
                NativeEncodedValueKind::Array | NativeEncodedValueKind::Generator => true,
                NativeEncodedValueKind::Object => {
                    self.native_query_object(encoded).is_some_and(|object| {
                        native_class_is_a(self, &object.class_name(), "traversable")
                    })
                }
                _ => false,
            }),
            Ir::Object => Some(matches!(
                kind,
                NativeEncodedValueKind::Object
                    | NativeEncodedValueKind::Callable
                    | NativeEncodedValueKind::Generator
                    | NativeEncodedValueKind::Fiber
            )),
            Ir::Bool => Some(matches!(kind, NativeEncodedValueKind::Bool(_))),
            Ir::Null | Ir::Void => Some(kind == NativeEncodedValueKind::Null),
            Ir::Mixed => Some(true),
            Ir::Never => Some(false),
            Ir::False => Some(kind == NativeEncodedValueKind::Bool(false)),
            Ir::True => Some(kind == NativeEncodedValueKind::Bool(true)),
            Ir::Class { name, .. } => Some(
                native_special_value_class_is_a(kind, name).unwrap_or_else(|| {
                    self.native_query_object(encoded)
                        .is_some_and(|object| native_class_is_a(self, &object.class_name(), name))
                }),
            ),
            Ir::Nullable { inner } => {
                if kind == NativeEncodedValueKind::Null {
                    Some(true)
                } else {
                    self.native_encoded_matches_ir_type(encoded, inner)
                }
            }
            Ir::Union { members } | Ir::Dnf { members } => {
                let mut unknown = false;
                for member in members {
                    match self.native_encoded_matches_ir_type(encoded, member) {
                        Some(true) => return Some(true),
                        Some(false) => {}
                        None => unknown = true,
                    }
                }
                (!unknown).then_some(false)
            }
            Ir::Intersection { members } => {
                let mut unknown = false;
                for member in members {
                    match self.native_encoded_matches_ir_type(encoded, member) {
                        Some(true) => {}
                        Some(false) => return Some(false),
                        None => unknown = true,
                    }
                }
                (!unknown).then_some(true)
            }
        }
    }

    /// Checks whether a native value already has a representation accepted by
    /// typed storage. Unlike call-argument admission, this must not treat an
    /// integer as an already-coerced float.
    fn native_encoded_exactly_matches_ir_type(
        &self,
        encoded: i64,
        type_: &php_ir::IrReturnType,
    ) -> Option<bool> {
        use php_ir::IrReturnType as Ir;
        let encoded = self.dereference_direct_encoding(encoded);
        let kind = self.native_encoded_value_kind(encoded)?;
        match type_ {
            Ir::Float => Some(kind == NativeEncodedValueKind::Float),
            Ir::Nullable { inner } => {
                if kind == NativeEncodedValueKind::Null {
                    Some(true)
                } else {
                    self.native_encoded_exactly_matches_ir_type(encoded, inner)
                }
            }
            Ir::Union { members } | Ir::Dnf { members } => {
                let mut unknown = false;
                for member in members {
                    match self.native_encoded_exactly_matches_ir_type(encoded, member) {
                        Some(true) => return Some(true),
                        Some(false) => {}
                        None => unknown = true,
                    }
                }
                (!unknown).then_some(false)
            }
            Ir::Intersection { members } => {
                let mut unknown = false;
                for member in members {
                    match self.native_encoded_exactly_matches_ir_type(encoded, member) {
                        Some(true) => {}
                        Some(false) => return Some(false),
                        None => unknown = true,
                    }
                }
                (!unknown).then_some(true)
            }
            _ => self.native_encoded_matches_ir_type(encoded, type_),
        }
    }

    /// Produces one owned native value for a typed by-value call parameter.
    /// `None` denotes a compatibility-only shape which has already crossed a
    /// cold call boundary and still requires the baseline `Value` coercer.
    fn coerce_native_call_argument_encoded(
        &mut self,
        encoded: i64,
        type_: &php_ir::IrReturnType,
        strict: bool,
    ) -> Result<Option<i64>, String> {
        use php_ir::IrReturnType as Type;
        let encoded = self.dereference_direct_encoding(encoded);
        let Some(kind) = self.native_encoded_value_kind(encoded) else {
            return Ok(None);
        };

        if let Type::Nullable { inner } = type_ {
            if kind == NativeEncodedValueKind::Null {
                return self.duplicate_authoritative_native_value(encoded);
            }
            return self.coerce_native_call_argument_encoded(encoded, inner, strict);
        }

        // PHP admits int for a float declaration even under strict_types and
        // the callee observes a float value.
        if matches!(type_, Type::Float) && kind == NativeEncodedValueKind::Int {
            let value = self
                .native_encoded_int(encoded)
                .expect("classified native int has an integer payload");
            return self
                .encode_native_float_owner(php_runtime::api::FloatValue::from_f64(value as f64))
                .map(Some);
        }
        if self.native_encoded_matches_ir_type(encoded, type_) == Some(true) || strict {
            return self.duplicate_authoritative_native_value(encoded);
        }

        let converted = match (type_, kind) {
            (Type::Int, NativeEncodedValueKind::String) => {
                let bytes = self
                    .native_string_bytes(encoded)
                    .expect("classified native string has bytes");
                String::from_utf8_lossy(bytes).trim().parse::<i64>().ok()
            }
            (Type::Int, NativeEncodedValueKind::Float) => {
                self.native_encoded_float(encoded).map(|value| value as i64)
            }
            (Type::Int, NativeEncodedValueKind::Bool(_)) => {
                self.native_encoded_bool(encoded).map(i64::from)
            }
            _ => None,
        };
        if let Some(value) = converted {
            return Ok(Some(value));
        }

        match (type_, kind) {
            (Type::Float, NativeEncodedValueKind::String) => {
                let bytes = self
                    .native_string_bytes(encoded)
                    .expect("classified native string has bytes");
                if let Ok(value) = String::from_utf8_lossy(bytes).trim().parse::<f64>() {
                    return self
                        .encode_native_float_owner(php_runtime::api::FloatValue::from_f64(value))
                        .map(Some);
                }
            }
            (Type::Float, NativeEncodedValueKind::Bool(_)) => {
                let value = if self.native_encoded_bool(encoded).unwrap_or(false) {
                    1.0
                } else {
                    0.0
                };
                return self
                    .encode_native_float_owner(php_runtime::api::FloatValue::from_f64(value))
                    .map(Some);
            }
            (Type::String, NativeEncodedValueKind::Int) => {
                let value = self
                    .native_encoded_int(encoded)
                    .expect("classified native int has an integer payload");
                return self
                    .encode_direct_string_bytes(value.to_string().as_bytes())
                    .map(Some);
            }
            (Type::String, NativeEncodedValueKind::Float) => {
                let value = self
                    .native_encoded_float(encoded)
                    .expect("classified native float has a float payload");
                return self
                    .encode_direct_string_bytes(value.to_string().as_bytes())
                    .map(Some);
            }
            (Type::String, NativeEncodedValueKind::Bool(value)) => {
                return self
                    .encode_direct_string_bytes(if value { b"1" } else { b"" })
                    .map(Some);
            }
            (
                Type::Bool,
                NativeEncodedValueKind::Int
                | NativeEncodedValueKind::Float
                | NativeEncodedValueKind::String,
            ) => {
                if let Some(value) = self.native_encoded_truthy(encoded) {
                    return Ok(Some(php_jit::jit_encode_constant(if value {
                        php_jit::JIT_VALUE_TRUE
                    } else {
                        php_jit::JIT_VALUE_FALSE
                    })));
                }
            }
            (Type::Nullable { inner }, _) => {
                return self.coerce_native_call_argument_encoded(encoded, inner, strict);
            }
            (Type::Union { members } | Type::Dnf { members }, _) => {
                for member in members {
                    let Some(candidate) =
                        self.coerce_native_call_argument_encoded(encoded, member, strict)?
                    else {
                        continue;
                    };
                    if self.native_encoded_matches_ir_type(candidate, type_) == Some(true) {
                        return Ok(Some(candidate));
                    }
                    self.release(candidate)?;
                }
            }
            _ => {}
        }
        self.duplicate_authoritative_native_value(encoded)
    }

    /// Returns `None` for a shape that needs baseline semantics, otherwise an
    /// exact PHP isset classification without constructing a Rust `Value`.
    fn native_encoded_is_set(&self, encoded: i64) -> Option<bool> {
        let encoded = self.dereference_direct_encoding(encoded);
        if php_jit::jit_decode_runtime_value(encoded).is_none()
            && php_jit::jit_decode_constant(encoded).is_none()
        {
            return Some(true);
        }
        if let Some(constant) = php_jit::jit_decode_constant(encoded) {
            return Some(!matches!(
                constant,
                u32::MAX | php_jit::JIT_VALUE_UNINITIALIZED
            ));
        }
        if let Some(index) = Self::direct_value_index(encoded) {
            let slot = self.direct_value_slots.get(index)?;
            if matches!(
                slot.kind,
                php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
                    | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR
            ) {
                return None;
            }
            return (slot.refcount != 0
                && !matches!(
                    slot.kind,
                    php_jit::JIT_NATIVE_VALUE_VIEW_REFERENCE_SCALAR
                        | php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
                ))
            .then_some(true);
        }
        let _ = php_jit::jit_decode_runtime_value(encoded)?;
        None
    }

    /// Exact native truthiness for scalar/string/array common shapes. Objects
    /// and materialized compatibility references remain baseline because
    /// SimpleXML and user-visible reference state require cold semantics.
    fn native_encoded_truthy(&self, encoded: i64) -> Option<bool> {
        let encoded = self.dereference_direct_encoding(encoded);
        if php_jit::jit_decode_runtime_value(encoded).is_none()
            && php_jit::jit_decode_constant(encoded).is_none()
        {
            return Some(encoded != 0);
        }
        if let Some(constant) = php_jit::jit_decode_constant(encoded) {
            return match constant {
                u32::MAX | php_jit::JIT_VALUE_UNINITIALIZED | php_jit::JIT_VALUE_FALSE => {
                    Some(false)
                }
                php_jit::JIT_VALUE_TRUE => Some(true),
                _ => None,
            };
        }
        if let Some(index) = Self::direct_value_index(encoded) {
            let slot = *self.direct_value_slots.get(index)?;
            if slot.refcount == 0 {
                return None;
            }
            return match slot.kind {
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_INT
                    if slot.flags == php_jit::JIT_NATIVE_DIRECT_INT_ABI_VERSION =>
                {
                    Some(slot.payload != 0)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_FLOAT => Some(f64::from_bits(slot.payload) != 0.0),
                php_jit::JIT_NATIVE_VALUE_VIEW_STRING => Some(
                    slot.payload != 0 && slot.reserved & php_jit::JIT_NATIVE_STRING_VALUE_ZERO == 0,
                ),
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY => Some(slot.payload != 0),
                php_jit::JIT_NATIVE_VALUE_VIEW_SHARED_ARRAY
                | php_jit::JIT_NATIVE_VALUE_VIEW_BORROWED_REFERENCE_ARRAY => {
                    baseline_shared_array_storage_is_empty(slot.payload as usize)
                        .map(|is_empty| !is_empty)
                }
                php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
                | php_jit::JIT_NATIVE_VALUE_VIEW_REFERENCE_SCALAR
                | php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR => None,
                php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
                | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR => None,
                _ => Some(true),
            };
        }
        let _ = php_jit::jit_decode_runtime_value(encoded)?;
        None
    }

    /// Outer `None` means a non-direct shape; inner `None` means a valid
    /// direct traversal whose key is absent.
    fn direct_dimension_path_encoded(
        &mut self,
        mut encoded: i64,
        keys: &[i64],
    ) -> Result<Option<Option<i64>>, String> {
        for key in keys {
            encoded = self.dereference_direct_encoding(encoded);
            if self.direct_array_slot(encoded).is_none() {
                return Ok(None);
            }
            let Some(key) = self.native_encoded_plain_array_key(*key) else {
                return Ok(None);
            };
            let Some(value) = self.direct_array_find_encoded(encoded, &key)? else {
                return Ok(Some(None));
            };
            encoded = value;
        }
        Ok(Some(Some(encoded)))
    }

    fn php_handle_is_uninitialized(&self, encoded: i64) -> bool {
        if php_jit::jit_decode_constant(encoded) == Some(php_jit::JIT_VALUE_UNINITIALIZED) {
            return true;
        }
        false
    }

    fn release(&mut self, encoded: i64) -> Result<(), String> {
        if let Some(index) = Self::direct_value_index(encoded) {
            return self.release_direct_value_index(index);
        }
        let Some(index) = php_jit::jit_decode_runtime_value(encoded) else {
            return Ok(());
        };
        Err(format!(
            "native runtime value {index} is outside the authoritative direct slot plane"
        ))
    }

    fn release_direct_value_index(&mut self, index: usize) -> Result<(), String> {
        let reached_zero = {
            let slot = self
                .direct_value_slots
                .get_mut(index)
                .ok_or_else(|| format!("direct native value {index} is missing"))?;
            if slot.refcount == 0 {
                return Err(format!(
                    "direct native value {index} was already released (retired kind {})",
                    slot.flags
                ));
            }
            slot.refcount -= 1;
            slot.refcount == 0
        };
        if !reached_zero {
            return Ok(());
        }
        self.cross_unit_stable_values.remove(&index);
        let mut direct_object_children = Vec::new();
        if self.direct_value_slots[index].kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT {
            let object = self
                .direct_object_owner(index)
                .ok_or_else(|| format!("direct native object {index} has no stable owner"))?;
            let has_cold_alias = object.gc_refcount_estimate() > 2;
            if self.object_has_native_destructor(&object.class_name()) || has_cold_alias {
                // The direct descriptor is losing its final encoded owner, but
                // an ObjectRef may still live in a PHP reference/root. Restore
                // Rust slots before dropping the native owner so that alias
                // remains a complete object rather than an empty shell.
                self.demote_direct_object_property_slots(index)?;
            } else {
                direct_object_children = self.take_direct_object_children(index)?;
            }
        }
        let slot = self.direct_value_slots[index];
        let released_object = if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT {
            let owner = std::mem::replace(&mut self.direct_object_owners[index], 0);
            if owner == 0 {
                return Err(format!(
                    "direct native object {index} lost its stable owner"
                ));
            }
            // SAFETY: object publication created exactly one Box<ObjectRef>
            // for this slot and release clears/reclaims it exactly once when
            // the authoritative direct refcount reaches zero.
            #[allow(unsafe_code)]
            let object =
                unsafe { *Box::from_raw(owner as usize as *mut php_runtime::api::ObjectRef) };
            if self.baseline_values.direct_object_handles.get(&object.id()) == Some(&(index as u32))
            {
                self.baseline_values
                    .direct_object_handles
                    .remove(&object.id());
            }
            Some(object)
        } else {
            None
        };
        let released_resource = if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_RESOURCE {
            if slot.aux == 0 {
                return Err(format!(
                    "direct native resource {index} lost its stable owner"
                ));
            }
            // SAFETY: resource publication created exactly one boxed
            // ResourceRef and final direct-slot release reclaims it once.
            #[allow(unsafe_code)]
            let resource =
                unsafe { Box::from_raw(slot.aux as usize as *mut php_runtime::api::ResourceRef) };
            if self.direct_resource_handles.get(&resource.id().get()) == Some(&(index as u32)) {
                self.direct_resource_handles.remove(&resource.id().get());
            }
            Some(resource)
        } else {
            None
        };
        let released_callable = if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE {
            if slot.aux == 0 {
                return Err(format!(
                    "direct native callable {index} lost its stable record"
                ));
            }
            // SAFETY: callable publication created exactly one boxed record
            // for this slot and final release reclaims it exactly once.
            #[allow(unsafe_code)]
            let callable =
                unsafe { Box::from_raw(slot.aux as usize as *mut NativePreparedCallableOwner) };
            if callable.native_view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE
                && self.direct_closure_handles.get(&slot.payload) == Some(&(index as u32))
            {
                self.direct_closure_handles.remove(&slot.payload);
            }
            Some(callable)
        } else {
            None
        };
        let released_fiber = if matches!(
            slot.kind,
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FIBER
                | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_FIBER
        ) {
            if slot.aux == 0 {
                return Err(format!(
                    "direct native Fiber {index} lost its stable record"
                ));
            }
            // SAFETY: Fiber publication created exactly one boxed record and
            // final direct-slot release reclaims it exactly once.
            #[allow(unsafe_code)]
            let fiber = unsafe { Box::from_raw(slot.aux as usize as *mut NativeDirectFiber) };
            self.baseline_values
                .direct_fiber_handles
                .retain(|_, mapped| *mapped as usize != index);
            self.baseline_values.direct_fiber_cells.remove(&index);
            Some(fiber)
        } else {
            None
        };
        let released_fiber_execution = released_fiber
            .as_ref()
            .and_then(|_| self.fiber_executions.remove(&(index as u64)));
        let released_generator = if matches!(
            slot.kind,
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_GENERATOR
                | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_GENERATOR
        ) {
            if slot.aux == 0 {
                return Err(format!(
                    "direct native Generator {index} lost its stable activation"
                ));
            }
            // SAFETY: Generator publication created exactly one boxed
            // activation and final direct-slot release reclaims it once.
            #[allow(unsafe_code)]
            let generator =
                unsafe { Box::from_raw(slot.aux as usize as *mut NativeDirectGenerator) };
            self.baseline_values
                .direct_generator_handles
                .retain(|_, mapped| *mapped as usize != index);
            self.baseline_values.direct_generator_cells.remove(&index);
            Some(generator)
        } else {
            None
        };
        let released_cold_generator = if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_COLD_GENERATOR
        {
            if slot.aux == 0 {
                return Err(format!(
                    "cold native Generator {index} lost its stable identity"
                ));
            }
            // SAFETY: cold Generator publication created exactly one
            // boxed identity and final direct-slot release reclaims it.
            #[allow(unsafe_code)]
            let generator =
                unsafe { Box::from_raw(slot.aux as usize as *mut php_runtime::api::GeneratorRef) };
            self.baseline_values
                .direct_generator_handles
                .retain(|_, mapped| *mapped as usize != index);
            Some(generator)
        } else {
            None
        };
        let released_cold_iterator = if matches!(
            slot.kind,
            php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
                | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR
        ) {
            if slot.aux == 0 {
                return Err(format!(
                    "cold native iterator {index} lost its stable record"
                ));
            }
            // SAFETY: iterator publication created exactly one boxed record
            // and final direct-slot release reclaims it once.
            #[allow(unsafe_code)]
            Some(unsafe { Box::from_raw(slot.aux as usize as *mut NativeColdIterator) })
        } else {
            None
        };
        if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_SHARED_ARRAY
            && !release_baseline_shared_array_storage(slot.payload as usize)
        {
            return Err(format!(
                "shared native array {index} storage was already released"
            ));
        }
        let freed_string_range = if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_STRING {
            let base = self.direct_string_bytes.as_ptr() as usize;
            let address = usize::try_from(slot.aux).unwrap_or(base);
            let start = address.saturating_sub(base);
            let length = usize::try_from(slot.payload).unwrap_or(0);
            if let Some(bytes) = self
                .direct_string_bytes
                .get(start..start.saturating_add(length))
            {
                let hash = native_direct_string_hash(bytes);
                let remove_bucket = self
                    .direct_string_interned_slots
                    .get_mut(&hash)
                    .is_some_and(|indices| {
                        indices.retain(|candidate| *candidate as usize != index);
                        indices.is_empty()
                    });
                if remove_bucket {
                    self.direct_string_interned_slots.remove(&hash);
                }
            }
            let capacity = php_jit::jit_native_direct_string_capacity(slot.reserved) as usize;
            (capacity != 0).then_some((start, capacity))
        } else {
            None
        };
        let (mut children, freed_array_range) =
            if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FOREACH {
                (vec![slot.payload as i64], None)
            } else if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY {
                if let Some(storage_version) =
                    self.baseline_values.direct_array_storage_ids.remove(&index)
                    && self
                        .baseline_values
                        .direct_array_handles
                        .get(&storage_version)
                        == Some(&(index as u32))
                {
                    self.baseline_values
                        .direct_array_handles
                        .remove(&storage_version);
                }
                let length = usize::try_from(slot.payload).unwrap_or(0);
                let base = self.direct_array_entries.as_ptr() as usize;
                let address = usize::try_from(slot.aux).unwrap_or(base);
                let entry_size = std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>();
                let start = address.saturating_sub(base) / entry_size;
                (
                    self.direct_array_entries
                        .get(start..start.saturating_add(length))
                        .unwrap_or_default()
                        .iter()
                        .flat_map(|entry| [entry.key, entry.value])
                        .collect::<Vec<_>>(),
                    Some((start, slot.reserved as usize)),
                )
            } else if slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
                && slot.flags == php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION
                && native_reference_state(slot.reserved)
                    != php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_EMPTY
            {
                (vec![slot.payload as i64], None)
            } else {
                (Vec::new(), None)
            };
        if let Some(callable) = released_callable {
            let view = callable.native_view;
            if view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE {
                if view.flags & php_jit::JIT_NATIVE_PREPARED_CLOSURE_HAS_IMPLICIT_THIS != 0 {
                    children.push(view.implicit_this);
                }
                if view.capture_count != 0 && view.captures != 0 {
                    // SAFETY: the callable owner still owns the immutable
                    // boxed capture slice addressed by its native view.
                    #[allow(unsafe_code)]
                    let captures = unsafe {
                        std::slice::from_raw_parts(
                            view.captures as usize as *const i64,
                            view.capture_count as usize,
                        )
                    };
                    children.extend_from_slice(captures);
                }
            } else if view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD {
                children.push(view.receiver);
            }
        }
        if let Some(fiber) = released_fiber {
            children.push(fiber.callable);
            children.extend(fiber.return_value);
        }
        let released_generator_state = released_generator
            .as_ref()
            .and_then(|generator| generator.handle.clone().zip(generator.state));
        if let Some(generator) = released_generator {
            if generator.lifecycle == php_runtime::api::GeneratorState::Created {
                children.extend(generator.arguments);
            }
            children.extend(generator.current_key);
            children.extend(generator.current_value);
            children.extend(generator.return_value);
            if let Some(delegation) = generator.delegation {
                children.push(match delegation {
                    NativeGeneratorDelegation::Array { source, .. } => source,
                    NativeGeneratorDelegation::Generator { generator } => generator,
                });
            }
        }
        if let Some(iterator) = released_cold_iterator {
            match *iterator {
                NativeColdIterator::Array(iterator) => {
                    if let Some(direct) = iterator.direct {
                        children.extend(
                            direct
                                .entries
                                .iter()
                                .flat_map(|entry| [entry.key, entry.value]),
                        );
                    }
                }
                NativeColdIterator::Object(iterator) => {
                    children.push(iterator.source);
                    children.extend(iterator.keys);
                }
                NativeColdIterator::Snapshot(_) | NativeColdIterator::User(_) => {}
                NativeColdIterator::LiveArray(iterator) => {
                    children.push(iterator.source);
                }
                NativeColdIterator::Generator(iterator) => {
                    self.baseline_generator_iterators
                        .remove(&iterator.generator.id());
                    children.extend(iterator.arguments);
                    if let Some(BaselineGeneratorDelegation::Generator { iterator, .. }) =
                        iterator.delegation
                    {
                        children.push(iterator);
                    }
                }
            }
        }
        children.extend(direct_object_children);
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            flags: slot.kind,
            ..php_jit::JitNativeValueSlot::default()
        };
        self.direct_array_states[index] = php_jit::JitNativeDirectArrayState::default();
        self.baseline_values.direct_reference_cells.remove(&index);
        self.baseline_values
            .materialized_direct_references
            .retain(|candidate| *candidate != index);
        if let Some((start, capacity)) = freed_array_range {
            self.free_direct_array_entries(start, capacity);
        }
        if let Some((start, capacity)) = freed_string_range {
            self.free_direct_string_bytes(start, capacity);
        }
        for child in children {
            self.release(child)?;
        }
        if let Some(execution) = released_fiber_execution {
            self.abandon_native_fiber_execution(execution)?;
        }
        if let Some((handle, state)) = released_generator_state {
            self.release_native_suspension_owners(&handle, &state)?;
        }
        if let Some(object) = released_object {
            let class_name = object.class_name();
            if self.object_has_native_destructor(&class_name) {
                let uniquely_owned = object.gc_refcount_estimate() == 1;
                if uniquely_owned {
                    self.record_object_release_root_check(true);
                }
                if uniquely_owned || !self.object_is_request_rooted(object.id()) {
                    self.run_object_destructor(object)?;
                }
            }
        }
        drop(released_resource);
        drop(released_cold_generator);
        let index = u32::try_from(index)
            .map_err(|_| "direct native free-list index overflow".to_owned())?;
        self.direct_value_slots[index as usize].payload = u64::from(*self.direct_value_free_head);
        *self.direct_value_free_head = index;
        Ok(())
    }

    fn release_if_live(&mut self, encoded: i64) -> Result<(), String> {
        if let Some(index) = Self::direct_value_index(encoded) {
            if self.direct_value_slots[index].refcount == 0 {
                return Ok(());
            }
            return self.release_direct_value_index(index);
        }
        let Some(index) = php_jit::jit_decode_runtime_value(encoded) else {
            return Ok(());
        };
        Err(format!(
            "native runtime value {index} is outside the authoritative direct slot plane"
        ))
    }

    fn object_is_request_rooted(&mut self, object_id: u64) -> bool {
        self.consume_native_root_mutation();
        if self.root_index.is_dirty() {
            let reason = self.root_index.last_reason().as_str();
            let roots = self.request_root_values();
            self.root_index.synchronize(&roots);
            self.record_object_release_root_check(false);
            self.record_root_rebuild_reason(reason);
        } else {
            self.record_object_release_root_check(true);
        }
        if self.root_index.contains(object_id) {
            return true;
        }
        self.live_native_values_contain_object(object_id)
    }

    fn live_native_values_contain_object(&self, object_id: u64) -> bool {
        let mut visited = std::collections::HashSet::new();
        let used = usize::try_from(*self.direct_value_next).unwrap_or(0);
        (0..used).any(|index| {
            self.direct_value_slots
                .get(index)
                .is_some_and(|slot| slot.refcount != 0)
                && self.direct_slot_contains_object(index, object_id, &mut visited)
        })
    }

    fn direct_slot_contains_object(
        &self,
        index: usize,
        object_id: u64,
        visited: &mut std::collections::HashSet<usize>,
    ) -> bool {
        if !visited.insert(index) {
            return false;
        }
        let Some(slot) = self
            .direct_value_slots
            .get(index)
            .copied()
            .filter(|slot| slot.refcount != 0)
        else {
            return false;
        };
        match slot.kind {
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT => {
                let Some(object) = self.direct_object(index) else {
                    return false;
                };
                if object.id() == object_id {
                    return true;
                }
                let mut cold_property_contains = false;
                object.visit_property_values(|value| {
                    cold_property_contains |= values_contain_object([value], object_id);
                });
                if cold_property_contains {
                    return true;
                }
                if !php_jit::jit_native_object_property_view_is_published(slot.flags) {
                    return false;
                }
                let Some((base, count)) = object.native_declared_slots_view(slot.payload) else {
                    return false;
                };
                // SAFETY: publication installs one boxed slot slice and keeps
                // it immovable until the descriptor is demoted. This scan is
                // synchronous on the owning request thread and performs no
                // mutation or cold conversion while the slice is borrowed.
                #[allow(unsafe_code)]
                let properties = unsafe { std::slice::from_raw_parts(base, count) };
                properties.iter().any(|property| {
                    property.initialized != 0
                        && self.encoded_value_contains_object(property.value, object_id, visited)
                })
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY => {
                let length = usize::try_from(slot.payload).unwrap_or(0);
                let base = self.direct_array_entries.as_ptr() as usize;
                let address = usize::try_from(slot.aux).unwrap_or(base);
                let entry_size = std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>();
                if address < base || !(address - base).is_multiple_of(entry_size) {
                    return false;
                }
                let start = (address - base) / entry_size;
                self.direct_array_entries
                    .get(start..start.saturating_add(length))
                    .is_some_and(|entries| {
                        entries.iter().any(|entry| {
                            self.encoded_value_contains_object(entry.key, object_id, visited)
                                || self.encoded_value_contains_object(
                                    entry.value,
                                    object_id,
                                    visited,
                                )
                        })
                    })
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
            | php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FOREACH => {
                self.encoded_value_contains_object(slot.payload as i64, object_id, visited)
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE => {
                self.direct_prepared_callable_view(index)
                    .is_some_and(|view| {
                        if view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD {
                            return self.encoded_value_contains_object(
                                view.receiver,
                                object_id,
                                visited,
                            );
                        }
                        if view.kind != php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE {
                            return false;
                        }
                        if view.flags & php_jit::JIT_NATIVE_PREPARED_CLOSURE_HAS_IMPLICIT_THIS != 0
                            && self.encoded_value_contains_object(
                                view.implicit_this,
                                object_id,
                                visited,
                            )
                        {
                            return true;
                        }
                        if view.capture_count == 0 || view.captures == 0 {
                            return false;
                        }
                        // SAFETY: the live callable owner holds the immutable
                        // capture allocation for the lifetime of this view.
                        #[allow(unsafe_code)]
                        let captures = unsafe {
                            std::slice::from_raw_parts(
                                view.captures as usize as *const i64,
                                view.capture_count as usize,
                            )
                        };
                        captures.iter().copied().any(|value| {
                            self.encoded_value_contains_object(value, object_id, visited)
                        })
                    })
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_FIBER
            | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_FIBER => {
                let native_contains = self.fiber_record(index).is_some_and(|fiber| {
                    self.encoded_value_contains_object(fiber.callable, object_id, visited)
                        || fiber.return_value.is_some_and(|value| {
                            self.encoded_value_contains_object(value, object_id, visited)
                        })
                });
                native_contains
                    || self
                        .baseline_values
                        .direct_fiber_cells
                        .get(&index)
                        .is_some_and(|fiber| {
                            let callable = fiber.callable();
                            values_contain_object([&callable], object_id)
                                || fiber
                                    .return_value()
                                    .is_some_and(|value| values_contain_object([&value], object_id))
                        })
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_GENERATOR
            | php_jit::JIT_NATIVE_VALUE_VIEW_MATERIALIZED_GENERATOR => {
                self.direct_generator(index).is_some_and(|generator| {
                    generator
                        .arguments
                        .iter()
                        .copied()
                        .any(|value| self.encoded_value_contains_object(value, object_id, visited))
                        || generator.current_key.is_some_and(|value| {
                            self.encoded_value_contains_object(value, object_id, visited)
                        })
                        || generator.current_value.is_some_and(|value| {
                            self.encoded_value_contains_object(value, object_id, visited)
                        })
                        || generator.return_value.is_some_and(|value| {
                            self.encoded_value_contains_object(value, object_id, visited)
                        })
                        || generator.delegation.as_ref().is_some_and(|delegation| {
                            let value = match delegation {
                                NativeGeneratorDelegation::Array { source, .. } => *source,
                                NativeGeneratorDelegation::Generator { generator } => *generator,
                            };
                            self.encoded_value_contains_object(value, object_id, visited)
                        })
                        || generator.state.as_ref().is_some_and(|state| {
                            state
                                .slots
                                .iter()
                                .take(state.slot_count as usize)
                                .enumerate()
                                .any(|(index, value)| {
                                    state.local_initialized(php_ir::LocalId::new(
                                        u32::try_from(index).unwrap_or(u32::MAX),
                                    )) && self
                                        .encoded_value_contains_object(*value, object_id, visited)
                                })
                                || state.registers.iter().enumerate().any(|(index, value)| {
                                    state.initialized_register_mask & (1_u64 << index) != 0
                                        && self.encoded_value_contains_object(
                                            *value, object_id, visited,
                                        )
                                })
                        })
                })
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT
            | php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR => self
                .cold_iterator(index)
                .is_some_and(|iterator| match iterator {
                    NativeColdIterator::Array(iterator) => values_contain_object(
                        iterator.source.iter().map(|(_, value)| value),
                        object_id,
                    ),
                    NativeColdIterator::Object(iterator) => iterator.object.id() == object_id,
                    NativeColdIterator::Snapshot(iterator) => values_contain_object(
                        iterator
                            .entries
                            .iter()
                            .flat_map(|(key, value)| [key, value]),
                        object_id,
                    ),
                    NativeColdIterator::LiveArray(iterator) => {
                        self.encoded_value_contains_object(iterator.source, object_id, visited)
                    }
                    NativeColdIterator::User(iterator) => iterator.object.id() == object_id,
                    NativeColdIterator::Generator(iterator) => iterator
                        .delegation
                        .as_ref()
                        .is_some_and(|delegation| match delegation {
                            BaselineGeneratorDelegation::Array { entries, .. } => {
                                values_contain_object(
                                    entries.iter().flat_map(|(key, value)| [key, value]),
                                    object_id,
                                )
                            }
                            BaselineGeneratorDelegation::Generator { .. } => false,
                        }),
                }),
            php_jit::JIT_NATIVE_VALUE_VIEW_SHARED_ARRAY
            | php_jit::JIT_NATIVE_VALUE_VIEW_BORROWED_REFERENCE_ARRAY => {
                baseline_shared_array_storage_contains_object(slot.payload as usize, object_id)
            }
            _ => false,
        }
    }

    fn encoded_value_contains_object(
        &self,
        encoded: i64,
        object_id: u64,
        visited: &mut std::collections::HashSet<usize>,
    ) -> bool {
        if let Some(index) = Self::direct_value_index(encoded) {
            return self.direct_slot_contains_object(index, object_id, visited);
        }
        false
    }

    fn run_object_destructor(&mut self, object: php_runtime::api::ObjectRef) -> Result<(), String> {
        if self
            .destroyed_objects
            .get(&object.id())
            .is_some_and(WeakObjectHandle::is_alive)
        {
            return Ok(());
        }
        enum DestructorTarget {
            Local(php_ir::FunctionId),
            External(NativeDynamicFunction),
        }
        let class_name = object.class_name();
        let destructor = self
            .unit
            .classes
            .iter()
            .find(|class| class.name == normalize_class_name(&class_name))
            .and_then(|class| {
                class
                    .methods
                    .iter()
                    .find(|method| method.name.eq_ignore_ascii_case("__destruct"))
            })
            .map(|method| DestructorTarget::Local(method.function))
            .or_else(|| {
                native_external_method(self, &class_name, "__destruct")
                    .map(|(function, _)| DestructorTarget::External(function))
            });
        let Some(destructor) = destructor else {
            return Ok(());
        };
        self.destroyed_objects
            .insert(object.id(), object.weak_handle());
        let receiver = self.encode_native_object_owner(object)?;
        let invoked = match destructor {
            DestructorTarget::Local(function) => invoke_native_method(self, function, &[receiver])
                .map(|_| ())
                .map_err(String::from),
            DestructorTarget::External(function) => invoke_native_external_function(
                self,
                function,
                &[receiver],
                Some(class_name),
                self.unit.strict_types,
            )
            .map(|_| ())
            .map_err(String::from),
        };
        let released = self.release(receiver);
        invoked.and(released)
    }

    fn object_has_native_destructor(&self, class_name: &str) -> bool {
        self.unit
            .classes
            .iter()
            .find(|class| class.name == normalize_class_name(class_name))
            .is_some_and(|class| {
                class
                    .methods
                    .iter()
                    .any(|method| method.name.eq_ignore_ascii_case("__destruct"))
            })
            || native_external_method(self, class_name, "__destruct").is_some()
    }

    fn function_id(&self, name: &str) -> Option<php_ir::FunctionId> {
        self.unit
            .function_table
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
            .map(|entry| entry.function)
            .or_else(|| {
                self.dynamic_functions.get(name).copied().or_else(|| {
                    name.bytes()
                        .any(|byte| byte.is_ascii_uppercase())
                        .then(|| name.to_ascii_lowercase())
                        .and_then(|normalized| self.dynamic_functions.get(&normalized).copied())
                })
            })
    }

    fn visible_include_function_names(&self) -> Rc<NativeFunctionNameScope> {
        self.visible_function_names.clone()
    }

    fn publish_function_names(&mut self, names: impl IntoIterator<Item = String>) {
        self.visible_function_names =
            NativeFunctionNameScope::child(self.visible_function_names.clone(), names);
    }

    fn demote_all_direct_objects(&mut self) -> Result<(), String> {
        let native_objects = (0..usize::try_from(*self.direct_value_next).unwrap_or(0))
            .filter_map(|index| {
                self.direct_value_slots
                    .get(index)
                    .is_some_and(|slot| {
                        slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
                            && php_jit::jit_native_object_property_view_is_published(slot.flags)
                    })
                    .then(|| self.direct_object_owner(index))
                    .flatten()
                    .filter(|object| {
                        object
                            .native_declared_slots_view(object.class_layout_epoch())
                            .is_some()
                    })
            })
            .collect::<Vec<_>>();
        for object in native_objects {
            self.materialize_direct_object_alias(&object)?;
        }
        Ok(())
    }

    fn take_include_symbols(&mut self) -> Result<NativeIncludeSymbols, String> {
        self.demote_trusted_static_properties();
        self.materialize_trusted_static_locals()?;
        // Include/eval hands Rust-owned request state to a separately owned
        // native arena. No ObjectRef crossing that ownership boundary may
        // retain declared-property slots encoded against this arena.
        self.demote_all_direct_objects()?;
        self.mark_roots_dirty(RootMutationReason::GlobalOrStatic);
        let NativeRegisteredCallbackTransfer {
            autoload_callbacks,
            shutdown_callbacks,
            error_handlers,
            exception_handlers,
        } = self.take_registered_callback_transfer()?;
        Ok(NativeIncludeSymbols {
            deployment_functions: std::sync::Arc::clone(&self.deployment_functions),
            deployment_classes: std::sync::Arc::clone(&self.deployment_classes),
            external_functions: std::mem::take(&mut self.external_functions),
            external_class_units: std::mem::take(&mut self.external_class_units),
            external_signature_epoch: self.external_signature_epoch,
            dynamic_units: std::mem::take(&mut self.dynamic_units),
            dynamic_classes: std::mem::take(&mut self.dynamic_classes),
            class_aliases: std::mem::take(&mut self.class_aliases),
            autoload_callbacks,
            shutdown_callbacks,
            static_property_transfer: std::mem::take(
                &mut self.baseline_values.static_property_transfer,
            ),
            typed_static_reference_constraints: std::mem::take(
                &mut self.typed_static_reference_constraints,
            ),
            static_locals: std::mem::take(&mut self.baseline_values.static_locals),
            enum_cases: std::mem::take(&mut self.baseline_values.enum_cases),
            destroyed_objects: std::mem::take(&mut self.destroyed_objects),
            error_reporting: Some(self.error_reporting),
            display_errors: Some(self.display_errors),
            error_handlers,
            exception_handlers,
            last_error: self.last_error.take(),
        })
    }

    fn detach_transient_include_unit(&mut self) -> Result<(), String> {
        if !self.include_child {
            return Ok(());
        }
        let unit = self
            .current_dynamic_unit
            .take()
            .ok_or_else(|| "include execution unit was not attached".to_owned())?;
        if unit + 1 != self.dynamic_units.len()
            || self.dynamic_units.get(unit).is_none_or(|package| {
                package.compiled.artifact_identity() != self.compiled.artifact_identity()
            })
        {
            self.current_dynamic_unit = Some(unit);
            return Err("include execution unit publication is inconsistent".to_owned());
        }
        self.dynamic_units
            .pop()
            .ok_or_else(|| "include execution unit disappeared".to_owned())?;
        cold_dynamic_units::refresh_linked_function_records(self);
        Ok(())
    }

    fn restore_include_symbols(&mut self, mut symbols: NativeIncludeSymbols) -> Result<(), String> {
        for package in &mut symbols.dynamic_units {
            package.reset_runtime_publication();
        }
        self.deployment_functions = symbols.deployment_functions;
        self.deployment_classes = symbols.deployment_classes;
        self.external_functions = symbols.external_functions;
        self.external_class_units = symbols.external_class_units;
        self.external_signature_epoch = symbols.external_signature_epoch;
        self.dynamic_units = symbols.dynamic_units;
        self.dynamic_classes = symbols.dynamic_classes;
        self.class_aliases = symbols.class_aliases;
        let registered_callbacks = NativeRegisteredCallbackTransfer {
            autoload_callbacks: std::mem::take(&mut symbols.autoload_callbacks),
            shutdown_callbacks: std::mem::take(&mut symbols.shutdown_callbacks),
            error_handlers: std::mem::take(&mut symbols.error_handlers),
            exception_handlers: std::mem::take(&mut symbols.exception_handlers),
        };
        self.baseline_values.static_property_transfer = symbols.static_property_transfer;
        self.typed_static_reference_constraints = symbols.typed_static_reference_constraints;
        self.baseline_values.static_locals = symbols.static_locals;
        self.baseline_values.enum_cases = symbols.enum_cases;
        self.destroyed_objects = symbols.destroyed_objects;
        if let Some(error_reporting) = symbols.error_reporting {
            self.error_reporting = error_reporting;
        }
        if let Some(display_errors) = symbols.display_errors {
            self.display_errors = display_errors;
        }
        self.last_error = symbols.last_error;
        self.restore_registered_callback_transfer(registered_callbacks)?;
        self.prepare_trusted_static_properties();
        self.prepare_trusted_static_locals();
        self.mark_roots_dirty(RootMutationReason::GlobalOrStatic);
        self.republish_transferred_dynamic_units()
    }

    fn external_function(&self, name: &str) -> Option<NativeDynamicFunction> {
        self.external_functions.get(name).copied().or_else(|| {
            let normalized = name
                .bytes()
                .any(|byte| byte.is_ascii_uppercase())
                .then(|| name.to_ascii_lowercase());
            normalized
                .as_deref()
                .and_then(|normalized| self.external_functions.get(normalized).copied())
                .or_else(|| {
                    let normalized = normalized.as_deref().unwrap_or(name);
                    self.deployment_functions
                        .get(normalized)
                        .copied()
                        .map(|function| NativeDynamicFunction { unit: 0, function })
                })
        })
    }

    fn can_invoke_external_in_place(&self, target: NativeDynamicFunction) -> bool {
        self.dynamic_units.get(target.unit).is_some_and(|package| {
            package
                .compiled
                .unit()
                .functions
                .get(target.function.index())
                .is_some()
        })
    }

    fn stabilize_active_dynamic_global_roots(&mut self, unit: usize) -> Result<(), String> {
        let names = self
            .dynamic_units
            .get(unit)
            .map(|package| package.cross_unit_global_names.clone())
            .ok_or_else(|| "dynamic native unit is missing".to_owned())?;
        let mut roots = names
            .iter()
            .filter_map(|name| self.native_global_reference_handles.get(name).copied())
            .collect::<Vec<_>>();
        roots.sort_unstable();
        roots.dedup();
        self.stabilize_owned_native_values_for_cross_unit(&mut roots)
    }

    fn replace_active_unit_runtime_state(
        &mut self,
        replacement: NativeUnitRuntimeState,
    ) -> NativeUnitRuntimeState {
        let NativeUnitRuntimeState {
            prepared_native_metadata_functions,
            trusted_request_local_function_offsets,
            trusted_request_local_slots,
            trusted_property_function_offsets,
            trusted_property_slots,
            trusted_closure_plans,
            trusted_exception_plans,
            trusted_exception_plan_owners,
            trusted_constant_slots,
            trusted_dynamic_constant_sites,
            trusted_global_reference_slots,
            trusted_global_reference_names,
            trusted_static_local_slots,
            trusted_static_property_slots,
            trusted_instanceof_plans,
            trusted_instanceof_entries,
            trusted_exception_route_plans,
            trusted_exception_route_entries,
            trusted_exception_route_symbol_epoch,
            trusted_class_plans,
        } = replacement;
        NativeUnitRuntimeState {
            prepared_native_metadata_functions: std::mem::replace(
                &mut self.prepared_native_metadata_functions,
                prepared_native_metadata_functions,
            ),
            trusted_request_local_function_offsets: std::mem::replace(
                &mut self.trusted_request_local_function_offsets,
                trusted_request_local_function_offsets,
            ),
            trusted_request_local_slots: std::mem::replace(
                &mut self.trusted_request_local_slots,
                trusted_request_local_slots,
            ),
            trusted_property_function_offsets: std::mem::replace(
                &mut self.trusted_property_function_offsets,
                trusted_property_function_offsets,
            ),
            trusted_property_slots: std::mem::replace(
                &mut self.trusted_property_slots,
                trusted_property_slots,
            ),
            trusted_closure_plans: std::mem::replace(
                &mut self.trusted_closure_plans,
                trusted_closure_plans,
            ),
            trusted_exception_plans: std::mem::replace(
                &mut self.trusted_exception_plans,
                trusted_exception_plans,
            ),
            trusted_exception_plan_owners: std::mem::replace(
                &mut self.trusted_exception_plan_owners,
                trusted_exception_plan_owners,
            ),
            trusted_constant_slots: std::mem::replace(
                &mut self.trusted_constant_slots,
                trusted_constant_slots,
            ),
            trusted_dynamic_constant_sites: std::mem::replace(
                &mut self.trusted_dynamic_constant_sites,
                trusted_dynamic_constant_sites,
            ),
            trusted_global_reference_slots: std::mem::replace(
                &mut self.trusted_global_reference_slots,
                trusted_global_reference_slots,
            ),
            trusted_global_reference_names: std::mem::replace(
                &mut self.trusted_global_reference_names,
                trusted_global_reference_names,
            ),
            trusted_static_local_slots: std::mem::replace(
                &mut self.trusted_static_local_slots,
                trusted_static_local_slots,
            ),
            trusted_static_property_slots: std::mem::replace(
                &mut self.trusted_static_property_slots,
                trusted_static_property_slots,
            ),
            trusted_instanceof_plans: std::mem::replace(
                &mut self.trusted_instanceof_plans,
                trusted_instanceof_plans,
            ),
            trusted_instanceof_entries: std::mem::replace(
                &mut self.trusted_instanceof_entries,
                trusted_instanceof_entries,
            ),
            trusted_exception_route_plans: std::mem::replace(
                &mut self.trusted_exception_route_plans,
                trusted_exception_route_plans,
            ),
            trusted_exception_route_entries: std::mem::replace(
                &mut self.trusted_exception_route_entries,
                trusted_exception_route_entries,
            ),
            trusted_exception_route_symbol_epoch: std::mem::replace(
                &mut self.trusted_exception_route_symbol_epoch,
                trusted_exception_route_symbol_epoch,
            ),
            trusted_class_plans: std::mem::replace(
                &mut self.trusted_class_plans,
                trusted_class_plans,
            ),
        }
    }

    fn republish_transferred_dynamic_units(&mut self) -> Result<(), String> {
        let units = self.dynamic_units.len();
        for unit in 0..units {
            self.with_active_dynamic_unit(unit, None, |_| ())?;
        }
        cold_dynamic_units::refresh_linked_function_records(self);
        Ok(())
    }

    fn with_active_dynamic_unit<R>(
        &mut self,
        unit: usize,
        request_local_bindings: Option<&[(String, i64)]>,
        operation: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, String> {
        if self.current_dynamic_unit == Some(unit)
            && self.dynamic_units.get(unit).is_some_and(|package| {
                package.compiled.artifact_identity() == self.compiled.artifact_identity()
            })
        {
            let previous_execution_scope = self.current_native_execution_scope;
            let active_scope_matches = usize::try_from(previous_execution_scope)
                .ok()
                .and_then(|identity| identity.checked_sub(1))
                .and_then(|index| self.native_execution_scopes.get(index))
                .is_some_and(|scope| scope.unit == Some(unit));
            if !active_scope_matches {
                let mut scope = usize::try_from(previous_execution_scope)
                    .ok()
                    .and_then(|identity| identity.checked_sub(1))
                    .and_then(|index| self.native_execution_scopes.get(index))
                    .map_or(
                        NativeExecutionScope {
                            unit: Some(unit),
                            called_class: None,
                            scope_class: None,
                        },
                        |scope| scope.as_ref().clone(),
                    );
                scope.unit = Some(unit);
                self.current_native_execution_scope =
                    self.register_native_execution_scope(scope)?;
            }
            let binding_result = request_local_bindings.map_or(Ok(()), |bindings| {
                self.publish_active_entry_request_local_bindings(bindings)
            });
            let _runtime_view = activate_native_context(self);
            let result = binding_result.map(|()| operation(self));
            self.current_native_execution_scope = previous_execution_scope;
            return result;
        }
        let (compiled, active_entries, active_runtime_state) = {
            let package = self
                .dynamic_units
                .get_mut(unit)
                .ok_or_else(|| "dynamic native unit is missing".to_owned())?;
            (
                package.compiled.clone(),
                std::mem::take(&mut package.native_entries),
                std::mem::take(&mut package.runtime_state),
            )
        };
        let previous_dynamic_unit = self.current_dynamic_unit;
        let previous_execution_scope = self.current_native_execution_scope;
        let previous_compiled = std::mem::replace(&mut self.compiled, compiled.clone());
        let previous_unit = std::mem::replace(&mut self.unit, ActiveNativeUnit::new(&compiled));
        let previous_identity =
            std::mem::replace(&mut self.unit_identity, compiled.artifact_identity());
        let previous_entries = std::mem::replace(&mut self.native_entries, active_entries);
        let previous_runtime_state = self.replace_active_unit_runtime_state(active_runtime_state);
        let mut detached_previous = Some((previous_entries, previous_runtime_state));
        if let Some(previous) = previous_dynamic_unit {
            let (previous_entries, previous_runtime_state) = detached_previous
                .take()
                .expect("previous active native unit state was already stored");
            let package = self
                .dynamic_units
                .get_mut(previous)
                .ok_or_else(|| "active native unit package is missing".to_owned())?;
            package.native_entries = previous_entries;
            package.runtime_state = previous_runtime_state;
        }
        self.current_dynamic_unit = Some(unit);
        let active_scope_matches = usize::try_from(self.current_native_execution_scope)
            .ok()
            .and_then(|identity| identity.checked_sub(1))
            .and_then(|index| self.native_execution_scopes.get(index))
            .is_some_and(|scope| scope.unit == Some(unit));
        if !active_scope_matches {
            let mut scope = usize::try_from(self.current_native_execution_scope)
                .ok()
                .and_then(|identity| identity.checked_sub(1))
                .and_then(|index| self.native_execution_scopes.get(index))
                .map_or(
                    NativeExecutionScope {
                        unit: Some(unit),
                        called_class: None,
                        scope_class: None,
                    },
                    |scope| scope.as_ref().clone(),
                );
            scope.unit = Some(unit);
            self.current_native_execution_scope = self.register_native_execution_scope(scope)?;
        }
        self.prepare_trusted_literal_slots();
        self.prepare_trusted_closure_plans();
        self.prepare_trusted_exception_plans();
        self.prepare_trusted_static_properties();
        self.prepare_trusted_constant_fetches();
        self.prepare_trusted_request_locals();
        let binding_result = request_local_bindings.map_or(Ok(()), |bindings| {
            self.publish_active_entry_request_local_bindings(bindings)
        });
        let global_binding_result = self.prepare_trusted_global_references();
        self.prepare_trusted_static_locals();
        self.prepare_trusted_class_plans();
        self.prepare_trusted_declared_properties();
        self.prepare_trusted_instanceof_plans();
        self.prepare_trusted_exception_routes();
        let prepared_functions = self.all_published_native_functions();
        self.prepared_native_metadata_functions
            .extend(prepared_functions);

        // Native code in an included/eval unit uses that unit's dense trusted
        // function-cell table. The outer request activation describes the
        // root deployment; refresh the by-value runtime view for the scoped
        // unit before constructing any nested JitDeoptState. Without this,
        // FunctionId N from an include indexed root FunctionId N and could
        // indirect-call arbitrary data as an address.
        let _runtime_view = activate_native_context(self);
        let result = binding_result
            .and(global_binding_result)
            .map(|()| operation(self));
        let root_stabilization = if result.is_ok() {
            self.stabilize_active_dynamic_global_roots(unit)
        } else {
            Ok(())
        };

        let active_runtime_state =
            self.replace_active_unit_runtime_state(NativeUnitRuntimeState::default());
        let active_entries = std::mem::take(&mut self.native_entries);
        {
            let package = self
                .dynamic_units
                .get_mut(unit)
                .expect("active dynamic native unit disappeared");
            package.native_entries = active_entries;
            package.runtime_state = active_runtime_state;
        }
        match previous_dynamic_unit {
            Some(previous) => {
                let (previous_entries, previous_runtime_state) = {
                    let package = self
                        .dynamic_units
                        .get_mut(previous)
                        .expect("previous native unit package disappeared");
                    (
                        std::mem::take(&mut package.native_entries),
                        std::mem::take(&mut package.runtime_state),
                    )
                };
                self.native_entries = previous_entries;
                let empty = self.replace_active_unit_runtime_state(previous_runtime_state);
                debug_assert!(
                    empty.trusted_property_function_offsets.is_empty(),
                    "inactive native unit left an unexpected runtime state installed"
                );
            }
            None => {
                let (previous_entries, previous_runtime_state) = detached_previous
                    .take()
                    .expect("detached native unit state is missing");
                self.native_entries = previous_entries;
                let empty = self.replace_active_unit_runtime_state(previous_runtime_state);
                debug_assert!(
                    empty.trusted_property_function_offsets.is_empty(),
                    "inactive native unit left an unexpected runtime state installed"
                );
            }
        }
        self.current_dynamic_unit = previous_dynamic_unit;
        self.current_native_execution_scope = previous_execution_scope;
        self.unit_identity = previous_identity;
        self.unit = previous_unit;
        self.compiled = previous_compiled;
        if self.trusted_exception_route_symbol_epoch != self.external_signature_epoch {
            self.prepare_trusted_exception_routes();
        }
        root_stabilization?;
        result
    }

    fn publish_active_entry_request_local_bindings(
        &mut self,
        bindings: &[(String, i64)],
    ) -> Result<(), String> {
        let entry = self.unit.entry;
        let locals = self
            .unit
            .functions
            .get(entry.index())
            .map(|function| function.locals.clone())
            .ok_or_else(|| "dynamic unit entry function is missing".to_owned())?;
        let base = self
            .trusted_request_local_function_offsets
            .get(entry.index())
            .copied()
            .and_then(|base| usize::try_from(base).ok())
            .ok_or_else(|| "dynamic unit entry local slots are missing".to_owned())?;
        for (name, encoded) in bindings {
            let Some(local) = locals.iter().position(|candidate| candidate == name) else {
                continue;
            };
            if self.php_handle_is_reference(*encoded) != Some(true) {
                return Err(format!(
                    "dynamic unit entry local ${name} has no native reference identity"
                ));
            }
            let index = base
                .checked_add(local)
                .ok_or_else(|| "dynamic unit entry local slot overflow".to_owned())?;
            let previous = self
                .trusted_request_local_slots
                .get(index)
                .copied()
                .ok_or_else(|| format!("dynamic unit entry local ${name} slot is missing"))?;
            self.retain(*encoded)?;
            self.trusted_request_local_slots[index] = php_jit::JitNativeRequestLocalSlot {
                encoded: *encoded,
                state: php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED,
                reserved: 0,
            };
            if previous.state == php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED
                && let Err(error) = self.release(previous.encoded)
            {
                return Err(error);
            }
        }
        Ok(())
    }

    fn duplicate_active_entry_request_local(
        &mut self,
        name: &str,
        preserve_reference: bool,
    ) -> Result<Option<i64>, String> {
        let entry = self.unit.entry;
        let local = self.unit.functions.get(entry.index()).and_then(|function| {
            function
                .locals
                .iter()
                .position(|candidate| candidate == name)
        });
        let Some(local) = local else {
            return Ok(None);
        };
        let Some(index) = self
            .trusted_request_local_function_offsets
            .get(entry.index())
            .copied()
            .and_then(|base| usize::try_from(base).ok())
            .and_then(|base| base.checked_add(local))
        else {
            return Err(format!("dynamic unit entry local ${name} slot is missing"));
        };
        let slot = self
            .trusted_request_local_slots
            .get(index)
            .copied()
            .filter(|slot| slot.state == php_jit::JIT_NATIVE_REQUEST_LOCAL_PUBLISHED)
            .ok_or_else(|| format!("dynamic unit entry local ${name} is unpublished"))?;
        if preserve_reference {
            self.duplicate_authoritative_native_value(slot.encoded)
        } else {
            self.duplicate_authoritative_dereferenced_native_value(slot.encoded)
        }
    }

    fn direct_array_slot(&self, encoded: i64) -> Option<(usize, php_jit::JitNativeValueSlot)> {
        let index = Self::direct_value_index(encoded)?;
        let slot = *self.direct_value_slots.get(index)?;
        (slot.refcount != 0 && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY)
            .then_some((index, slot))
    }

    /// Resolves a direct-reference chain to its authoritative direct array.
    /// The returned encoding owns no additional reference; callers either
    /// borrow entries or retain the selected child explicitly.
    fn direct_array_encoding(&self, encoded: i64) -> Option<i64> {
        let encoded = self.dereference_direct_encoding(encoded);
        self.direct_array_slot(encoded).map(|_| encoded)
    }

    fn direct_array_entry_range(&self, encoded: i64) -> Option<(usize, usize)> {
        let encoded = self.dereference_direct_encoding(encoded);
        let (_, slot) = self.direct_array_slot(encoded)?;
        let length = usize::try_from(slot.payload).ok()?;
        let base = self.direct_array_entries.as_ptr() as usize;
        let address = usize::try_from(slot.aux).ok()?;
        let entry_size = std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>();
        let offset = address.checked_sub(base)?;
        (offset % entry_size == 0).then_some(())?;
        let start = offset / entry_size;
        (start.checked_add(length)? <= self.direct_array_entries.len()).then_some((start, length))
    }

    fn direct_array_entries_for(
        &self,
        encoded: i64,
    ) -> Option<&[php_jit::JitNativeDirectArrayEntry]> {
        let (start, length) = self.direct_array_entry_range(encoded)?;
        self.direct_array_entries
            .get(start..start.checked_add(length)?)
    }

    /// Reads one authoritative direct-array entry without copying the whole
    /// array into a temporary compatibility vector.
    ///
    /// Call binding may need mutable access to the request while it walks an
    /// argument array (warnings, reference preparation, or target dispatch).
    /// Returning the plain ABI record by value keeps no slice borrow alive
    /// across those operations and preserves the stable native array as the
    /// only argument representation.
    fn direct_array_entry_at(
        &self,
        start: usize,
        index: usize,
    ) -> php_jit::JitNativeDirectArrayEntry {
        self.direct_array_entries[start + index]
    }

    /// Rewrites unit-indexed constants embedded in an authoritative native
    /// ownership graph before that graph crosses an IR-unit boundary.
    ///
    /// Arrays are not the only possible carrier: references, declared object
    /// slots, and prepared closure captures can all own an array (or a literal)
    /// that is later read in another unit. Walk those native owners in place;
    /// no Rust `Value`, `PhpArray`, or compatibility facade participates.
    fn stabilize_direct_array_for_cross_unit(&mut self, encoded: i64) -> Result<(), String> {
        let mut visited = std::collections::BTreeSet::new();
        self.stabilize_cross_unit_graph_value(encoded, &mut visited)?;
        Ok(())
    }

    fn stabilize_cross_unit_graph_value(
        &mut self,
        encoded: i64,
        visited: &mut std::collections::BTreeSet<usize>,
    ) -> Result<i64, String> {
        self.consume_native_root_mutation();
        let encoded = self.stabilize_cross_unit_value(encoded)?;
        let Some(index) = Self::direct_value_index(encoded) else {
            return Ok(encoded);
        };
        if self.cross_unit_stable_values.contains(&index) {
            return Ok(encoded);
        }
        if !visited.insert(index) {
            return Ok(encoded);
        }
        let slot = self
            .direct_value_slots
            .get(index)
            .copied()
            .filter(|slot| slot.refcount != 0)
            .ok_or_else(|| format!("direct native value {index} is missing"))?;
        match slot.kind {
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY => {
                let length = usize::try_from(slot.payload)
                    .map_err(|_| format!("direct native array {index} length overflow"))?;
                let base = self.direct_array_entries.as_ptr() as usize;
                let address = usize::try_from(slot.aux)
                    .map_err(|_| format!("direct native array {index} address overflow"))?;
                let entry_size = std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>();
                let offset = address
                    .checked_sub(base)
                    .ok_or_else(|| format!("direct native array {index} is outside its arena"))?;
                if offset % entry_size != 0 {
                    return Err(format!("direct native array {index} address is unaligned"));
                }
                let start = offset / entry_size;
                let end = start
                    .checked_add(length)
                    .ok_or_else(|| format!("direct native array {index} range overflow"))?;
                if end > self.direct_array_entries.len() {
                    return Err(format!(
                        "direct native array {index} entries are outside its arena"
                    ));
                }
                for entry_index in start..end {
                    let entry = self.direct_array_entries[entry_index];
                    let key = self.stabilize_cross_unit_graph_value(entry.key, visited)?;
                    let value = self.stabilize_cross_unit_graph_value(entry.value, visited)?;
                    self.direct_array_entries[entry_index] =
                        php_jit::JitNativeDirectArrayEntry { key, value };
                }
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
                if slot.flags == php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION
                    && native_reference_state(slot.reserved)
                        != php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_EMPTY =>
            {
                let payload =
                    self.stabilize_cross_unit_graph_value(slot.payload as i64, visited)?;
                self.direct_value_slots[index].payload = payload as u64;
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
                if php_jit::jit_native_object_property_view_is_published(slot.flags) =>
            {
                let object = self
                    .direct_object(index)
                    .ok_or_else(|| format!("direct native object {index} has no stable owner"))?;
                let (base, count) =
                    object
                        .native_declared_slots_view(slot.payload)
                        .ok_or_else(|| {
                            format!("direct native object {index} lost its declared slots")
                        })?;
                for property_index in 0..count {
                    // SAFETY: the object owns one immovable native slot slice
                    // for this layout. This request-thread walk neither
                    // demotes the object nor changes the slice allocation.
                    #[allow(unsafe_code)]
                    let property = unsafe { *base.add(property_index) };
                    if property.initialized == 0 {
                        continue;
                    }
                    let value = self.stabilize_cross_unit_graph_value(property.value, visited)?;
                    #[allow(unsafe_code)]
                    unsafe {
                        *base.add(property_index) =
                            php_runtime::api::NativeDeclaredPropertySlot { value, ..property };
                    }
                }
            }
            php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE => {
                let view = self.direct_prepared_callable_view(index).copied();
                let children = view.map_or_else(Vec::new, |view| {
                    if view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD {
                        return vec![view.receiver];
                    }
                    if view.kind != php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE {
                        return Vec::new();
                    }
                    let mut children = Vec::with_capacity(
                        view.capture_count as usize
                            + usize::from(
                                view.flags & php_jit::JIT_NATIVE_PREPARED_CLOSURE_HAS_IMPLICIT_THIS
                                    != 0,
                            ),
                    );
                    if view.flags & php_jit::JIT_NATIVE_PREPARED_CLOSURE_HAS_IMPLICIT_THIS != 0 {
                        children.push(view.implicit_this);
                    }
                    if view.capture_count != 0 && view.captures != 0 {
                        // SAFETY: the live callable owner holds this immutable
                        // capture allocation until final slot release.
                        #[allow(unsafe_code)]
                        let captures = unsafe {
                            std::slice::from_raw_parts(
                                view.captures as usize as *const i64,
                                view.capture_count as usize,
                            )
                        };
                        children.extend_from_slice(captures);
                    }
                    children
                });
                let stabilized = children
                    .into_iter()
                    .map(|value| self.stabilize_cross_unit_graph_value(value, visited))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut published_implicit_this = None;
                let published_receiver = view
                    .filter(|view| {
                        view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_BOUND_OBJECT_METHOD
                    })
                    .and_then(|_| stabilized.first().copied());
                if view.is_some_and(|view| view.kind == php_jit::JIT_NATIVE_CALLABLE_KIND_CLOSURE)
                    && let Some(closure) = self.direct_prepared_closure_mut(index)
                {
                    let mut values = stabilized.into_iter();
                    if closure.implicit_this.is_some() {
                        closure.implicit_this = values.next();
                    }
                    for capture in &mut closure.captures {
                        *capture = values
                            .next()
                            .expect("closure capture stabilization kept its arity");
                    }
                    closure.native_view.implicit_this =
                        closure.implicit_this.unwrap_or_else(|| {
                            php_jit::jit_encode_constant(php_jit::JIT_VALUE_UNINITIALIZED)
                        });
                    published_implicit_this = Some(closure.native_view.implicit_this);
                }
                if published_implicit_this.is_some() || published_receiver.is_some() {
                    let owner = slot.aux as usize as *mut NativePreparedCallableOwner;
                    // SAFETY: this is the same request-owned record validated
                    // by direct_prepared_callable_mut above. The authoritative
                    // C view must be refreshed together with its explicitly
                    // cold Closure metadata.
                    #[allow(unsafe_code)]
                    unsafe {
                        if let Some(implicit_this) = published_implicit_this {
                            (*owner).native_view.implicit_this = implicit_this;
                        }
                        if let Some(receiver) = published_receiver {
                            (*owner).native_view.receiver = receiver;
                        }
                    }
                }
            }
            _ => {}
        }
        self.cross_unit_stable_values.insert(index);
        Ok(encoded)
    }

    /// Rehomes only unit-indexed immediates before already-owned native frame
    /// values cross into another IR unit. Direct values keep the same owner;
    /// direct arrays keep the same COW identity while their embedded constant
    /// indexes are stabilized in place.
    fn stabilize_owned_native_values_for_cross_unit(
        &mut self,
        values: &mut [i64],
    ) -> Result<(), String> {
        for encoded in values {
            let unit_local_constant = php_jit::jit_decode_constant(*encoded).is_some_and(|index| {
                index != u32::MAX
                    && index != php_jit::JIT_VALUE_UNINITIALIZED
                    && index != php_jit::JIT_VALUE_FALSE
                    && index != php_jit::JIT_VALUE_TRUE
            });
            if unit_local_constant || Self::direct_value_index(*encoded).is_some() {
                let mut visited = std::collections::BTreeSet::new();
                *encoded = self.stabilize_cross_unit_graph_value(*encoded, &mut visited)?;
            }
        }
        Ok(())
    }

    fn stabilize_cross_unit_value(&mut self, encoded: i64) -> Result<i64, String> {
        let Some(constant) = php_jit::jit_decode_constant(encoded) else {
            return Ok(encoded);
        };
        if matches!(
            constant,
            u32::MAX
                | php_jit::JIT_VALUE_UNINITIALIZED
                | php_jit::JIT_VALUE_FALSE
                | php_jit::JIT_VALUE_TRUE
        ) {
            return Ok(encoded);
        }
        self.stabilize_active_unit_constant(constant)
    }

    fn direct_array_length(&self, encoded: i64) -> Option<usize> {
        self.direct_array_entries_for(encoded).map(<[_]>::len)
    }

    fn direct_array_is_unique(&self, encoded: i64) -> Option<bool> {
        self.direct_array_slot(encoded)
            .map(|(_, slot)| slot.refcount == 1)
    }

    fn direct_array_can_append(&self, encoded: i64) -> Option<bool> {
        let (index, _) = self.direct_array_slot(encoded)?;
        let state = self.direct_array_states.get(index)?;
        let next = if state.has_next_append_key != 0 {
            state.next_append_key
        } else {
            0
        };
        if next != i64::MAX {
            return Some(true);
        }
        Some(
            !self
                .direct_array_entries_for(encoded)?
                .iter()
                .any(|entry| self.native_encoded_int(entry.key) == Some(i64::MAX)),
        )
    }

    fn fresh_direct_array_next_append_key(
        &self,
        entries: &[php_jit::JitNativeDirectArrayEntry],
    ) -> Option<i64> {
        entries
            .iter()
            .filter_map(|entry| self.native_encoded_int(entry.key))
            .map(|key| key.saturating_add(1))
            .max()
    }

    fn direct_array_find_encoded(
        &self,
        encoded: i64,
        key: &php_runtime::api::ArrayKey,
    ) -> Result<Option<i64>, String> {
        let Some(entries) = self.direct_array_entries_for(encoded) else {
            return Err("native value is not a direct array".to_owned());
        };
        Ok(entries
            .iter()
            .find(|entry| self.native_encoded_matches_array_key(entry.key, key))
            .map(|entry| entry.value))
    }

    /// Binds one entry of an authoritative direct array as a PHP reference.
    ///
    /// The direct array remains the only array representation: its entry owns
    /// one reference handle and the returned handle is an independent owner
    /// for the callee. A shared array is deliberately rejected here because
    /// its COW replacement must also update the containing lvalue.
    fn bind_native_direct_array_element_reference(
        &mut self,
        encoded: i64,
        key: &php_runtime::api::ArrayKey,
    ) -> Result<Option<i64>, String> {
        let Some(array) = self.direct_array_encoding(encoded) else {
            return Ok(None);
        };
        if self.direct_array_is_unique(array) != Some(true) {
            return Ok(None);
        }
        if let Some(current) = self.direct_array_find_encoded(array, key)?
            && self.php_handle_is_reference(current) == Some(true)
        {
            self.retain(current)?;
            return Ok(Some(current));
        }

        let payload = self
            .direct_array_find_encoded(array, key)?
            .unwrap_or_else(|| php_jit::jit_encode_constant(u32::MAX));
        // Preserve the entry's current owner until direct_array_insert_encoded
        // has installed and retained the reference. The retained payload then
        // moves into the new reference descriptor.
        self.retain(payload)?;
        let reference = match self.encode_direct_reference_payload_owned(payload) {
            Ok(reference) => reference,
            Err(error) => {
                self.release(payload)?;
                return Err(error);
            }
        };
        if let Err(error) = self.direct_array_insert_encoded(array, Some(key), reference) {
            self.release(reference)?;
            return Err(error);
        }
        Ok(Some(reference))
    }

    /// Collapses a reference created solely for one array-walk callback.
    ///
    /// The array entry is restored only when it remains the reference's sole
    /// owner. A callback-exported alias raises the reference count and keeps
    /// the shared PHP identity intact.
    fn collapse_native_direct_array_element_reference(
        &mut self,
        encoded: i64,
        key: &php_runtime::api::ArrayKey,
        reference: i64,
    ) -> Result<bool, String> {
        let Some(array) = self.direct_array_encoding(encoded) else {
            return Ok(false);
        };
        let Some((array_index, array_slot)) = self.direct_array_slot(array) else {
            return Ok(false);
        };
        if array_slot.refcount != 1 {
            return Ok(false);
        }
        let Some(index) = Self::direct_value_index(reference) else {
            return Ok(false);
        };
        let Some(slot) = self.direct_value_slots.get(index).copied() else {
            return Ok(false);
        };
        if slot.refcount != 1
            || slot.kind != php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR
            || slot.flags != php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION
            || native_reference_state(slot.reserved)
                == php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_EMPTY
        {
            return Ok(false);
        }
        let payload = slot.payload as i64;
        let (start, length) = self
            .direct_array_entry_range(array)
            .ok_or_else(|| "native array-walk cleanup lost its array entries".to_owned())?;
        let Some(entry_index) = (start..start.saturating_add(length)).find(|entry_index| {
            let entry = self.direct_array_entries[*entry_index];
            entry.value == reference && self.native_encoded_matches_array_key(entry.key, key)
        }) else {
            return Ok(false);
        };

        // Ordinary assignment to a reference-valued array entry must preserve
        // its PHP identity, so `direct_array_insert_encoded` deliberately
        // replaces the reference payload instead of the entry. This operation
        // is different: it ends the internal reference created solely for one
        // array-walk callback. Transfer the payload owner directly to the
        // array entry, then retire the now-unreachable reference descriptor.
        self.retain(payload)?;
        self.direct_array_entries[entry_index].value = payload;
        if let Err(error) = self.release(reference) {
            self.direct_array_entries[entry_index].value = reference;
            self.release(payload)?;
            return Err(error);
        }
        self.cross_unit_stable_values.remove(&array_index);
        Ok(true)
    }

    /// Publishes a newly produced native array whose entry handles are already
    /// individually owned by the caller. Ownership moves into the resulting
    /// slot; no Rust `PhpArray` or duplicate value tree is constructed.
    #[track_caller]
    fn publish_owned_direct_array_entries(
        &mut self,
        entries: Vec<php_jit::JitNativeDirectArrayEntry>,
    ) -> Result<i64, String> {
        let next_append_key = self.fresh_direct_array_next_append_key(&entries);
        let release_entries =
            |context: &mut Self, entries: &[php_jit::JitNativeDirectArrayEntry]| {
                for entry in entries {
                    let _ = context.release(entry.key);
                    let _ = context.release(entry.value);
                }
            };
        let (start, capacity) = match self.reserve_direct_array_entries(entries.len()) {
            Ok(range) => range,
            Err(error) => {
                release_entries(self, &entries);
                return Err(error);
            }
        };
        self.direct_array_entries[start..start + entries.len()].copy_from_slice(&entries);
        let index = match self.reserve_direct_value_slot() {
            Ok(index) => index,
            Err(error) => {
                self.free_direct_array_entries(start, capacity);
                release_entries(self, &entries);
                return Err(error);
            }
        };
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
            flags: php_jit::jit_native_direct_array_flags(None),
            reserved: u32::try_from(capacity).unwrap_or(u32::MAX),
            payload: entries.len() as u64,
            aux: self.direct_array_entries[start..].as_ptr() as usize as u64,
        };
        self.direct_array_states[index] = php_jit::JitNativeDirectArrayState {
            next_append_key: next_append_key.unwrap_or(0),
            has_next_append_key: u32::from(next_append_key.is_some()),
            reserved: 0,
        };
        self.record_direct_array_materialization(entries.len(), std::panic::Location::caller());
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .ok_or_else(|| "direct native value handle overflow".to_owned())?;
        Ok((php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG | u64::from(runtime_index)) as i64)
    }

    #[track_caller]
    fn clone_direct_array_handle(&mut self, encoded: i64) -> Result<i64, String> {
        let (_, source_slot) = self
            .direct_array_slot(encoded)
            .ok_or_else(|| "native value is not a direct array".to_owned())?;
        let source_index = Self::direct_value_index(encoded)
            .ok_or_else(|| "native value is not a direct array".to_owned())?;
        let source_state = self.direct_array_states[source_index];
        let entries = self
            .direct_array_entries_for(encoded)
            .ok_or_else(|| "direct native array entries are unavailable".to_owned())?
            .to_vec();
        let (start, capacity) = self.reserve_direct_array_entries(entries.len())?;
        let mut retained = Vec::with_capacity(entries.len() * 2);
        for entry in &entries {
            for child in [entry.key, entry.value] {
                if let Err(error) = self.retain(child) {
                    for child in retained {
                        let _ = self.release(child);
                    }
                    self.free_direct_array_entries(start, capacity);
                    return Err(error);
                }
                retained.push(child);
            }
        }
        self.direct_array_entries[start..start + entries.len()].copy_from_slice(&entries);
        let index = match self.reserve_direct_value_slot() {
            Ok(index) => index,
            Err(error) => {
                for child in retained {
                    let _ = self.release(child);
                }
                self.free_direct_array_entries(start, capacity);
                return Err(error);
            }
        };
        self.direct_value_slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
            flags: source_slot.flags,
            reserved: u32::try_from(capacity).unwrap_or(u32::MAX),
            payload: entries.len() as u64,
            aux: self.direct_array_entries[start..].as_ptr() as usize as u64,
        };
        self.direct_array_states[index] = source_state;
        self.record_direct_array_materialization(entries.len(), std::panic::Location::caller());
        let runtime_index = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE))
            .ok_or_else(|| "direct native value handle overflow".to_owned())?;
        Ok((php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG | u64::from(runtime_index)) as i64)
    }

    fn direct_array_insert_encoded(
        &mut self,
        encoded: i64,
        key: Option<&php_runtime::api::ArrayKey>,
        value: i64,
    ) -> Result<(), String> {
        let (array_index, mut slot) = self
            .direct_array_slot(encoded)
            .ok_or_else(|| "native value is not a direct array".to_owned())?;
        self.cross_unit_stable_values.remove(&array_index);
        if slot.refcount != 1 {
            return Err("direct native array write requires unique ownership".to_owned());
        }
        if key.is_none() && self.direct_array_can_append(encoded) == Some(false) {
            return Err(php_runtime::api::PHP_ARRAY_APPEND_OVERFLOW_MESSAGE.to_owned());
        }
        let length = usize::try_from(slot.payload)
            .map_err(|_| "direct native array length overflow".to_owned())?;
        let base = self.direct_array_entries.as_ptr() as usize;
        let address = usize::try_from(slot.aux)
            .map_err(|_| "direct native array address overflow".to_owned())?;
        let entry_size = std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>();
        let offset = address
            .checked_sub(base)
            .ok_or_else(|| "direct native array address is outside its arena".to_owned())?;
        if offset % entry_size != 0 {
            return Err("direct native array address is unaligned".to_owned());
        }
        let mut start = offset / entry_size;
        let normalized_key = match key {
            Some(key) => key.clone(),
            None => {
                let state = self.direct_array_states[array_index];
                php_runtime::api::ArrayKey::Int(if state.has_next_append_key != 0 {
                    state.next_append_key
                } else {
                    0
                })
            }
        };
        let entries = self
            .direct_array_entries
            .get(start..start.saturating_add(length))
            .ok_or_else(|| "direct native array entries are outside its arena".to_owned())?
            .to_vec();
        let mut existing = None;
        for (position, entry) in entries.iter().enumerate() {
            if self.native_encoded_matches_array_key(entry.key, &normalized_key) {
                existing = Some(position);
                break;
            }
        }
        if let Some(position) = existing {
            let entry_index = start + position;
            let previous = self.direct_array_entries[entry_index].value;
            if self.php_handle_is_reference(previous) == Some(true)
                && self.php_handle_is_reference(value) == Some(false)
            {
                let replacement = self.duplicate_dereferenced_native_value(value)?;
                if self.replace_direct_reference_payload_owned(previous, replacement)? {
                    return Ok(());
                }
                // A cold boundary may have materialized this exact reference
                // identity. Republish its payload once, then perform the
                // assignment in the authoritative native slot; never decode
                // both operands and rebuild their graphs here.
                if let Err(error) = self.restore_authoritative_direct_reference(previous) {
                    self.release(replacement)?;
                    return Err(error);
                }
                if self.replace_direct_reference_payload_owned(previous, replacement)? {
                    return Ok(());
                }
                self.release(replacement)?;
                return Err(
                    "native array reference entry could not republish its direct payload"
                        .to_owned(),
                );
            }
            self.retain(value)?;
            self.direct_array_entries[entry_index].value = value;
            self.release(previous)?;
            return Ok(());
        }

        let encoded_key = self.encode_native_array_key_owned(&normalized_key)?;
        if let Err(error) = self.retain(value) {
            let _ = self.release(encoded_key);
            return Err(error);
        }
        let capacity = slot.reserved as usize;
        if length == capacity {
            let (new_start, new_capacity) = match self.reserve_direct_array_entries(length + 1) {
                Ok(range) => range,
                Err(error) => {
                    let _ = self.release(encoded_key);
                    let _ = self.release(value);
                    return Err(error);
                }
            };
            self.direct_array_entries
                .copy_within(start..start + length, new_start);
            self.free_direct_array_entries(start, capacity);
            start = new_start;
            slot.reserved = u32::try_from(new_capacity).unwrap_or(u32::MAX);
            slot.aux = self.direct_array_entries[start..].as_ptr() as usize as u64;
        }
        self.direct_array_entries[start + length] = php_jit::JitNativeDirectArrayEntry {
            key: encoded_key,
            value,
        };
        if let php_runtime::api::ArrayKey::Int(key) = normalized_key {
            let next = key.saturating_add(1);
            let state = &mut self.direct_array_states[array_index];
            if state.has_next_append_key == 0 || next > state.next_append_key {
                state.next_append_key = next;
            }
            state.has_next_append_key = 1;
        }
        slot.payload = (length + 1) as u64;
        self.direct_value_slots[array_index] = slot;
        Ok(())
    }

    /// Removes one entry from a uniquely owned authoritative direct array.
    ///
    /// The caller performs encoded-handle COW first. Keeping removal in the
    /// direct entry plane is important for by-value call parameters: mutating
    /// a shared request slot would otherwise write through into the caller
    /// even though PHP requires the callee to observe an independent array
    /// value.
    fn direct_array_remove_encoded(
        &mut self,
        encoded: i64,
        key: &php_runtime::api::ArrayKey,
    ) -> Result<(), String> {
        let (array_index, mut slot) = self
            .direct_array_slot(encoded)
            .ok_or_else(|| "native value is not a direct array".to_owned())?;
        self.cross_unit_stable_values.remove(&array_index);
        if slot.refcount != 1 {
            return Err("direct native array removal requires unique ownership".to_owned());
        }
        let length = usize::try_from(slot.payload)
            .map_err(|_| "direct native array length overflow".to_owned())?;
        let Some(position) = self
            .direct_array_entries_for(encoded)
            .ok_or_else(|| "direct native array entries are unavailable".to_owned())?
            .iter()
            .position(|entry| self.native_encoded_matches_array_key(entry.key, key))
        else {
            return Ok(());
        };
        let base = self.direct_array_entries.as_ptr() as usize;
        let address = usize::try_from(slot.aux)
            .map_err(|_| "direct native array address overflow".to_owned())?;
        let entry_size = std::mem::size_of::<php_jit::JitNativeDirectArrayEntry>();
        let offset = address
            .checked_sub(base)
            .ok_or_else(|| "direct native array address is outside its arena".to_owned())?;
        if offset % entry_size != 0 {
            return Err("direct native array address is unaligned".to_owned());
        }
        let start = offset / entry_size;
        let removed = self.direct_array_entries[start + position];
        self.release(removed.key)?;
        self.release(removed.value)?;
        self.direct_array_entries
            .copy_within(start + position + 1..start + length, start + position);
        let new_length = length - 1;
        self.direct_array_entries[start + new_length] =
            php_jit::JitNativeDirectArrayEntry { key: 0, value: 0 };

        let cursor = php_jit::jit_native_direct_array_cursor(slot.flags)
            .and_then(|cursor| usize::try_from(cursor).ok())
            .filter(|cursor| *cursor < length)
            .and_then(|cursor| {
                if cursor > position {
                    Some(cursor - 1)
                } else if cursor == position && position >= new_length {
                    None
                } else {
                    Some(cursor)
                }
            })
            .and_then(|cursor| u32::try_from(cursor).ok());
        slot.flags = php_jit::jit_native_direct_array_flags(cursor);
        slot.payload = new_length as u64;
        self.direct_value_slots[array_index] = slot;
        Ok(())
    }

    fn publish_direct_object_slots(
        &mut self,
        object: i64,
        property: &str,
        _value: i64,
        function: i64,
        continuation: i64,
        state: u32,
    ) -> Result<(), String> {
        if !matches!(
            state,
            php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_PUBLISHED
                | php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_WRITABLE
                | php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_REFERENCEABLE
                | php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_DIMENSION_WRITABLE
        ) {
            return Err(format!("invalid trusted property slot state {state}"));
        }
        let direct_index = |context: &Self| {
            let direct_object = context.dereference_direct_encoding(object);
            Self::direct_value_index(direct_object).filter(|index| {
                context.direct_value_slots.get(*index).is_some_and(|slot| {
                    slot.refcount != 0 && slot.kind == php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT
                })
            })
        };
        let index = if let Some(index) = direct_index(self) {
            index
        } else {
            // A cold continuation may have materialized the receiver
            // reference. Restore that exact descriptor and follow its native
            // payload; do not reconstruct or re-encode the object graph.
            self.restore_authoritative_direct_reference(object)?;
            let Some(index) = direct_index(self) else {
                return Ok(());
            };
            index
        };
        let published = (|| {
            if !self.promote_direct_object_property_slots(index)? {
                return Ok(());
            }
            let object = self
                .direct_object(index)
                .ok_or_else(|| format!("direct native object {index} has no stable owner"))?;
            let class_name = object.class_name();
            let prepared = self.prepared_native_runtime_class(&class_name);
            let caller_function = u32::try_from(function)
                .map_err(|_| "trusted property function index overflow".to_owned())?;
            let declaration =
                native_instance_property_declaration(self, &class_name, property, caller_function);
            let state_is_stable = declaration.as_ref().is_some_and(|declaration| {
                let entry = &declaration.entry;
                let readable =
                    native_instance_property_readable(self, declaration, caller_function);
                let writable =
                    native_instance_property_writable(self, declaration, caller_function);
                let mutable = prepared
                    .as_ref()
                    .is_some_and(|class| !class.entry.flags.is_readonly)
                    && !entry.flags.is_readonly;
                match state {
                    php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_PUBLISHED => {
                        readable && entry.hooks.get.is_none()
                    }
                    php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_WRITABLE
                    | php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_REFERENCEABLE => {
                        readable
                            && writable
                            && mutable
                            && !entry.flags.is_typed
                            && entry.type_.is_none()
                            && entry.hooks.get.is_none()
                            && entry.hooks.set.is_none()
                    }
                    php_jit::JIT_NATIVE_TRUSTED_PROPERTY_SLOT_DIMENSION_WRITABLE => {
                        readable
                            && writable
                            && mutable
                            && entry.hooks.get.is_none()
                            && entry.hooks.set.is_none()
                    }
                    _ => false,
                }
            });
            if !state_is_stable {
                return Ok(());
            }
            let Some(slot_index) = object.declared_slot_index(property) else {
                return Ok(());
            };
            let function = usize::try_from(caller_function)
                .map_err(|_| "trusted property function index overflow".to_owned())?;
            let continuation = usize::try_from(
                u32::try_from(continuation)
                    .map_err(|_| "trusted property continuation index overflow".to_owned())?,
            )
            .map_err(|_| "trusted property continuation index overflow".to_owned())?;
            let Some(base) = self
                .trusted_property_function_offsets
                .get(function)
                .copied()
                .and_then(|base| usize::try_from(base).ok())
            else {
                return Ok(());
            };
            let Some(plan) = self
                .trusted_property_slots
                .get_mut(base.saturating_add(continuation))
            else {
                return Ok(());
            };
            *plan = php_jit::JitNativeTrustedPropertySlot {
                state,
                slot_index,
                layout_id: object.class_layout_epoch(),
            };
            Ok(())
        })();
        published
    }

    fn register_native_execution_scope(
        &mut self,
        scope: NativeExecutionScope,
    ) -> Result<u32, String> {
        if let Some(index) = self
            .native_execution_scopes
            .iter()
            .position(|candidate| candidate.as_ref() == &scope)
        {
            return u32::try_from(index + 1)
                .map_err(|_| "native execution scope identity overflow".to_owned());
        }
        self.native_execution_scopes.push(Box::new(scope));
        u32::try_from(self.native_execution_scopes.len())
            .map_err(|_| "native execution scope identity overflow".to_owned())
    }

    fn native_execution_target_from_state(
        &self,
        state: &php_jit::JitDeoptState,
        fallback: Option<&NativeExecutionTarget>,
    ) -> Result<NativeExecutionTarget, String> {
        let runtime_view = state.active_runtime_view();
        let identity = runtime_view.fiber_execution_scope;
        let index = usize::try_from(identity)
            .ok()
            .and_then(|identity| identity.checked_sub(1))
            .ok_or_else(|| "suspended native activation has no execution scope".to_owned())?;
        let recorded = self
            .native_execution_scopes
            .get(index)
            .ok_or_else(|| format!("suspended native execution scope {identity} is missing"))?;
        let function_entries = runtime_view.trusted_function_entries;
        let inferred_unit = self
            .dynamic_units
            .iter()
            .enumerate()
            .find_map(|(unit, package)| {
                let entries = package
                    .compiled
                    .prepared_deployment_image()
                    .native_function_entries
                    .as_ptr() as usize as u64;
                (entries == function_entries).then_some(Some(unit))
            })
            .or_else(|| {
                let entries = self
                    .compiled
                    .prepared_deployment_image()
                    .native_function_entries
                    .as_ptr() as usize as u64;
                (self.current_dynamic_unit.is_none() && entries == function_entries).then_some(None)
            })
            .unwrap_or(recorded.unit);
        // Function IDs are dense only within one compiled unit. A nested
        // linked callee may therefore have the same numeric ID as its caller;
        // the captured runtime view, not that coincidental ID, owns the
        // continuation and its baseline metadata.
        let same_activation = fallback.is_some_and(|fallback| {
            fallback.function.raw() == state.function_id && fallback.unit == inferred_unit
        });
        let scope = fallback
            .filter(|fallback| {
                same_activation
                    || (recorded.unit != inferred_unit && fallback.unit == inferred_unit)
            })
            .map_or_else(|| recorded.as_ref().clone(), NativeExecutionTarget::scope);
        let inferred_unit = if same_activation {
            fallback.and_then(|fallback| fallback.unit)
        } else {
            inferred_unit
        };
        Ok(NativeExecutionTarget {
            unit: inferred_unit,
            function: php_ir::FunctionId::new(state.function_id),
            called_class: scope.called_class.clone(),
            scope_class: scope.scope_class.clone(),
        })
    }

    fn run_in_native_execution_target<R, E>(
        &mut self,
        target: &NativeExecutionTarget,
        operation: impl FnOnce(&mut Self) -> Result<R, E>,
    ) -> Result<R, E>
    where
        E: From<String>,
    {
        let identity = self
            .register_native_execution_scope(target.scope())
            .map_err(E::from)?;
        let previous_identity =
            std::mem::replace(&mut self.current_native_execution_scope, identity);
        let push_called_class = target
            .called_class
            .as_ref()
            .is_some_and(|called_class| self.called_classes.last() != Some(called_class));
        if push_called_class {
            self.called_classes.push(
                target
                    .called_class
                    .as_ref()
                    .expect("called class was classified above")
                    .clone(),
            );
        }
        let push_scope_class = target.scope_class.as_ref().is_some_and(|scope_class| {
            self.lexical_scope_classes.last().map(String::as_str) != Some(scope_class.as_ref())
        });
        if push_scope_class {
            self.lexical_scope_classes.push(
                target
                    .scope_class
                    .as_ref()
                    .expect("scope class was classified above")
                    .to_string(),
            );
        }

        let target_is_active = match target.unit {
            Some(unit) => {
                self.current_dynamic_unit == Some(unit)
                    && self.dynamic_units.get(unit).is_some_and(|package| {
                        package.compiled.artifact_identity() == self.compiled.artifact_identity()
                    })
            }
            None => self.current_dynamic_unit.is_none(),
        };
        let result = if target_is_active {
            let _runtime_view = activate_native_context(self);
            operation(self)
        } else {
            match target.unit {
                Some(unit) => self
                    .with_active_dynamic_unit(unit, None, operation)
                    .map_err(E::from)?,
                None => Err(E::from(format!(
                    "root native execution target {} cannot run inside dynamic unit {:?}",
                    target.function.raw(),
                    self.current_dynamic_unit,
                ))),
            }
        };

        if push_scope_class {
            self.lexical_scope_classes.pop();
        }
        if push_called_class {
            self.called_classes.pop();
        }
        self.current_native_execution_scope = previous_identity;
        result
    }

    fn duplicate_direct_generator_value(&mut self, encoded: i64) -> Result<i64, String> {
        self.duplicate_authoritative_native_value(encoded)?
            .ok_or_else(|| {
                format!(
                    "direct Generator value {} crossed from baseline storage",
                    self.native_encoded_type_name(encoded)
                )
            })
    }

    fn replace_direct_generator_current_owned(
        &mut self,
        index: usize,
        key: Option<i64>,
        value: i64,
        forwarded: bool,
    ) -> Result<(i64, i64), String> {
        let (old_key, old_value, key) = {
            let generator = self
                .direct_generator_mut(index)
                .ok_or_else(|| format!("direct Generator {index} is missing"))?;
            let key = if forwarded {
                key.unwrap_or_else(|| php_jit::jit_encode_constant(u32::MAX))
            } else if let Some(key) = key {
                if let Some(explicit) = (php_jit::jit_decode_runtime_value(key).is_none()
                    && php_jit::jit_decode_constant(key).is_none())
                .then_some(key)
                    && explicit >= generator.next_auto_key
                {
                    generator.next_auto_key = explicit.saturating_add(1);
                }
                key
            } else {
                let key = generator.next_auto_key;
                generator.next_auto_key = generator.next_auto_key.saturating_add(1);
                key
            };
            let old_key = generator.current_key.replace(key);
            let old_value = generator.current_value.replace(value);
            generator.lifecycle = php_runtime::api::GeneratorState::Suspended;
            generator.yields_seen = generator.yields_seen.saturating_add(1);
            (old_key, old_value, key)
        };
        if let Some(old_key) = old_key {
            self.release(old_key)?;
        }
        if let Some(old_value) = old_value {
            self.release(old_value)?;
        }
        let output_key = self.duplicate_direct_generator_value(key)?;
        match self.duplicate_direct_generator_value(value) {
            Ok(output_value) => Ok((output_key, output_value)),
            Err(error) => {
                self.release(output_key)?;
                Err(error)
            }
        }
    }

    fn instruction_for_continuation(
        &self,
        function: u32,
        continuation: u32,
    ) -> Option<NativeInstructionPtr> {
        self.prepared_continuation_instructions(php_ir::FunctionId::new(function))
            .and_then(|instructions| instructions.get(continuation as usize).cloned())
            .flatten()
            .map(|instruction| NativeInstructionPtr(std::sync::Arc::as_ptr(&instruction)))
    }

    pub(super) fn instruction_kind_debug(&self, function: u32, continuation: u32) -> String {
        self.instruction_for_continuation(function, continuation)
            .map(|instruction| format!("{:?}", instruction.kind))
            .unwrap_or_else(|| "<missing continuation>".to_owned())
    }

    fn prepared_native_callsite(
        &self,
        function: u32,
        continuation: u32,
    ) -> Option<*const crate::compiled_unit::NativeCallSiteDescriptor> {
        self.compiled
            .prepared_native_callsites(php_ir::FunctionId::new(function))
            .and_then(|callsites| callsites.get(continuation as usize).cloned())
            .flatten()
            .map(|descriptor| std::sync::Arc::as_ptr(&descriptor))
    }

    fn deferred_function_argument_requires_reference(
        &self,
        function: u32,
        continuation: u32,
        argument: usize,
    ) -> Option<bool> {
        let callsites = self
            .compiled
            .prepared_native_callsites(php_ir::FunctionId::new(function))?;
        let descriptor = callsites
            .get(continuation as usize)
            .and_then(Option::as_deref)?;
        if !matches!(
            descriptor.kind,
            crate::compiled_unit::NativeCallSiteKind::Function
        ) {
            return None;
        }
        let name = descriptor.target_symbol.as_deref()?;
        let parameters = if let Some(function) = self.function_id(name) {
            self.unit
                .functions
                .get(function.index())
                .map(|function| function.params.as_slice())
        } else if let Some(target) = self.external_function(name) {
            self.dynamic_units
                .get(target.unit)
                .and_then(|unit| unit.compiled.unit().functions.get(target.function.index()))
                .map(|function| function.params.as_slice())
        } else {
            None
        }?;
        baseline_call_dispatch::native_function_argument_requires_reference_at(
            descriptor.arguments.as_ref(),
            parameters,
            argument,
        )
    }

    fn native_method_table_epoch(&self) -> u64 {
        let dynamic_epoch = self.dynamic_units.len() as u64;
        self.unit_identity.rotate_left(29) ^ dynamic_epoch
    }

    fn lookup_native_method_pic(
        &self,
        descriptor: &crate::compiled_unit::NativeCallSiteDescriptor,
        receiver_class: &str,
        receiver_layout_id: u64,
        method: &str,
    ) -> Option<NativeMethodPicTarget> {
        let method_table_epoch = self.native_method_table_epoch();
        if let Some((function, is_static)) = descriptor.lookup_method_pic(
            receiver_class,
            method,
            receiver_layout_id,
            method_table_epoch,
        ) {
            return Some(NativeMethodPicTarget::CurrentUnit {
                function,
                is_static,
            });
        }
        let pic = self.native_method_pics.get(&descriptor.pic_slot)?;
        if pic.megamorphic {
            return None;
        }
        pic.entries
            .iter()
            .find(|entry| {
                entry.receiver_class.eq_ignore_ascii_case(receiver_class)
                    && entry.method.eq_ignore_ascii_case(method)
                    && entry.class_layout_epoch == receiver_layout_id
                    && entry.method_table_epoch == method_table_epoch
            })
            .map(|entry| entry.target)
    }

    fn install_native_method_pic(
        &mut self,
        descriptor: &crate::compiled_unit::NativeCallSiteDescriptor,
        receiver_class: &str,
        receiver_layout_id: u64,
        method: &str,
        target: NativeMethodPicTarget,
    ) -> bool {
        let method_table_epoch = self.native_method_table_epoch();
        if let NativeMethodPicTarget::CurrentUnit {
            function,
            is_static,
        } = target
        {
            return descriptor.install_method_pic(
                receiver_class,
                method,
                receiver_layout_id,
                method_table_epoch,
                function,
                is_static,
            );
        }
        if let NativeMethodPicTarget::DynamicUnit {
            function,
            is_static: false,
        } = target
        {
            self.publish_external_method_specialization(
                descriptor,
                receiver_class,
                receiver_layout_id,
                method,
                method_table_epoch,
                function,
            );
        }
        let pic = self
            .native_method_pics
            .entry(descriptor.pic_slot)
            .or_default();
        if pic.megamorphic {
            return false;
        }
        if pic.entries.iter().any(|entry| {
            entry.receiver_class.eq_ignore_ascii_case(receiver_class)
                && entry.method.eq_ignore_ascii_case(method)
                && entry.class_layout_epoch == receiver_layout_id
                && entry.method_table_epoch == method_table_epoch
        }) {
            return true;
        }
        if pic.entries.len() >= NATIVE_METHOD_PIC_LIMIT {
            pic.entries.clear();
            pic.megamorphic = true;
            return false;
        }
        pic.entries.push(NativeMethodPicEntry {
            receiver_class: std::sync::Arc::from(receiver_class),
            method: std::sync::Arc::from(method),
            class_layout_epoch: receiver_layout_id,
            method_table_epoch,
            target,
        });
        true
    }

    fn publish_external_method_specialization(
        &mut self,
        descriptor: &crate::compiled_unit::NativeCallSiteDescriptor,
        receiver_class: &str,
        receiver_layout_id: u64,
        method: &str,
        method_table_epoch: u64,
        target: NativeDynamicFunction,
    ) {
        let Some(link_index) = descriptor.external_method_link_index else {
            return;
        };
        let Some(package) = self.dynamic_units.get(target.unit) else {
            return;
        };
        let Some(function) = package
            .compiled
            .unit()
            .functions
            .get(target.function.index())
        else {
            return;
        };
        if function.returns_by_ref
            || package.published_runtime_view.abi_version != php_jit::JIT_RUNTIME_ABI_VERSION
        {
            return;
        }
        let deployment = package.compiled.prepared_deployment_image();
        let (Some(preferred_entry), Some(baseline_entry)) = (
            deployment
                .preferred_function_entries
                .get(target.function.index()),
            deployment
                .native_function_entries
                .get(target.function.index()),
        ) else {
            return;
        };
        if baseline_entry.load(std::sync::atomic::Ordering::Acquire) == 0 {
            return;
        }
        let signature = php_jit::JitExternalFunctionSignature {
            name: format!("{}::{method}", receiver_class.trim_start_matches('\\')),
            link_index,
            published: true,
            params: function
                .params
                .iter()
                .map(|parameter| php_jit::JitExternalParameterSignature {
                    name: parameter.name.clone(),
                    by_ref: parameter.by_ref,
                    variadic: parameter.variadic,
                })
                .collect(),
            native_params: function.params.clone(),
            native_default_constant_indices: function
                .params
                .iter()
                .map(|parameter| {
                    let default = parameter.default.as_ref()?;
                    package
                        .compiled
                        .unit()
                        .constants
                        .iter()
                        .position(|constant| constant == default)
                        .and_then(|index| u32::try_from(index).ok())
                })
                .collect(),
            native_arity: u32::try_from(function.params.len().saturating_add(1))
                .unwrap_or(u32::MAX),
            requires_non_reference_trampoline: native_function_requires_non_reference_trampoline(
                function, true,
            ),
            returns_by_reference: false,
            exception_routes: native_function_exception_routes(target.function, function),
        };
        let record = php_jit::JitNativeLinkedFunction {
            preferred_entry: std::ptr::from_ref(preferred_entry) as usize as u64,
            baseline_entry: std::ptr::from_ref(baseline_entry) as usize as u64,
            runtime_view: std::ptr::from_ref(package.published_runtime_view.as_ref()) as usize
                as u64,
            prepared_class: 0,
        };
        if !descriptor.install_external_method_pic(
            receiver_class,
            method,
            receiver_layout_id,
            method_table_epoch,
            signature,
        ) {
            return;
        }
        let Some(caller_unit) = self.current_dynamic_unit else {
            return;
        };
        if let Some(slot) = self
            .dynamic_units
            .get_mut(caller_unit)
            .and_then(|package| package.linked_functions.get_mut(link_index as usize))
        {
            *slot = record;
        }
    }
}

impl NativeExecutionDeadlineCapability {
    fn published(context: &mut NativeRequestColdState<'_>) -> Self {
        Self {
            deadline: std::ptr::from_ref(&context.execution_deadline_at),
            diagnostic: std::ptr::from_mut(&mut context.diagnostic),
        }
    }

    /// Checks and publishes only the deadline diagnostic owned by this
    /// capability. No value plane, call frame, unit, or compatibility state
    /// is reachable from the exact poll.
    #[allow(unsafe_code)]
    fn poll(&mut self) -> i32 {
        let Some(deadline) = (unsafe { self.deadline.as_ref() }) else {
            return php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32;
        };
        if deadline.is_none_or(|deadline| std::time::Instant::now() < deadline) {
            return 0;
        }
        let Some(diagnostic) = (unsafe { self.diagnostic.as_mut() }) else {
            return php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32;
        };
        *diagnostic = Some(php_runtime::api::RuntimeDiagnostic::new(
            "E_PHP_VM_EXECUTION_TIMEOUT",
            php_runtime::api::RuntimeSeverity::RecoverableError,
            "maximum execution time exceeded",
            php_runtime::api::RuntimeSourceSpan::default(),
            Vec::new(),
            None,
        ));
        php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32
    }
}

impl NativeFrameArenaCapability {
    fn published(context: &mut NativeRequestColdState<'_>) -> Self {
        Self {
            arena: std::ptr::from_mut(&mut context.native_frame_arena),
            diagnostic: std::ptr::from_mut(&mut context.diagnostic),
        }
    }

    /// Allocates one generated frame from the authoritative native arena.
    ///
    /// Publication guarantees both pointers are valid for the synchronous
    /// request lifetime, so the compiled boundary performs no cold-context
    /// recovery or repeated engine-integrity validation.
    #[allow(unsafe_code)]
    fn allocate(&mut self, bytes: u64, alignment: u64) -> u64 {
        let result = usize::try_from(bytes)
            .map_err(|_| "E_PHP_VM_NATIVE_FRAME_LIMIT: frame size does not fit usize".to_owned())
            .and_then(|bytes| {
                usize::try_from(alignment)
                    .map_err(|_| {
                        "E_PHP_VM_NATIVE_FRAME_ALIGNMENT: alignment does not fit usize".to_owned()
                    })
                    .and_then(|alignment| unsafe { &mut *self.arena }.allocate(bytes, alignment))
            });
        match result {
            Ok(address) => address as u64,
            Err(message) => {
                unsafe {
                    *self.diagnostic = Some(php_runtime::api::RuntimeDiagnostic::new(
                        "E_PHP_VM_NATIVE_FRAME_LIMIT",
                        php_runtime::api::RuntimeSeverity::FatalError,
                        message,
                        php_runtime::api::RuntimeSourceSpan::default(),
                        Vec::new(),
                        None,
                    ));
                }
                0
            }
        }
    }

    #[allow(unsafe_code)]
    fn release(&mut self, address: u64) -> i32 {
        if unsafe { &mut *self.arena }
            .release(address as usize)
            .is_ok()
        {
            0
        } else {
            php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32
        }
    }
}

fn native_publication_constant_is_stable(constant: &php_ir::IrConstant) -> bool {
    match constant {
        php_ir::IrConstant::Null
        | php_ir::IrConstant::Bool(_)
        | php_ir::IrConstant::Int(_)
        | php_ir::IrConstant::Float(_)
        | php_ir::IrConstant::String(_)
        | php_ir::IrConstant::StringBytes(_) => true,
        php_ir::IrConstant::Array(entries) => entries.iter().all(|entry| {
            entry
                .key
                .as_ref()
                .is_none_or(native_publication_constant_is_stable)
                && native_publication_constant_is_stable(&entry.value)
        }),
        php_ir::IrConstant::NamedConstant(_) | php_ir::IrConstant::ClassConstant { .. } => false,
    }
}

fn native_internal_class_is_available(class_name: &str) -> bool {
    php_std::ExtensionRegistry::standard_library()
        .enabled_class(class_name)
        .is_some()
        || matches!(
            class_name,
            "stdclass"
                | "exception"
                | "errorexception"
                | "error"
                | "typeerror"
                | "valueerror"
                | "argumentcounterror"
                | "fibererror"
                | "closure"
                | "generator"
                | "fiber"
                | "arrayobject"
                | "arrayiterator"
        )
}

fn native_class_is_publication_allocatable(
    context: &NativeRequestColdState<'_>,
    owner_unit: Option<usize>,
    class: &php_ir::module::ClassEntry,
) -> bool {
    let owner_ir_unit = |owner: Option<usize>| -> Option<&php_ir::IrUnit> {
        match owner {
            None => Some(&*context.unit),
            Some(unit) => context
                .dynamic_units
                .get(unit)
                .map(|package| package.compiled.unit()),
        }
    };
    let mut current = Some((owner_unit, class));
    let mut visited = std::collections::BTreeSet::new();
    while let Some((owner, candidate)) = current {
        if candidate.flags.is_abstract
            || candidate.flags.is_interface
            || candidate.flags.is_trait
            || candidate.flags.is_enum
            || !visited.insert((owner, candidate.name.as_str()))
        {
            return false;
        }
        let Some(constants) = owner_ir_unit(owner).map(|unit| unit.constants.as_slice()) else {
            return false;
        };
        if candidate.properties.iter().any(|property| {
            property
                .default
                .and_then(|constant| constants.get(constant.index()))
                .is_some_and(|constant| !native_publication_constant_is_stable(constant))
        }) {
            return false;
        }
        current = match candidate.parent.as_deref() {
            None => None,
            Some(parent) => {
                let parent = normalize_class_name(parent);
                if let Some(parent) = owner_ir_unit(owner)
                    .into_iter()
                    .flat_map(|unit| &unit.classes)
                    .find(|class| class.name == parent)
                {
                    Some((owner, parent))
                } else if let Some((unit, parent)) = native_external_class_ref(context, &parent) {
                    Some((Some(unit), parent))
                } else {
                    if !native_internal_class_is_available(&parent) {
                        return false;
                    }
                    None
                }
            }
        };
    }
    true
}

struct NativeStaticPropertyDeclaration {
    owner_unit: Option<usize>,
    owner_name: String,
    owner_display_name: String,
    caller_owns_scope: bool,
    flags: php_ir::module::ClassPropertyFlags,
    default: Option<php_ir::ConstId>,
    has_deferred_default: bool,
    type_: Option<php_ir::IrReturnType>,
}

#[derive(Clone)]
struct NativeInstancePropertyDeclaration {
    owner_unit: Option<usize>,
    owner: crate::compiled_unit::CompiledClass,
    entry: php_ir::module::ClassPropertyEntry,
}

fn native_instance_property_declaration(
    context: &NativeRequestColdState<'_>,
    class_name: &str,
    property: &str,
    caller_function: u32,
) -> Option<NativeInstancePropertyDeclaration> {
    let mut candidate = normalize_class_name(class_name);
    if let Some(caller_name) =
        native_effective_calling_class(context, caller_function).map(|class| class.name.clone())
        && native_class_is_a(context, &candidate, &caller_name)
    {
        let scoped_owner = native_active_class_handle(context, &caller_name).map_or_else(
            || {
                native_external_class_handle(context, &caller_name)
                    .map(|(unit, class)| (Some(unit), class))
            },
            |class| Some((None, class)),
        );
        if let Some((owner_unit, owner)) = scoped_owner
            && let Some(entry) = owner
                .properties
                .iter()
                .find(|entry| {
                    !entry.flags.is_static && entry.flags.is_private && entry.name == property
                })
                .cloned()
        {
            return Some(NativeInstancePropertyDeclaration {
                owner_unit,
                owner,
                entry,
            });
        }
    }
    let mut visited = std::collections::BTreeSet::new();
    while visited.insert(candidate.clone()) {
        let (owner_unit, owner) = native_active_class_handle(context, &candidate).map_or_else(
            || {
                native_external_class_handle(context, &candidate)
                    .map(|(unit, class)| (Some(unit), class))
            },
            |class| Some((None, class)),
        )?;
        if let Some(entry) = owner
            .properties
            .iter()
            .find(|entry| !entry.flags.is_static && entry.name == property)
            .cloned()
        {
            return Some(NativeInstancePropertyDeclaration {
                owner_unit,
                owner,
                entry,
            });
        }
        candidate = normalize_class_name(owner.parent.as_ref()?);
    }
    None
}

fn native_instance_property_readable(
    context: &NativeRequestColdState<'_>,
    declaration: &NativeInstancePropertyDeclaration,
    caller_function: u32,
) -> bool {
    if !declaration.entry.flags.is_private && !declaration.entry.flags.is_protected {
        return true;
    }
    let Some(caller) = native_effective_calling_class(context, caller_function) else {
        return false;
    };
    if declaration.entry.flags.is_private {
        caller.name == declaration.owner.name
    } else {
        native_class_is_a(context, &caller.name, &declaration.owner.name)
    }
}

fn native_instance_property_writable(
    context: &NativeRequestColdState<'_>,
    declaration: &NativeInstancePropertyDeclaration,
    caller_function: u32,
) -> bool {
    if !declaration.entry.flags.set_is_private && !declaration.entry.flags.set_is_protected {
        return true;
    }
    let Some(caller) = native_effective_calling_class(context, caller_function) else {
        return false;
    };
    if declaration.entry.flags.set_is_private {
        caller.name == declaration.owner.name
    } else {
        native_class_is_a(context, &caller.name, &declaration.owner.name)
    }
}

fn native_static_property_declaration(
    context: &NativeRequestColdState<'_>,
    class_name: &str,
    property: &str,
    caller_function: u32,
) -> Option<NativeStaticPropertyDeclaration> {
    let mut candidate = normalize_class_name(class_name);
    let mut visited = std::collections::BTreeSet::new();
    while visited.insert(candidate.clone()) {
        let (unit, class) = if let Some(class) = context
            .unit
            .classes
            .iter()
            .find(|class| class.name == candidate)
        {
            (None, class)
        } else {
            let (unit, class) = native_external_class_ref(context, &candidate)?;
            (Some(unit), class)
        };
        if let Some(entry) = class
            .properties
            .iter()
            .find(|entry| entry.flags.is_static && entry.name == property)
        {
            return Some(NativeStaticPropertyDeclaration {
                owner_unit: unit,
                owner_name: class.name.clone(),
                owner_display_name: class.display_name.clone(),
                caller_owns_scope: class
                    .methods
                    .iter()
                    .any(|method| method.function.raw() == caller_function),
                flags: entry.flags,
                default: entry.default,
                has_deferred_default: entry.default_class_constant.is_some()
                    || entry.default_named_constant.is_some()
                    || entry.default_expr.is_some(),
                type_: entry.type_.clone(),
            });
        }
        candidate = normalize_class_name(class.parent.as_ref()?);
    }
    None
}

fn native_external_method(
    context: &NativeRequestColdState<'_>,
    class_name: &str,
    method: &str,
) -> Option<(NativeDynamicFunction, php_ir::module::ClassMethodEntry)> {
    let (mut unit, mut class) =
        native_external_class_handle(context, class_name).or_else(|| {
            let local = context
                .unit
                .classes
                .iter()
                .find(|class| class.name == normalize_class_name(class_name))?;
            native_external_class_handle(context, local.parent.as_deref()?)
        })?;
    loop {
        if let Some(entry) = class
            .methods
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(method))
            .cloned()
        {
            return Some((
                NativeDynamicFunction {
                    unit,
                    function: entry.function,
                },
                entry,
            ));
        }
        let parent = class.parent.as_deref()?;
        let normalized_parent = normalize_class_name(parent);
        let (parent_unit, parent_class) = context
            .current_dynamic_unit
            .and_then(|unit| {
                context
                    .dynamic_units
                    .get(unit)?
                    .compiled
                    .lookup_unit_class_handle(&normalized_parent)
                    .map(|class| (unit, class))
            })
            .or_else(|| native_external_class_handle(context, parent))?;
        unit = parent_unit;
        class = parent_class;
    }
}

fn create_native_external_object(
    context: &mut NativeRequestColdState<'_>,
    class_name: &str,
    arguments: &[i64],
    source: &php_ir::Instruction,
) -> Result<i64, String> {
    let (unit, class) = native_external_class_handle(context, class_name)
        .ok_or_else(|| format!("E_PHP_VM_UNKNOWN_CLASS: Class {class_name} not found"))?;
    if class.flags.is_abstract
        || class.flags.is_interface
        || class.flags.is_trait
        || class.flags.is_enum
    {
        return Err(format!(
            "Cannot instantiate {} {}",
            class_name, class.display_name
        ));
    }
    native_prepare_runtime_class_constants(context, Some(unit), &class, source)?;
    let object = new_native_object(context, Some(unit), &class)?;
    let receiver = context.encode_native_object_owner(object)?;
    if let Some((constructor, _)) = native_external_method(context, class_name, "__construct") {
        let mut constructor_arguments = Vec::with_capacity(arguments.len() + 1);
        constructor_arguments.push(receiver);
        constructor_arguments.extend_from_slice(arguments);
        let _ = invoke_native_resolved_external_function(
            context,
            constructor,
            &constructor_arguments,
            Some(class.name.clone()),
            context.unit.strict_types,
        )?;
    }
    Ok(receiver)
}

fn native_function_has_implicit_closure_this(function: &php_ir::IrFunction) -> bool {
    function.implicit_closure_this_local().is_some()
}

#[cfg(test)]
fn native_backtrace_frame(
    compiled: &crate::compiled_unit::CompiledUnit,
    function: php_ir::FunctionId,
    called_class: Option<Arc<str>>,
    object: Option<php_runtime::api::ObjectRef>,
    arguments: request_state::NativeTraceArguments,
) -> NativeBacktraceFrame {
    let metadata = NativeFunctionMetadataPtr::from_compiled(compiled, function);
    native_backtrace_frame_from_metadata(metadata, called_class, object, arguments)
}

fn native_backtrace_frame_from_metadata(
    metadata: Option<NativeFunctionMetadataPtr>,
    called_class: Option<Arc<str>>,
    object: Option<php_runtime::api::ObjectRef>,
    arguments: request_state::NativeTraceArguments,
) -> NativeBacktraceFrame {
    let fixed_argument_count = metadata.as_ref().map_or(0, |metadata| {
        metadata
            .params
            .iter()
            .position(|parameter| parameter.variadic)
            .unwrap_or(metadata.params.len())
            .min(arguments.len()) as u32
    });
    let class = metadata.as_ref().and_then(|metadata| {
        metadata
            .trace_class
            .as_ref()
            .map(|class| called_class.unwrap_or_else(|| Arc::clone(class)))
    });
    NativeBacktraceFrame {
        metadata,
        class,
        object,
        arguments,
        fixed_argument_count,
    }
}

fn invoke_native_external_function(
    context: &mut NativeRequestColdState<'_>,
    target: NativeDynamicFunction,
    arguments: &[i64],
    called_class: Option<String>,
    strict: bool,
) -> NativeCallResult {
    invoke_native_external_function_with_metadata(
        context,
        target,
        arguments,
        None,
        called_class,
        strict,
    )
}

fn invoke_native_resolved_external_function(
    context: &mut NativeRequestColdState<'_>,
    target: NativeDynamicFunction,
    arguments: &[i64],
    called_class: Option<String>,
    strict: bool,
) -> NativeCallResult {
    invoke_native_resolved_external_function_with_metadata(
        context,
        target,
        arguments,
        None,
        called_class,
        strict,
    )
}

fn invoke_native_external_function_with_metadata(
    context: &mut NativeRequestColdState<'_>,
    target: NativeDynamicFunction,
    arguments: &[i64],
    metadata: Option<&[php_ir::instruction::IrCallArg]>,
    called_class: Option<String>,
    strict: bool,
) -> NativeCallResult {
    invoke_native_external_function_with_metadata_at_tier(
        context,
        target,
        arguments,
        metadata,
        called_class,
        strict,
        false,
    )
}

fn invoke_native_resolved_external_function_with_metadata(
    context: &mut NativeRequestColdState<'_>,
    target: NativeDynamicFunction,
    arguments: &[i64],
    metadata: Option<&[php_ir::instruction::IrCallArg]>,
    called_class: Option<String>,
    strict: bool,
) -> NativeCallResult {
    invoke_native_external_function_with_metadata_at_tier(
        context,
        target,
        arguments,
        metadata,
        called_class,
        strict,
        true,
    )
}

fn invoke_native_external_function_with_metadata_at_tier(
    context: &mut NativeRequestColdState<'_>,
    target: NativeDynamicFunction,
    arguments: &[i64],
    metadata: Option<&[php_ir::instruction::IrCallArg]>,
    called_class: Option<String>,
    strict: bool,
    baseline_continuation: bool,
) -> NativeCallResult {
    prepare_dynamic_native_entry(context, target.unit, target.function)?;
    let mut transferred_arguments = transfer_native_external_arguments(context, arguments)?;
    let execution_target = NativeExecutionTarget {
        unit: Some(target.unit),
        function: target.function,
        called_class: called_class
            .as_deref()
            .map(Arc::from)
            .or_else(|| context.called_classes.last().cloned()),
        scope_class: context
            .lexical_scope_classes
            .last()
            .map(|scope| Arc::from(scope.as_str())),
    };
    let result = context.run_in_native_execution_target(&execution_target, |context| {
        let result = if baseline_continuation {
            invoke_native_resolved_function_with_metadata_strict(
                context,
                target.function,
                &transferred_arguments,
                metadata,
                strict,
            )
        } else {
            invoke_baseline_bound_function_with_metadata_strict(
                context,
                target.function,
                &transferred_arguments,
                metadata,
                strict,
                false,
            )
        };
        // External callees may mutate a receiver, reference, or array argument
        // and publish literals from their own IrUnit into that authoritative
        // ownership graph. Rehome those newly written constants while the
        // callee unit is still active, before restoring the caller's runtime
        // view. This is the symmetric return half of argument transfer.
        context.stabilize_owned_native_values_for_cross_unit(&mut transferred_arguments)?;
        match result {
            Ok(encoded) => Ok(context.transfer_external_return(encoded, target.unit)?),
            Err(NativeCallControl::Exit(encoded)) => {
                let encoded = context.transfer_external_return(encoded, target.unit)?;
                Err(NativeCallControl::Exit(encoded))
            }
            Err(control) => Err(control),
        }
    });
    let mut release_error = None;
    for argument in transferred_arguments {
        if let Err(error) = context.release(argument) {
            release_error.get_or_insert(error);
        }
    }
    match (result, release_error) {
        (Err(control), _) => Err(control),
        (Ok(_), Some(error)) => Err(error.into()),
        (Ok(value), None) => Ok(value),
    }
}

fn create_native_external_generator_with_metadata(
    context: &mut NativeRequestColdState<'_>,
    target: NativeDynamicFunction,
    arguments: &[i64],
    metadata: Option<&[php_ir::instruction::IrCallArg]>,
    called_class: Option<String>,
    strict: bool,
) -> NativeCallResult {
    prepare_dynamic_native_entry(context, target.unit, target.function)?;
    let transferred = transfer_native_external_arguments(context, arguments)?;
    let execution_target = NativeExecutionTarget {
        unit: Some(target.unit),
        function: target.function,
        called_class: called_class
            .as_deref()
            .map(Arc::from)
            .or_else(|| context.called_classes.last().cloned()),
        scope_class: context
            .lexical_scope_classes
            .last()
            .map(|scope| Arc::from(scope.as_str())),
    };
    let result = context.run_in_native_execution_target(&execution_target, |context| {
        create_baseline_bound_generator_with_metadata_strict(
            context,
            target.function,
            &transferred,
            metadata,
            strict,
        )
    });
    let mut release_error = None;
    for argument in transferred {
        if let Err(error) = context.release(argument) {
            release_error.get_or_insert(error);
        }
    }
    match (result, release_error) {
        (Err(control), _) => Err(control),
        (Ok(_), Some(error)) => Err(error.into()),
        (Ok(generator), None) => Ok(generator),
    }
}

pub(super) fn resume_native_optimizing_exit(
    context: &mut NativeRequestColdState<'_>,
    active_artifact: php_jit::JitFunctionHandle,
    outcome: Result<php_jit::JitI64InvokeOutcome, php_jit::JitInvokeError>,
) -> Result<php_jit::JitI64InvokeOutcome, php_jit::JitInvokeError> {
    resume_native_optimizing_exit_with_artifact(context, Some(active_artifact), outcome)
        .map(|(_, outcome)| outcome)
}

fn native_transition_metadata<'a>(
    handle: &'a php_jit::JitFunctionHandle,
    state: &php_jit::JitDeoptState,
) -> Option<&'a php_jit::JitNativeTransitionMetadata> {
    handle.region_state_metadata().and_then(|metadata| {
        metadata.native_transitions.iter().find(|entry| {
            entry.function.raw() == state.function_id
                && entry.continuation_id == state.continuation_id
        })
    })
}

fn active_artifact_owns_published_transition(
    context: &NativeRequestColdState<'_>,
    handle: &php_jit::JitFunctionHandle,
    state: &php_jit::JitDeoptState,
) -> bool {
    let Some(metadata) = handle.region_state_metadata() else {
        return false;
    };
    if metadata.native_version != state.native_version
        || native_transition_metadata(handle, state).is_none()
    {
        return false;
    }
    let Some(function_entry) = metadata
        .function_entries
        .iter()
        .find(|entry| entry.function.raw() == state.function_id)
    else {
        return false;
    };
    context
        .compiled
        .prepared_deployment_image()
        .preferred_function_entries
        .get(state.function_id as usize)
        .is_some_and(|entry| {
            entry.load(std::sync::atomic::Ordering::Acquire) == function_entry.address
        })
}

fn native_transition_owner_adjustments(
    source: &php_jit::JitNativeTransitionMetadata,
    target: &php_jit::JitNativeTransitionMetadata,
    state: &php_jit::JitDeoptState,
) -> (Vec<i64>, Vec<i64>) {
    let source_locals = source
        .owned_locals
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let target_locals = target
        .owned_locals
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let source_registers = source
        .owned_registers
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let target_registers = target
        .owned_registers
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut retain = Vec::new();
    let mut release = Vec::new();
    for local in source_locals.union(&target_locals).copied() {
        if !state.local_initialized(local) {
            continue;
        }
        let value = state.slots[local.index()];
        match (
            source_locals.contains(&local),
            target_locals.contains(&local),
        ) {
            (false, true) => retain.push(value),
            (true, false) => release.push(value),
            _ => {}
        }
    }
    for snapshot in 0..php_jit::JIT_DEOPT_MAX_REGISTERS {
        let initialized = state.initialized_register_mask
            & 1_u64
                .checked_shl(u32::try_from(snapshot).unwrap_or(u32::MAX))
                .unwrap_or(0)
            != 0;
        if !initialized {
            continue;
        }
        let register = php_ir::RegId::new(state.register_ids[snapshot]);
        let value = state.registers[snapshot];
        match (
            source_registers.contains(&register),
            target_registers.contains(&register),
        ) {
            (false, true) => retain.push(value),
            (true, false) => release.push(value),
            _ => {}
        }
    }
    (retain, release)
}

fn reconcile_native_transition_owners(
    context: &mut NativeRequestColdState<'_>,
    source: &php_jit::JitFunctionHandle,
    target: &php_jit::JitFunctionHandle,
    state: &php_jit::JitDeoptState,
) -> Result<(), String> {
    let source = native_transition_metadata(source, state).ok_or_else(|| {
        format!(
            "optimizing transition {}:{} has no source ownership metadata",
            state.function_id, state.continuation_id
        )
    })?;
    let target = native_transition_metadata(target, state).ok_or_else(|| {
        format!(
            "optimizing transition {}:{} has no baseline ownership metadata",
            state.function_id, state.continuation_id
        )
    })?;
    let (retain, release) = native_transition_owner_adjustments(source, target, state);
    // Acquire the baseline-only owners first. If the same encoded value moves
    // between two ownership identities, it can never transiently reach zero.
    for value in retain {
        context.retain(value)?;
    }
    for value in release {
        context.release(value)?;
    }
    Ok(())
}

fn remap_native_transition_registers(
    target: &php_jit::JitNativeTransitionMetadata,
    state: &php_jit::JitDeoptState,
) -> Result<php_jit::JitDeoptState, String> {
    let mut remapped = *state;
    remapped.initialized_register_mask = 0;
    remapped.register_ids.fill(0);
    remapped.registers.fill(0);
    for (target_slot, register) in target.live_registers.iter().copied().enumerate() {
        let Some(source_slot) = (0..php_jit::JIT_DEOPT_MAX_REGISTERS).find(|source_slot| {
            state.initialized_register_mask & (1_u64 << source_slot) != 0
                && state.register_ids[*source_slot] == register.raw()
        }) else {
            return Err(format!(
                "optimizing transition {}:{} did not publish live baseline register {}",
                state.function_id,
                state.continuation_id,
                register.raw()
            ));
        };
        remapped.register_ids[target_slot] = register.raw();
        remapped.registers[target_slot] = state.registers[source_slot];
        remapped.initialized_register_mask |= 1_u64 << target_slot;
    }
    Ok(remapped)
}

fn resume_native_optimizing_exit_with_artifact(
    context: &mut NativeRequestColdState<'_>,
    mut active_artifact: Option<php_jit::JitFunctionHandle>,
    mut outcome: Result<php_jit::JitI64InvokeOutcome, php_jit::JitInvokeError>,
) -> Result<
    (
        Option<php_jit::JitFunctionHandle>,
        php_jit::JitI64InvokeOutcome,
    ),
    php_jit::JitInvokeError,
> {
    loop {
        let Ok(php_jit::JitI64InvokeOutcome::SideExit { status, state, .. }) = &outcome else {
            return outcome.map(|outcome| (active_artifact, outcome));
        };
        if *status != php_jit::JitCallStatus::RECOMPILE_REQUESTED.0 as i32 {
            return outcome.map(|outcome| (active_artifact, outcome));
        }
        let transition_instruction =
            context.instruction_for_continuation(state.function_id, state.continuation_id);
        let mut transition_reason = transition_instruction
            .as_ref()
            .map(|instruction| native_optimizing_transition_reason(&instruction.kind))
            .unwrap_or_else(|| std::borrow::Cow::Borrowed("optimizer_unknown"));
        if transition_reason.as_ref() == "optimizer_array:IssetDim" {
            let mut detail = match state.control_reserved {
                php_jit::JIT_OPTIMIZING_EXIT_ARRAY_NOT_TAGGED => "not_tagged",
                php_jit::JIT_OPTIMIZING_EXIT_ARRAY_VIEW_MISSING => "view_missing",
                php_jit::JIT_OPTIMIZING_EXIT_ARRAY_KEY_UNSUPPORTED => "key_unsupported",
                _ => "unknown",
            }
            .to_owned();
            if state.control_reserved == php_jit::JIT_OPTIMIZING_EXIT_ARRAY_NOT_TAGGED
                && let Some(instruction) = transition_instruction.as_ref()
                && let php_ir::InstructionKind::IssetDim { local, .. } = &instruction.kind
                && state.local_initialized(*local)
            {
                detail.push(':');
                detail.push_str(native_transition_value_kind(state.slots[local.index()]));
            }
            transition_reason =
                std::borrow::Cow::Owned(format!("{}:{detail}", transition_reason.as_ref()));
        } else if transition_reason.as_ref() == "optimizer_local:LoadLocal"
            && let Some(instruction) = transition_instruction.as_ref()
            && let php_ir::InstructionKind::LoadLocal { local, .. } = &instruction.kind
            && state.local_initialized(*local)
        {
            let stored = native_transition_direct_value_kind(context, state.slots[local.index()]);
            let next = context
                .instruction_for_continuation(
                    state.function_id,
                    state.continuation_id.saturating_add(1),
                )
                .map(|instruction| {
                    let rendered = format!("{:?}", instruction.kind);
                    rendered
                        .split_once([' ', '{', '('])
                        .map_or(rendered.as_str(), |(name, _)| name)
                        .to_owned()
                })
                .unwrap_or_else(|| "terminal".to_owned());
            transition_reason = std::borrow::Cow::Owned(format!(
                "{}:{stored}:next_{next}",
                transition_reason.as_ref()
            ));
        } else if transition_reason.as_ref() == "optimizer_array:AssignDim"
            && let Some(instruction) = transition_instruction.as_ref()
            && let php_ir::InstructionKind::AssignDim { local, .. } = &instruction.kind
            && state.local_initialized(*local)
        {
            let encoded = state.slots[local.index()];
            let raw = native_transition_value_kind(encoded);
            let stored = native_transition_direct_value_kind(context, encoded);
            let descriptor = php_jit::jit_decode_runtime_value(encoded).map_or_else(
                || "immediate".to_owned(),
                |index| {
                    if index >= php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE {
                        return context
                            .direct_value_slots
                            .get((index - php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE) as usize)
                            .map_or_else(
                                || "direct_missing".to_owned(),
                                |slot| format!("direct_kind_{}_refs_{}", slot.kind, slot.refcount),
                            );
                    }
                    "cold_record".to_owned()
                },
            );
            transition_reason = std::borrow::Cow::Owned(format!(
                "{}:{raw}:{stored}:{descriptor}",
                transition_reason.as_ref()
            ));
        } else if transition_reason
            .as_ref()
            .starts_with("optimizer_call:CallFunction:")
            && let Some(instruction) = transition_instruction.as_ref()
            && let php_ir::InstructionKind::CallFunction { args, .. } = &instruction.kind
        {
            let values = args
                .iter()
                .take(4)
                .map(|argument| {
                    let encoded = match argument.value {
                        php_ir::Operand::Local(local) if state.local_initialized(local) => {
                            Some(state.slots[local.index()])
                        }
                        php_ir::Operand::Register(register) => (0
                            ..php_jit::JIT_DEOPT_MAX_REGISTERS)
                            .find(|index| {
                                state.initialized_register_mask & (1_u64 << index) != 0
                                    && state.register_ids[*index] == register.raw()
                            })
                            .map(|index| state.registers[index]),
                        php_ir::Operand::Constant(_) | php_ir::Operand::Local(_) => None,
                    };
                    encoded.map_or_else(
                        || "constant_or_unpublished".to_owned(),
                        |encoded| {
                            format!(
                                "{}/{}",
                                native_transition_value_kind(encoded),
                                native_transition_direct_value_kind(context, encoded),
                            )
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            transition_reason = std::borrow::Cow::Owned(format!(
                "{}:values_{values}:detail_{:#x}",
                transition_reason.as_ref(),
                state.control_reserved,
            ));
        }
        let transition_started = context
            .options
            .collect_counters
            .then(std::time::Instant::now);
        let function = php_ir::FunctionId::new(state.function_id);
        let replays_store = transition_instruction.as_ref().is_some_and(|instruction| {
            matches!(instruction.kind, php_ir::InstructionKind::StoreLocal { .. })
        });
        let fallback = NativeExecutionTarget {
            unit: context.current_dynamic_unit,
            function,
            called_class: context.called_classes.last().cloned(),
            scope_class: context
                .lexical_scope_classes
                .last()
                .map(|scope| Arc::from(scope.as_str())),
        };
        let target = context
            .native_execution_target_from_state(state, Some(&fallback))
            .map_err(|_| php_jit::JitInvokeError::MissingNativeTransition {
                function: state.function_id,
                continuation: state.continuation_id,
            })?;
        let state = *state;
        let carried_artifact = active_artifact.clone();
        let (baseline, resumed) = context
            .run_in_native_execution_target(&target, |context| -> Result<_, String> {
                // A direct linked callee can side-exit without returning
                // through the caller's Rust coordinator. The carried handle
                // may therefore own the caller graph, and dense unit-local
                // FunctionIds can make unrelated metadata appear to match.
                // Match its exact function entry against the runtime-view-
                // selected unit's preferred publication cell before using
                // it. This preserves the generation that actually produced
                // the exit without trusting a coincidental dense ID.
                let source = if let Some(source) = carried_artifact.as_ref().filter(|source| {
                    active_artifact_owns_published_transition(context, source, &state)
                }) {
                    source.clone()
                } else {
                    ensure_native_entry(context, function)?
                };
                let source_metadata =
                    native_transition_metadata(&source, &state).ok_or_else(|| {
                        format!(
                            "optimizing transition {}:{} has no active-unit source metadata",
                            state.function_id, state.continuation_id
                        )
                    })?;
                if source_metadata.native_version != state.native_version {
                    let error = format!(
                        "optimizing transition {}:{} source tier {} does not match state tier {}",
                        state.function_id,
                        state.continuation_id,
                        source_metadata.native_version,
                        state.native_version
                    );
                    cold_diagnostics::record_native_helper_failure(context, error.clone());
                    return Err(error);
                }
                let baseline = ensure_native_baseline_entry(context, function)?;
                if let Err(error) =
                    reconcile_native_transition_owners(context, &source, &baseline, &state)
                {
                    cold_diagnostics::record_native_helper_failure(context, error.clone());
                    return Err(error);
                }
                let Some(target_metadata) = native_transition_metadata(&baseline, &state) else {
                    let error = format!(
                        "optimizing transition {}:{} has no reconciled baseline metadata",
                        state.function_id, state.continuation_id
                    );
                    cold_diagnostics::record_native_helper_failure(context, error.clone());
                    return Err(error);
                };
                let baseline_state =
                    match remap_native_transition_registers(target_metadata, &state) {
                        Ok(state) => state,
                        Err(error) => {
                            cold_diagnostics::record_native_helper_failure(context, error.clone());
                            return Err(error);
                        }
                    };
                let runtime = context.native_runtime_ptr();
                context.baseline_transition_store_owner_pending = replays_store;
                let resumed = baseline.invoke_i64_native_transition_with_unwind_runtime(
                    &baseline_state,
                    php_jit::JIT_RUNTIME_ABI_HASH,
                    runtime,
                    |types, value| native_catch_matches(context, types, value),
                );
                context.baseline_transition_store_owner_pending = false;
                Ok((baseline, resumed))
            })
            .map_err(|_| php_jit::JitInvokeError::MissingNativeTransition {
                function: state.function_id,
                continuation: state.continuation_id,
            })?;
        outcome = resumed;
        active_artifact = Some(baseline);
        if let Some(started) = transition_started {
            context.record_native_transition(transition_reason.as_ref(), started.elapsed(), 0);
        }
    }
}

fn native_transition_value_kind(encoded: i64) -> &'static str {
    let encoded = encoded as u64;
    match encoded & php_jit::JIT_VALUE_RUNTIME_KIND_MASK {
        php_jit::JIT_VALUE_RUNTIME_REFERENCE_TAG => "reference",
        php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG => "array",
        php_jit::JIT_VALUE_RUNTIME_OBJECT_TAG => "object",
        php_jit::JIT_VALUE_RUNTIME_STRING_TAG => "string",
        php_jit::JIT_VALUE_RUNTIME_FLOAT_TAG => "float",
        php_jit::JIT_VALUE_RUNTIME_RESOURCE_TAG => "resource",
        php_jit::JIT_VALUE_RUNTIME_CALLABLE_TAG => "callable",
        php_jit::JIT_VALUE_RUNTIME_GENERATOR_TAG => "generator",
        php_jit::JIT_VALUE_RUNTIME_FIBER_TAG => "fiber",
        php_jit::JIT_VALUE_RUNTIME_ITERATOR_TAG => "iterator",
        _ if encoded == php_jit::jit_encode_constant(u32::MAX) as u64 => "null",
        _ if encoded & php_jit::JIT_VALUE_TAG_MASK == php_jit::JIT_VALUE_CONSTANT_TAG => "constant",
        _ => "immediate",
    }
}

fn native_transition_direct_value_kind(
    context: &NativeRequestColdState<'_>,
    encoded: i64,
) -> &'static str {
    if let Some(index) = NativeRequestColdState::direct_value_index(encoded)
        && let Some(slot) = context
            .direct_value_slots
            .get(index)
            .filter(|slot| slot.refcount != 0)
    {
        return match slot.kind {
            php_jit::JIT_NATIVE_VALUE_VIEW_PREPARED_CALLABLE => "prepared_callable",
            php_jit::JIT_NATIVE_VALUE_VIEW_COLD_GENERATOR => "materialized_generator",
            php_jit::JIT_NATIVE_VALUE_VIEW_FOREACH_DIRECT => "array_iterator",
            php_jit::JIT_NATIVE_VALUE_VIEW_COLD_ITERATOR => {
                context
                    .cold_iterator(index)
                    .map_or("missing", |iterator| match iterator {
                        NativeColdIterator::Array(_) => "array_iterator",
                        NativeColdIterator::Object(_) => "object_iterator",
                        NativeColdIterator::Snapshot(_) => "snapshot_iterator",
                        NativeColdIterator::LiveArray(_) => "live_array_iterator",
                        NativeColdIterator::User(_) => "iterator",
                        NativeColdIterator::Generator(_) => "baseline_generator_iterator",
                    })
            }
            _ => native_transition_value_kind(encoded),
        };
    }
    let Some(_) = php_jit::jit_decode_runtime_value(encoded) else {
        return native_transition_value_kind(encoded);
    };
    "missing"
}

fn native_optimizing_transition_reason(
    kind: &php_ir::InstructionKind,
) -> std::borrow::Cow<'static, str> {
    use php_ir::InstructionKind;

    let family = match kind {
        InstructionKind::LoadLocal { .. }
        | InstructionKind::StoreLocal { .. }
        | InstructionKind::Discard { .. }
        | InstructionKind::IssetLocal { .. }
        | InstructionKind::EmptyLocal { .. }
        | InstructionKind::UnsetLocal { .. } => "optimizer_local",
        InstructionKind::Unary { .. }
        | InstructionKind::Binary { .. }
        | InstructionKind::Compare { .. }
        | InstructionKind::Cast { .. } => "optimizer_scalar",
        InstructionKind::NewArray { .. }
        | InstructionKind::ArrayInsert { .. }
        | InstructionKind::ArraySpread { .. }
        | InstructionKind::FetchDim { .. }
        | InstructionKind::AssignDim { .. }
        | InstructionKind::AppendDim { .. }
        | InstructionKind::UnsetDim { .. }
        | InstructionKind::IssetDim { .. }
        | InstructionKind::EmptyDim { .. } => "optimizer_array",
        InstructionKind::ForeachInit { .. }
        | InstructionKind::ForeachInitRef { .. }
        | InstructionKind::ForeachNext { .. }
        | InstructionKind::ForeachNextRef { .. }
        | InstructionKind::ForeachCleanup { .. } => "optimizer_foreach",
        InstructionKind::FetchProperty { .. }
        | InstructionKind::AssignProperty { .. }
        | InstructionKind::FetchDynamicStaticProperty { .. }
        | InstructionKind::AssignDynamicStaticProperty { .. }
        | InstructionKind::FetchObjectClassName { .. } => "optimizer_property",
        InstructionKind::BindReference { .. }
        | InstructionKind::BindReferenceDim { .. }
        | InstructionKind::BindReferenceProperty { .. }
        | InstructionKind::BindReferenceFromProperty { .. }
        | InstructionKind::BindReferenceFromPropertyDim { .. }
        | InstructionKind::BindReferencePropertyDim { .. }
        | InstructionKind::BindReferenceDimFromProperty { .. }
        | InstructionKind::BindReferenceFromDim { .. }
        | InstructionKind::BindReferenceFromStaticPropertyDim { .. }
        | InstructionKind::BindReferenceStaticProperty { .. }
        | InstructionKind::BindReferenceFromCall { .. }
        | InstructionKind::BindReferenceFromMethodCall { .. } => "optimizer_reference",
        InstructionKind::CallFunction { .. }
        | InstructionKind::CallMethod { .. }
        | InstructionKind::CallStaticMethod { .. }
        | InstructionKind::CallClosure { .. }
        | InstructionKind::CallCallable { .. }
        | InstructionKind::Pipe { .. }
        | InstructionKind::NewObject { .. }
        | InstructionKind::DynamicNewObject { .. } => "optimizer_call",
        InstructionKind::Include { .. }
        | InstructionKind::Eval { .. }
        | InstructionKind::DeclareFunction { .. }
        | InstructionKind::DeclareClass { .. } => "optimizer_dynamic_code",
        _ => "optimizer_other",
    };
    // This runs only while diagnostic counters are enabled. Preserve the
    // exact IR opcode, but not its operands, so an aggregate family cannot
    // hide the next dominant warm transition after an earlier exit is
    // removed.
    if let InstructionKind::Binary { op, .. } = kind {
        return format!("{family}:Binary:{op:?}").into();
    }
    if let InstructionKind::CallFunction { name, args, .. } = kind {
        let named = args
            .iter()
            .filter(|argument| argument.name.is_some())
            .count();
        let unpacked = args.iter().filter(|argument| argument.unpack).count();
        return format!(
            "{family}:CallFunction:{}:argc{}:named{}:unpack{}",
            name.trim_start_matches('\\').to_ascii_lowercase(),
            args.len(),
            named,
            unpacked,
        )
        .into();
    }
    let debug = format!("{kind:?}");
    let end = debug.find([' ', '{', '(']).unwrap_or(debug.len());
    format!("{family}:{}", &debug[..end]).into()
}

fn native_class_is_a(context: &NativeRequestColdState<'_>, class_name: &str, target: &str) -> bool {
    let target = normalize_class_name(target);
    let class_name = normalize_class_name(class_name);
    if class_name == "arrayiterator" && matches!(target.as_str(), "iterator" | "traversable") {
        return true;
    }
    let mut pending = vec![class_name];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(candidate) = pending.pop() {
        if candidate == target {
            return true;
        }
        if !visited.insert(candidate.clone()) {
            continue;
        }
        if let Some(class) = context
            .unit
            .classes
            .iter()
            .find(|class| class.name == candidate)
        {
            if let Some(parent) = &class.parent {
                pending.push(normalize_class_name(parent));
            }
            pending.extend(
                class
                    .interfaces
                    .iter()
                    .map(|interface| normalize_class_name(interface)),
            );
        } else if let Some((_, class)) = native_external_class_ref(context, &candidate) {
            if let Some(parent) = &class.parent {
                pending.push(normalize_class_name(parent));
            }
            pending.extend(
                class
                    .interfaces
                    .iter()
                    .map(|interface| normalize_class_name(interface)),
            );
        } else if let Some(class) =
            php_std::ExtensionRegistry::standard_library().enabled_class(&candidate)
            && let Some(metadata) = class.source_metadata()
        {
            if let Some(parent) = metadata.parent {
                pending.push(normalize_class_name(parent));
            }
            pending.extend(
                metadata
                    .interfaces
                    .iter()
                    .map(|interface| normalize_class_name(interface)),
            );
        }
    }
    false
}

fn native_method_in_hierarchy(
    context: &NativeRequestColdState<'_>,
    class_name: &str,
    method: &str,
) -> Option<php_ir::FunctionId> {
    let mut candidate = normalize_class_name(class_name);
    loop {
        let class = context
            .unit
            .classes
            .iter()
            .find(|class| class.name == candidate)?;
        if let Some(entry) = class
            .methods
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(method))
        {
            return Some(entry.function);
        }
        candidate = normalize_class_name(class.parent.as_ref()?);
    }
}

fn native_function_is_generator(
    context: &NativeRequestColdState<'_>,
    function: php_ir::FunctionId,
) -> bool {
    context
        .unit
        .functions
        .get(function.index())
        .is_some_and(|function| {
            function.flags.is_generator
                || function
                    .blocks
                    .iter()
                    .flat_map(|block| &block.instructions)
                    .any(|instruction| {
                        matches!(
                            instruction.kind,
                            php_ir::InstructionKind::Yield { .. }
                                | php_ir::InstructionKind::YieldFrom { .. }
                        )
                    })
        })
}

fn native_function_requires_non_reference_trampoline(
    function: &php_ir::IrFunction,
    method_scope_sensitive: bool,
) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            matches!(
                instruction.kind,
                php_ir::InstructionKind::Yield { .. } | php_ir::InstructionKind::YieldFrom { .. }
            ) || matches!(
                &instruction.kind,
                php_ir::InstructionKind::CallFunction { name, .. }
                    if name.trim_start_matches('\\').eq_ignore_ascii_case("debug_backtrace")
            ) || method_scope_sensitive
                && matches!(
                    &instruction.kind,
                    php_ir::InstructionKind::FetchClassConstant {
                        class_name,
                        ..
                    } | php_ir::InstructionKind::CallStaticMethod {
                        class_name,
                        ..
                    } if class_name.eq_ignore_ascii_case("static")
                )
        })
    }) || function.attributes.iter().any(|attribute| {
        attribute
            .resolved_name
            .as_deref()
            .or(attribute.fallback_name.as_deref())
            .unwrap_or(&attribute.name)
            .trim_start_matches('\\')
            .eq_ignore_ascii_case("deprecated")
    })
}

fn native_function_exception_routes(
    function: php_ir::FunctionId,
    definition: &php_ir::IrFunction,
) -> Option<php_ir::FunctionId> {
    definition
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|instruction| {
            matches!(
                instruction.kind,
                php_ir::InstructionKind::EnterTry { catch: Some(_), .. }
                    | php_ir::InstructionKind::EnterTry {
                        finally: Some(_),
                        ..
                    }
            )
        })
        .then_some(function)
}

fn native_calling_class<'a>(
    context: &'a NativeRequestColdState<'_>,
    function: u32,
) -> Option<&'a php_ir::ClassEntry> {
    context.unit.classes.iter().find(|class| {
        class
            .methods
            .iter()
            .any(|method| method.function.raw() == function)
    })
}

fn native_effective_calling_class<'a>(
    context: &'a NativeRequestColdState<'_>,
    function: u32,
) -> Option<&'a php_ir::ClassEntry> {
    native_calling_class(context, function).or_else(|| {
        let scope = context.lexical_scope_classes.last()?;
        let normalized = normalize_class_name(scope);
        context
            .unit
            .classes
            .iter()
            .find(|class| class.name == normalized)
    })
}

fn native_resolve_scoped_class_name(
    context: &NativeRequestColdState<'_>,
    class_name: &str,
    caller_function: u32,
) -> Result<String, String> {
    match class_name.to_ascii_lowercase().as_str() {
        "self" => native_effective_calling_class(context, caller_function)
            .map(|class| class.display_name.clone())
            .ok_or_else(|| "Cannot use \"self\" in the global scope".to_owned()),
        "static" => context
            .called_classes
            .last()
            .map(|class| class.to_string())
            .or_else(|| {
                native_effective_calling_class(context, caller_function)
                    .map(|class| class.display_name.clone())
            })
            .ok_or_else(|| "Cannot use \"static\" in the global scope".to_owned()),
        "parent" => native_effective_calling_class(context, caller_function)
            .and_then(|class| {
                class
                    .parent_display_name
                    .clone()
                    .or_else(|| class.parent.clone())
            })
            .ok_or_else(|| "Cannot use \"parent\" when no parent scope is active".to_owned()),
        _ => Ok(class_name.to_owned()),
    }
}

fn native_method_access_error(
    context: &NativeRequestColdState<'_>,
    function: php_ir::FunctionId,
    caller_function: u32,
    _late_static_call: bool,
) -> Option<String> {
    let (declaring_class, method) = context.unit.classes.iter().find_map(|class| {
        class
            .methods
            .iter()
            .find(|method| method.function == function)
            .map(|method| (class, method))
    })?;
    if !method.flags.is_private && !method.flags.is_protected {
        return None;
    }
    let caller = native_effective_calling_class(context, caller_function);
    if method.flags.is_private && caller.is_none_or(|caller| caller.name != declaring_class.name) {
        if caller.is_none() {
            return Some(format!(
                "Call to private method {}::{}() from global scope",
                declaring_class.display_name, method.name
            ));
        }
        return Some(format!(
            "Cannot access private method {}::{}()",
            declaring_class.display_name, method.name
        ));
    }
    if method.flags.is_protected
        && caller
            .is_none_or(|caller| !native_class_is_a(context, &caller.name, &declaring_class.name))
    {
        return Some(format!(
            "Cannot access protected method {}::{}()",
            declaring_class.display_name, method.name
        ));
    }
    None
}

fn native_external_method_access_error(
    context: &NativeRequestColdState<'_>,
    target: NativeDynamicFunction,
    caller_function: u32,
    _late_static_call: bool,
) -> Option<String> {
    let unit = context.dynamic_units.get(target.unit)?.compiled.unit();
    let (declaring_class, method) = unit.classes.iter().find_map(|class| {
        class
            .methods
            .iter()
            .find(|method| method.function == target.function)
            .map(|method| (class, method))
    })?;
    if !method.flags.is_private && !method.flags.is_protected {
        return None;
    }
    let caller = native_effective_calling_class(context, caller_function);
    if method.flags.is_private && caller.is_none_or(|caller| caller.name != declaring_class.name) {
        if caller.is_none() {
            return Some(format!(
                "Call to private method {}::{}() from global scope",
                declaring_class.display_name, method.name
            ));
        }
        return Some(format!(
            "Cannot access private method {}::{}()",
            declaring_class.display_name, method.name
        ));
    }
    if method.flags.is_protected
        && caller
            .is_none_or(|caller| !native_class_is_a(context, &caller.name, &declaring_class.name))
    {
        return Some(format!(
            "Cannot access protected method {}::{}()",
            declaring_class.display_name, method.name
        ));
    }
    None
}

#[cfg(test)]
mod tests;
