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
pub(in crate::vm) use cold_publication::resume_native_optimizing_exit;
use cold_publication::*;
mod cold_request_state;
mod diagnostic_helpers;
mod diagnostic_telemetry;
mod frame_arena;
mod request_state;

use crate::native_exact::{
    NATIVE_TEMPNAM_SEQUENCE, NativeBcmathCapability, NativeConfigurationCapability,
    NativeExecutionDeadlineCapability, NativeFilterCapability, NativeFixedCallablePlan,
    NativeFrameArenaCapability, NativeHttpResponseCapability, NativeMbstringCapability,
    NativeRandomCapability, NativeRegisteredAutoloadCallback, NativeRegisteredCallbackSource,
    NativeRegisteredCallbackState, NativeRegisteredErrorHandler, NativeRegisteredShutdownCallback,
    NativeRequestFastState, NativeRequestQueryCapability, NativeSessionCapability,
    NativeStreamContextState, NativeSymbolQueryCapability, native_direct_string_hash,
    native_fixed_callable_plan, php_constant_category, php_core_runtime_constant,
};
use cold_dynamic_units::*;
pub(super) use cold_dynamic_units::{jit_native_function_resolve_abi, native_entries_from_records};
pub(in crate::vm) use cold_request_state::NativeRequestColdState;
pub(in crate::vm) use cold_request_state::NativeRequestOwner;
pub(crate) use frame_arena::NativeFrameArena;
pub(super) use frame_arena::{jit_native_frame_alloc_abi, jit_native_frame_release_abi};

pub(super) use crate::native_exact::exact_call_dispatch::{
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
pub(super) use crate::native_exact::exact_runtime_ops::{
    jit_native_acos_f64_abi, jit_native_acosh_f64_abi, jit_native_acquire_callable_abi,
    jit_native_array_cast_abi, jit_native_array_union_abi, jit_native_asin_f64_abi,
    jit_native_asinh_f64_abi, jit_native_atan_f64_abi, jit_native_atan2_f64_abi,
    jit_native_atanh_f64_abi, jit_native_bit_and_abi, jit_native_bit_not_abi,
    jit_native_bit_or_abi, jit_native_bit_xor_abi, jit_native_callback_return_string_abi,
    jit_native_concat_abi, jit_native_cos_f64_abi, jit_native_cosh_f64_abi, jit_native_count_abi,
    jit_native_deg2rad_f64_abi, jit_native_dynamic_property_slot_abi,
    jit_native_dynamic_property_test_slot_abi, jit_native_echo_bytes_abi, jit_native_equal_abi,
    jit_native_exp_f64_abi, jit_native_expm1_f64_abi, jit_native_float_cast_abi,
    jit_native_float_to_string_abi, jit_native_fmod_f64_abi, jit_native_fpow_f64_abi,
    jit_native_greater_abi, jit_native_greater_equal_abi, jit_native_hypot_f64_abi,
    jit_native_identical_abi, jit_native_int_cast_abi, jit_native_less_abi,
    jit_native_less_equal_abi, jit_native_log_f64_abi, jit_native_log1p_f64_abi,
    jit_native_log10_f64_abi, jit_native_not_equal_abi, jit_native_not_identical_abi,
    jit_native_numeric_string_abi, jit_native_object_cast_abi, jit_native_object_class_name_abi,
    jit_native_plain_object_clone_abi, jit_native_prepared_closure_new_abi,
    jit_native_prepared_exception_new_abi, jit_native_prepared_object_new_abi,
    jit_native_rad2deg_f64_abi, jit_native_resolve_callable_abi, jit_native_round_f64_abi,
    jit_native_sin_f64_abi, jit_native_sinh_f64_abi, jit_native_sizeof_abi,
    jit_native_spaceship_abi, jit_native_string_cast_abi, jit_native_tan_f64_abi,
    jit_native_tanh_f64_abi, jit_native_unary_minus_abi, jit_native_unary_plus_abi,
};
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
    execute_baseline_prepared_runtime_builtin, native_source_line, native_source_line_for_span,
    native_string,
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
    NativeRegisteredCallbackTransfer, baseline_shared_array_storage_is_empty,
    release_baseline_shared_array_storage,
};
use baseline_value_semantics::*;
pub(crate) use cold_diagnostics::*;
pub(in crate::vm) use diagnostic_helpers::*;
use diagnostic_telemetry::NativeRuntimeTelemetry;
use request_state::{NativeBacktraceFrame, NativeRegisteredExtensionRequestState};
pub(crate) use request_state::{NativeFunctionNameScope, NativeLastError};

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
pub(crate) struct NativeDynamicFunction {
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

pub(crate) struct NativeDynamicUnit {
    pub(crate) compiled: crate::compiled_unit::CompiledUnit,
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
pub(crate) enum NativeComparisonValue<'a> {
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
pub(crate) struct NativeComparisonObject<'a> {
    pub(crate) identity: u64,
    pub(crate) layout_id: Option<u64>,
    pub(crate) owner: &'a php_runtime::api::ObjectRef,
}

#[derive(Default)]
pub(crate) struct NativeComparisonTraversal {
    pub(crate) arrays: Vec<(usize, usize)>,
    pub(crate) objects: Vec<(u64, u64)>,
    pub(crate) unordered: bool,
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

pub(crate) fn native_comparison_truthy(value: NativeComparisonValue<'_>) -> bool {
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

pub(crate) fn native_comparison_values_order(
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

pub(crate) const fn native_reference_state(state: u32) -> u32 {
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

pub(crate) struct NativePreparedClosure {
    /// Stable generated-code view. The capture allocation is boxed before
    /// this record is published, so both pointers remain request-stable.
    pub(crate) native_view: php_jit::JitNativePreparedClosureView,
    /// PHP closure metadata only. `captures` and `bound_this` are always
    /// empty here; their authoritative owners are the encoded fields below.
    pub(crate) closure: php_runtime::api::ClosurePayload,
    pub(crate) capture_descriptors: Arc<[(String, bool)]>,
    pub(crate) implicit_this: Option<i64>,
    pub(crate) captures: Box<[i64]>,
    /// Published only by the exact same-unit closure allocation boundary.
    /// Baseline materialization and rebinding deliberately leave this absent.
    pub(crate) fixed_visible_arity: Option<u32>,
    pub(crate) first_parameter_by_reference: bool,
    pub(crate) returns_int: bool,
    pub(crate) returns_string: bool,
    pub(crate) returns_releasable_scalar: bool,
}

impl NativePreparedClosure {
    pub(crate) fn new(
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
pub(crate) struct NativePreparedCallableOwner {
    pub(crate) native_view: php_jit::JitNativePreparedCallableView,
    /// Closure debug/context metadata is consulted only after an explicit
    /// baseline/cold boundary. Captures and the bound receiver remain
    /// authoritative in `native_view`.
    pub(crate) cold_closure: Option<NativePreparedClosure>,
    /// Stable byte owners addressed by `native_view`. These buffers carry no
    /// independent kind or dispatch semantics.
    pub(crate) _name_bytes: Box<[u8]>,
    pub(crate) _method_bytes: Box<[u8]>,
    pub(crate) _class_bytes: Box<[u8]>,
}

impl NativePreparedCallableOwner {
    pub(crate) fn from_native_parts(
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

    pub(crate) fn install_fixed_plan(&mut self, plan: NativeFixedCallablePlan) {
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

    pub(crate) fn user_function(
        name: Box<[u8]>,
        resolved_function: Option<NativeFixedCallablePlan>,
    ) -> Self {
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

    pub(crate) fn closure(closure: NativePreparedClosure) -> Self {
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

    pub(crate) fn bound_object(
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

    pub(crate) fn bound_class(
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
pub(crate) struct NativeExecutionScope {
    pub(crate) unit: Option<usize>,
    pub(crate) called_class: Option<Arc<str>>,
    pub(crate) scope_class: Option<Arc<str>>,
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

pub(crate) struct PreparedNativeRuntimeClass {
    pub(crate) entry: php_runtime::api::ClassEntry,
    pub(crate) display_name: String,
    pub(crate) layout_id: u64,
    /// One request-owned native owner per initialized default. Each object
    /// instance retains these encoded values into its cloned slot vector.
    pub(crate) default_native_slots: Box<[php_runtime::api::NativeDeclaredPropertySlot]>,
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

#[cfg(test)]
mod tests;
