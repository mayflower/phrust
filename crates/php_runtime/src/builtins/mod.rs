//! Deterministic internal builtin registry for the runtime VM.

mod context;
mod error;
mod generated;
pub(in crate::builtins) mod modules;
mod registry;
mod request_state;
mod signatures;

pub use crate::source_span::RuntimeSourceSpan;
pub use context::{
    ApcuState, BuiltinContext, CurlState, FilesystemRuntimeState, FtpOptionValue, FtpState,
    GettextState, IconvEncodingState, ImapConnectionConfig, ImapMailboxSnapshot, ImapState,
    JSON_ERROR_RECURSION, JSON_PARTIAL_OUTPUT_ON_ERROR, JSON_THROW_ON_ERROR, LdapSearchScope,
    LdapState, MbSubstituteCharacter, OpcacheState, OpenSslErrorState, PcntlState, ReadlineState,
    SYSVMSG_EAGAIN, SYSVMSG_EINVAL, SYSVMSG_IPC_NOWAIT, ShmopState, SoapState, SocketState,
    Ssh2FingerprintHash, Ssh2State, StreamContextState, StrtokState, SysvMessageQueueState,
    SysvSemaphoreError, SysvSemaphoreState, SysvSharedMemoryState,
};
pub(in crate::builtins) use context::{
    CurlEasyCollector, CurlMultiDone, CurlMultiRuntimeState, CurlMultiTransferState,
};
pub use error::{BuiltinError, BuiltinErrorContext};
#[doc(hidden)]
pub use modules::core::{NativePrintfScalar, format_native_printf_scalars};
#[doc(hidden)]
pub use modules::curl::{CurlNetworkTestOverride, set_curl_network_tests_override_for_tests};
pub use modules::fileinfo::validate_fileinfo_options;
#[doc(hidden)]
pub use modules::filesystem::{
    native_basename, native_dirname, native_file_exists, native_file_get_contents,
    native_filemtime, native_filesize, native_is_dir, native_is_file, native_is_readable,
    native_is_writable, native_realpath,
};
#[doc(hidden)]
pub use modules::igbinary::{
    serialize_value as igbinary_serialize_value, unserialize_value as igbinary_unserialize_value,
};
pub use modules::intl::{
    NORMALIZER_FORM_C, NORMALIZER_FORM_D, NORMALIZER_FORM_KC, NORMALIZER_FORM_KD,
    is_normalized_string, normalize_string,
};
#[doc(hidden)]
pub use modules::json::{
    NativeDecodedArrayKey, NativeDecodedValue, decode_native_json_associative, exact_json_decode,
    exact_json_encode, exact_json_last_error, exact_json_last_error_msg, exact_json_validate,
    validate_native_json,
};
pub use modules::json_fast::append_json_default_string;
#[doc(hidden)]
pub use modules::math::{
    NativeParsedBaseNumber, native_base_convert, native_decimal_to_base, native_parse_base_digits,
    native_round_f64,
};
#[doc(hidden)]
pub use modules::msgpack::{
    pack_value as msgpack_pack_value, unpack_value as msgpack_unpack_value,
};
#[doc(hidden)]
pub use modules::pcre::{
    NativePregMatchAllResult, NativePregMatchResult, NativePregReplaceManyResult,
    NativePregReplaceResult, exact_preg_filter, exact_preg_grep, exact_preg_last_error,
    exact_preg_last_error_msg, exact_preg_match, exact_preg_match_all, exact_preg_quote,
    exact_preg_replace, exact_preg_split, native_preg_grep, native_preg_match,
    native_preg_match_all, native_preg_replace_many, native_preg_replace_scalar, native_preg_split,
};
#[doc(hidden)]
pub use modules::soap::{
    SoapParsedBody, build_soap_envelope, load_wsdl, parse_soap_response, parse_wsdl, soap_http_post,
};
#[doc(hidden)]
pub use modules::sockets::{native_inet_ntop, native_inet_pton, native_ip2long, native_long2ip};
#[doc(hidden)]
pub use modules::strings::{
    NATIVE_HTML_ESCAPE_DEFAULT_FLAGS, NATIVE_PHP_QUERY_RFC3986, exact_printf, exact_sprintf,
    exact_vprintf, exact_vsprintf, native_addcslashes, native_base64_decode, native_base64_encode,
    native_bin2hex, native_convert_uudecode, native_convert_uuencode, native_crc32, native_hash,
    native_hash_hmac, native_hex2bin, native_html_entity_decode, native_htmlentities,
    native_htmlspecialchars, native_htmlspecialchars_decode, native_http_build_query_component,
    native_http_build_query_scalar, native_md5, native_natural_compare, native_parse_str,
    native_parse_url, native_quoted_printable_decode, native_quotemeta, native_rawurldecode,
    native_rawurlencode, native_sha1, native_str_pad, native_string_search_slice,
    native_strip_tags, native_stripcslashes, native_stripslashes, native_strpbrk, native_strrchr,
    native_strtr, native_substr_compare, native_substr_replace, native_ucwords, native_urldecode,
    native_urlencode,
};
#[doc(hidden)]
pub use modules::zlib::{
    ZLIB_ENCODING_DEFLATE, ZLIB_ENCODING_GZIP, ZLIB_ENCODING_RAW, native_zlib_decode,
    native_zlib_decode_auto, native_zlib_encode,
};
pub use modules::{array_intrinsics, json_fast, string_intrinsics};
pub use registry::{
    BuiltinCompatibility, BuiltinEntry, BuiltinExecutionKind, BuiltinHandlerKind, BuiltinRegistry,
};
pub use request_state::{BuiltinRequestState, JsonRequestState, PcreRequestState};
pub use signatures::{BuiltinOutcome, BuiltinResult, InternalFunction};

pub fn hash_algorithm_exists(algorithm: &str) -> bool {
    modules::hash::hash_algorithm_exists(algorithm)
}
