use super::*;

pub(super) fn native_call_target_metadata(target: &RegionCallTarget) -> (u32, u32, u64, u64) {
    match target {
        RegionCallTarget::Function { name, function } => (
            crate::JitNativeCallKind::FUNCTION.0,
            function.map_or(u32::MAX, FunctionId::raw),
            stable_call_symbol_hash(name),
            0,
        ),
        RegionCallTarget::Method { method, .. } => (
            crate::JitNativeCallKind::METHOD.0,
            u32::MAX,
            stable_call_symbol_hash(method),
            0,
        ),
        RegionCallTarget::StaticMethod { class_name, method } => (
            crate::JitNativeCallKind::STATIC_METHOD.0,
            u32::MAX,
            stable_call_symbol_hash(method),
            stable_call_symbol_hash(class_name),
        ),
        RegionCallTarget::Closure { .. } => (crate::JitNativeCallKind::CLOSURE.0, u32::MAX, 0, 0),
        RegionCallTarget::Callable { .. } => (crate::JitNativeCallKind::CALLABLE.0, u32::MAX, 0, 0),
        RegionCallTarget::Pipe { .. } => (crate::JitNativeCallKind::PIPE.0, u32::MAX, 0, 0),
        RegionCallTarget::Constructor { class_name, .. } => (
            crate::JitNativeCallKind::CONSTRUCTOR.0,
            u32::MAX,
            0,
            stable_call_symbol_hash(class_name),
        ),
        RegionCallTarget::DynamicConstructor { .. } => (
            crate::JitNativeCallKind::DYNAMIC_CONSTRUCTOR.0,
            u32::MAX,
            0,
            0,
        ),
        RegionCallTarget::Semantic { operation } => (
            crate::JitNativeCallKind::SEMANTIC_OPERATION.0,
            operation.operation_id().raw(),
            0,
            0,
        ),
    }
}

pub(super) fn stable_call_symbol_hash(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte.to_ascii_lowercase())).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub(super) fn stable_builtin_helper_id(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\').to_ascii_lowercase();
    if normalized.contains('\\') {
        return None;
    }
    php_runtime::api::BuiltinRegistry::new()
        .get(&normalized)
        .map(php_runtime::api::BuiltinEntry::helper_id)
        .filter(|helper_id| *helper_id != 0)
}

pub(super) fn stable_builtin_dense_id(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\').to_ascii_lowercase();
    if normalized.contains('\\') {
        return None;
    }
    php_runtime::api::BuiltinRegistry::new()
        .get(&normalized)
        .map(php_runtime::api::BuiltinEntry::dense_id)
}

pub(super) fn stable_builtin_type_predicate(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\').to_ascii_lowercase();
    if normalized.contains('\\') {
        return None;
    }
    match normalized.as_str() {
        "is_null" => Some(0),
        "is_bool" => Some(1),
        "is_int" | "is_integer" | "is_long" => Some(2),
        "is_float" | "is_double" | "is_real" => Some(3),
        "is_string" => Some(4),
        "is_array" => Some(5),
        "is_object" => Some(6),
        "is_resource" => Some(7),
        "is_scalar" => Some(8),
        "is_countable" => Some(9),
        "is_iterable" => Some(10),
        _ => None,
    }
}

pub(super) fn stable_builtin_is_numeric(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("is_numeric")
}

pub(super) fn stable_builtin_error_reporting(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("error_reporting")
}

/// Scalar math primitives whose ordinary int/float forms are emitted over
/// native numeric slots. Each discriminant is compile-time lowering metadata,
/// never a runtime operation ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableScalarMathBuiltin {
    Abs,
    Ceil,
    Floor,
    Sqrt,
    Fdiv,
    Fmod,
    IsFinite,
    IsInfinite,
    IsNan,
    Pi,
}

impl StableScalarMathBuiltin {
    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Fdiv | Self::Fmod => arity == 2,
            Self::Pi => arity == 0,
            Self::Abs
            | Self::Ceil
            | Self::Floor
            | Self::Sqrt
            | Self::IsFinite
            | Self::IsInfinite
            | Self::IsNan => arity == 1,
        }
    }
}

pub(super) fn stable_builtin_scalar_math(
    target: &RegionCallTarget,
) -> Option<StableScalarMathBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "abs" => Some(StableScalarMathBuiltin::Abs),
        "ceil" => Some(StableScalarMathBuiltin::Ceil),
        "floor" => Some(StableScalarMathBuiltin::Floor),
        "sqrt" => Some(StableScalarMathBuiltin::Sqrt),
        "fdiv" => Some(StableScalarMathBuiltin::Fdiv),
        "fmod" => Some(StableScalarMathBuiltin::Fmod),
        "is_finite" => Some(StableScalarMathBuiltin::IsFinite),
        "is_infinite" => Some(StableScalarMathBuiltin::IsInfinite),
        "is_nan" => Some(StableScalarMathBuiltin::IsNan),
        "pi" => Some(StableScalarMathBuiltin::Pi),
        _ => None,
    }
}

/// Stateless transcendental math builtins whose ordinary numeric forms call
/// one exact, compile-time-selected pure symbol. The index is publication
/// metadata only: generated code never passes it to a runtime dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StablePureMathBuiltin {
    Acos,
    Acosh,
    Asin,
    Asinh,
    Atan,
    Atan2,
    Atanh,
    Cos,
    Cosh,
    Deg2Rad,
    Exp,
    Expm1,
    Fpow,
    Hypot,
    Log,
    Log10,
    Log1p,
    Rad2Deg,
    Sin,
    Sinh,
    Tan,
    Tanh,
}

impl StablePureMathBuiltin {
    pub(super) const COUNT: usize = 22;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Acos => 0,
            Self::Acosh => 1,
            Self::Asin => 2,
            Self::Asinh => 3,
            Self::Atan => 4,
            Self::Atan2 => 5,
            Self::Atanh => 6,
            Self::Cos => 7,
            Self::Cosh => 8,
            Self::Deg2Rad => 9,
            Self::Exp => 10,
            Self::Expm1 => 11,
            Self::Fpow => 12,
            Self::Hypot => 13,
            Self::Log => 14,
            Self::Log10 => 15,
            Self::Log1p => 16,
            Self::Rad2Deg => 17,
            Self::Sin => 18,
            Self::Sinh => 19,
            Self::Tan => 20,
            Self::Tanh => 21,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Acos => "phrust_native_acos_f64",
            Self::Acosh => "phrust_native_acosh_f64",
            Self::Asin => "phrust_native_asin_f64",
            Self::Asinh => "phrust_native_asinh_f64",
            Self::Atan => "phrust_native_atan_f64",
            Self::Atan2 => "phrust_native_atan2_f64",
            Self::Atanh => "phrust_native_atanh_f64",
            Self::Cos => "phrust_native_cos_f64",
            Self::Cosh => "phrust_native_cosh_f64",
            Self::Deg2Rad => "phrust_native_deg2rad_f64",
            Self::Exp => "phrust_native_exp_f64",
            Self::Expm1 => "phrust_native_expm1_f64",
            Self::Fpow => "phrust_native_fpow_f64",
            Self::Hypot => "phrust_native_hypot_f64",
            Self::Log => "phrust_native_log_f64",
            Self::Log10 => "phrust_native_log10_f64",
            Self::Log1p => "phrust_native_log1p_f64",
            Self::Rad2Deg => "phrust_native_rad2deg_f64",
            Self::Sin => "phrust_native_sin_f64",
            Self::Sinh => "phrust_native_sinh_f64",
            Self::Tan => "phrust_native_tan_f64",
            Self::Tanh => "phrust_native_tanh_f64",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Atan2 | Self::Fpow | Self::Hypot => arity == 2,
            Self::Acos
            | Self::Acosh
            | Self::Asin
            | Self::Asinh
            | Self::Atan
            | Self::Atanh
            | Self::Cos
            | Self::Cosh
            | Self::Deg2Rad
            | Self::Exp
            | Self::Expm1
            | Self::Log
            | Self::Log10
            | Self::Log1p
            | Self::Rad2Deg
            | Self::Sin
            | Self::Sinh
            | Self::Tan
            | Self::Tanh => arity == 1,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Acos,
            Self::Acosh,
            Self::Asin,
            Self::Asinh,
            Self::Atan,
            Self::Atan2,
            Self::Atanh,
            Self::Cos,
            Self::Cosh,
            Self::Deg2Rad,
            Self::Exp,
            Self::Expm1,
            Self::Fpow,
            Self::Hypot,
            Self::Log,
            Self::Log10,
            Self::Log1p,
            Self::Rad2Deg,
            Self::Sin,
            Self::Sinh,
            Self::Tan,
            Self::Tanh,
        ]
    }
}

pub(super) fn stable_builtin_pure_math(target: &RegionCallTarget) -> Option<StablePureMathBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "acos" => Some(StablePureMathBuiltin::Acos),
        "acosh" => Some(StablePureMathBuiltin::Acosh),
        "asin" => Some(StablePureMathBuiltin::Asin),
        "asinh" => Some(StablePureMathBuiltin::Asinh),
        "atan" => Some(StablePureMathBuiltin::Atan),
        "atan2" => Some(StablePureMathBuiltin::Atan2),
        "atanh" => Some(StablePureMathBuiltin::Atanh),
        "cos" => Some(StablePureMathBuiltin::Cos),
        "cosh" => Some(StablePureMathBuiltin::Cosh),
        "deg2rad" => Some(StablePureMathBuiltin::Deg2Rad),
        "exp" => Some(StablePureMathBuiltin::Exp),
        "expm1" => Some(StablePureMathBuiltin::Expm1),
        "fpow" => Some(StablePureMathBuiltin::Fpow),
        "hypot" => Some(StablePureMathBuiltin::Hypot),
        "log" => Some(StablePureMathBuiltin::Log),
        "log10" => Some(StablePureMathBuiltin::Log10),
        "log1p" => Some(StablePureMathBuiltin::Log1p),
        "rad2deg" => Some(StablePureMathBuiltin::Rad2Deg),
        "sin" => Some(StablePureMathBuiltin::Sin),
        "sinh" => Some(StablePureMathBuiltin::Sinh),
        "tan" => Some(StablePureMathBuiltin::Tan),
        "tanh" => Some(StablePureMathBuiltin::Tanh),
        _ => None,
    }
}

/// Scalar conversion and type-name consumers that can stay on the same native
/// value representation as casts and tag tests. Optional or reference-mutating
/// forms deliberately remain on their one baseline continuation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableScalarConsumerBuiltin {
    BoolVal,
    FloatVal,
    IntVal,
    StrVal,
    GetType,
    GetDebugType,
}

pub(super) fn stable_builtin_scalar_consumer(
    target: &RegionCallTarget,
) -> Option<StableScalarConsumerBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "boolval" => Some(StableScalarConsumerBuiltin::BoolVal),
        "floatval" => Some(StableScalarConsumerBuiltin::FloatVal),
        "intval" => Some(StableScalarConsumerBuiltin::IntVal),
        "strval" => Some(StableScalarConsumerBuiltin::StrVal),
        "gettype" => Some(StableScalarConsumerBuiltin::GetType),
        "get_debug_type" => Some(StableScalarConsumerBuiltin::GetDebugType),
        _ => None,
    }
}

/// Numeric builtins that are the function-form counterparts of native
/// arithmetic or one exact pure numeric call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableNumericOperatorBuiltin {
    Pow,
    IntDiv,
    Round,
}

impl StableNumericOperatorBuiltin {
    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Pow | Self::IntDiv => arity == 2,
            Self::Round => arity >= 1 && arity <= 3,
        }
    }
}

pub(super) fn stable_builtin_numeric_operator(
    target: &RegionCallTarget,
) -> Option<StableNumericOperatorBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "pow" => Some(StableNumericOperatorBuiltin::Pow),
        "intdiv" => Some(StableNumericOperatorBuiltin::IntDiv),
        "round" => Some(StableNumericOperatorBuiltin::Round),
        _ => None,
    }
}

/// Exact native handlers for PHP's complete integer/base conversion family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableBaseConversionBuiltin {
    BaseConvert,
    BinDec,
    DecBin,
    DecHex,
    DecOct,
    HexDec,
    OctDec,
}

impl StableBaseConversionBuiltin {
    pub(super) const COUNT: usize = 7;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::BaseConvert => 0,
            Self::BinDec => 1,
            Self::DecBin => 2,
            Self::DecHex => 3,
            Self::DecOct => 4,
            Self::HexDec => 5,
            Self::OctDec => 6,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::BaseConvert => "phrust_native_base_convert",
            Self::BinDec => "phrust_native_bindec",
            Self::DecBin => "phrust_native_decbin",
            Self::DecHex => "phrust_native_dechex",
            Self::DecOct => "phrust_native_decoct",
            Self::HexDec => "phrust_native_hexdec",
            Self::OctDec => "phrust_native_octdec",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::BaseConvert => arity == 3,
            _ => arity == 1,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::BaseConvert,
            Self::BinDec,
            Self::DecBin,
            Self::DecHex,
            Self::DecOct,
            Self::HexDec,
            Self::OctDec,
        ]
    }
}

pub(super) fn stable_builtin_base_conversion(
    target: &RegionCallTarget,
) -> Option<StableBaseConversionBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "base_convert" => Some(StableBaseConversionBuiltin::BaseConvert),
        "bindec" => Some(StableBaseConversionBuiltin::BinDec),
        "decbin" => Some(StableBaseConversionBuiltin::DecBin),
        "dechex" => Some(StableBaseConversionBuiltin::DecHex),
        "decoct" => Some(StableBaseConversionBuiltin::DecOct),
        "hexdec" => Some(StableBaseConversionBuiltin::HexDec),
        "octdec" => Some(StableBaseConversionBuiltin::OctDec),
        _ => None,
    }
}

/// Exact stateless conversions between textual and packed network addresses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableNetworkAddressBuiltin {
    Ip2Long,
    Long2Ip,
    InetPton,
    InetNtop,
}

impl StableNetworkAddressBuiltin {
    pub(super) const COUNT: usize = 4;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Ip2Long => 0,
            Self::Long2Ip => 1,
            Self::InetPton => 2,
            Self::InetNtop => 3,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Ip2Long => "phrust_native_ip2long",
            Self::Long2Ip => "phrust_native_long2ip",
            Self::InetPton => "phrust_native_inet_pton",
            Self::InetNtop => "phrust_native_inet_ntop",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::Ip2Long, Self::Long2Ip, Self::InetPton, Self::InetNtop]
    }
}

pub(super) fn stable_builtin_network_address(
    target: &RegionCallTarget,
) -> Option<StableNetworkAddressBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "ip2long" => Some(StableNetworkAddressBuiltin::Ip2Long),
        "long2ip" => Some(StableNetworkAddressBuiltin::Long2Ip),
        "inet_pton" => Some(StableNetworkAddressBuiltin::InetPton),
        "inet_ntop" => Some(StableNetworkAddressBuiltin::InetNtop),
        _ => None,
    }
}

/// Complete stateless zlib/gzip encode-decode family over native byte strings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableCompressionCodecBuiltin {
    GzEncode,
    GzCompress,
    GzDeflate,
    GzDecode,
    GzUncompress,
    GzInflate,
    ZlibDecode,
    ZlibEncode,
}

impl StableCompressionCodecBuiltin {
    pub(super) const COUNT: usize = 8;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::GzEncode => 0,
            Self::GzCompress => 1,
            Self::GzDeflate => 2,
            Self::GzDecode => 3,
            Self::GzUncompress => 4,
            Self::GzInflate => 5,
            Self::ZlibDecode => 6,
            Self::ZlibEncode => 7,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::GzEncode => "phrust_native_gzencode",
            Self::GzCompress => "phrust_native_gzcompress",
            Self::GzDeflate => "phrust_native_gzdeflate",
            Self::GzDecode => "phrust_native_gzdecode",
            Self::GzUncompress => "phrust_native_gzuncompress",
            Self::GzInflate => "phrust_native_gzinflate",
            Self::ZlibDecode => "phrust_native_zlib_decode",
            Self::ZlibEncode => "phrust_native_zlib_encode",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::GzEncode | Self::GzCompress | Self::GzDeflate => arity >= 1 && arity <= 3,
            Self::GzDecode | Self::GzUncompress | Self::GzInflate | Self::ZlibDecode => {
                arity >= 1 && arity <= 2
            }
            Self::ZlibEncode => arity >= 2 && arity <= 3,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::GzEncode,
            Self::GzCompress,
            Self::GzDeflate,
            Self::GzDecode,
            Self::GzUncompress,
            Self::GzInflate,
            Self::ZlibDecode,
            Self::ZlibEncode,
        ]
    }
}

pub(super) fn stable_builtin_compression_codec(
    target: &RegionCallTarget,
) -> Option<StableCompressionCodecBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "gzencode" => Some(StableCompressionCodecBuiltin::GzEncode),
        "gzcompress" => Some(StableCompressionCodecBuiltin::GzCompress),
        "gzdeflate" => Some(StableCompressionCodecBuiltin::GzDeflate),
        "gzdecode" => Some(StableCompressionCodecBuiltin::GzDecode),
        "gzuncompress" => Some(StableCompressionCodecBuiltin::GzUncompress),
        "gzinflate" => Some(StableCompressionCodecBuiltin::GzInflate),
        "zlib_decode" => Some(StableCompressionCodecBuiltin::ZlibDecode),
        "zlib_encode" => Some(StableCompressionCodecBuiltin::ZlibEncode),
        _ => None,
    }
}

/// Exact symbol operations. The selector is part of the dedicated native ABI
/// and never enters the prepared builtin dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableSymbolQueryBuiltin {
    Define,
    Defined,
    FunctionExists,
    ClassExists,
    InterfaceExists,
    TraitExists,
    EnumExists,
    MethodExists,
    PropertyExists,
}

impl StableSymbolQueryBuiltin {
    pub(super) const COUNT: usize = 9;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Define => 0,
            Self::Defined => 1,
            Self::FunctionExists => 2,
            Self::ClassExists => 3,
            Self::InterfaceExists => 4,
            Self::TraitExists => 5,
            Self::EnumExists => 6,
            Self::MethodExists => 7,
            Self::PropertyExists => 8,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Define => "phrust_native_define",
            Self::Defined => "phrust_native_defined",
            Self::FunctionExists => "phrust_native_function_exists",
            Self::ClassExists => "phrust_native_class_exists",
            Self::InterfaceExists => "phrust_native_interface_exists",
            Self::TraitExists => "phrust_native_trait_exists",
            Self::EnumExists => "phrust_native_enum_exists",
            Self::MethodExists => "phrust_native_method_exists",
            Self::PropertyExists => "phrust_native_property_exists",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Define,
            Self::Defined,
            Self::FunctionExists,
            Self::ClassExists,
            Self::InterfaceExists,
            Self::TraitExists,
            Self::EnumExists,
            Self::MethodExists,
            Self::PropertyExists,
        ]
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Define => arity == 2,
            Self::Defined | Self::FunctionExists => arity == 1,
            Self::ClassExists | Self::InterfaceExists | Self::TraitExists | Self::EnumExists => {
                arity == 1 || arity == 2
            }
            Self::MethodExists | Self::PropertyExists => arity == 2,
        }
    }
}

pub(super) fn stable_builtin_symbol_query(
    target: &RegionCallTarget,
) -> Option<StableSymbolQueryBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "define" => Some(StableSymbolQueryBuiltin::Define),
        "defined" => Some(StableSymbolQueryBuiltin::Defined),
        "function_exists" => Some(StableSymbolQueryBuiltin::FunctionExists),
        "class_exists" => Some(StableSymbolQueryBuiltin::ClassExists),
        "interface_exists" => Some(StableSymbolQueryBuiltin::InterfaceExists),
        "trait_exists" => Some(StableSymbolQueryBuiltin::TraitExists),
        "enum_exists" => Some(StableSymbolQueryBuiltin::EnumExists),
        "method_exists" => Some(StableSymbolQueryBuiltin::MethodExists),
        "property_exists" => Some(StableSymbolQueryBuiltin::PropertyExists),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StablePcreBuiltin {
    Match,
    MatchAll,
    Replace,
    Filter,
    Split,
    Grep,
    Quote,
    LastError,
    LastErrorMessage,
}

impl StablePcreBuiltin {
    pub(super) const COUNT: usize = 9;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Match => 0,
            Self::MatchAll => 1,
            Self::Replace => 2,
            Self::Filter => 3,
            Self::Split => 4,
            Self::Grep => 5,
            Self::Quote => 6,
            Self::LastError => 7,
            Self::LastErrorMessage => 8,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Match => "phrust_native_preg_match",
            Self::MatchAll => "phrust_native_preg_match_all",
            Self::Replace => "phrust_native_preg_replace",
            Self::Filter => "phrust_native_preg_filter",
            Self::Split => "phrust_native_preg_split",
            Self::Grep => "phrust_native_preg_grep",
            Self::Quote => "phrust_native_preg_quote",
            Self::LastError => "phrust_native_preg_last_error",
            Self::LastErrorMessage => "phrust_native_preg_last_error_msg",
        }
    }

    pub(super) const fn argument_is_by_reference(self, index: usize) -> bool {
        matches!(
            (self, index),
            (Self::Match | Self::MatchAll, 2) | (Self::Replace | Self::Filter, 4)
        )
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Match | Self::MatchAll => arity >= 2 && arity <= 5,
            Self::Replace | Self::Filter => arity >= 3 && arity <= 5,
            Self::Split => arity >= 2 && arity <= 4,
            Self::Grep => arity == 2 || arity == 3,
            Self::Quote => arity == 1 || arity == 2,
            Self::LastError | Self::LastErrorMessage => arity == 0,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Match,
            Self::MatchAll,
            Self::Replace,
            Self::Filter,
            Self::Split,
            Self::Grep,
            Self::Quote,
            Self::LastError,
            Self::LastErrorMessage,
        ]
    }
}

/// Non-callback PCRE calls are exact prepared capability handlers. Callback
/// variants stay on the baseline-native callable path because they execute
/// user PHP code.
pub(super) fn stable_builtin_pcre(target: &RegionCallTarget) -> Option<StablePcreBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "preg_match" => Some(StablePcreBuiltin::Match),
        "preg_match_all" => Some(StablePcreBuiltin::MatchAll),
        "preg_replace" => Some(StablePcreBuiltin::Replace),
        "preg_filter" => Some(StablePcreBuiltin::Filter),
        "preg_split" => Some(StablePcreBuiltin::Split),
        "preg_grep" => Some(StablePcreBuiltin::Grep),
        "preg_quote" => Some(StablePcreBuiltin::Quote),
        "preg_last_error" => Some(StablePcreBuiltin::LastError),
        "preg_last_error_msg" => Some(StablePcreBuiltin::LastErrorMessage),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableJsonBuiltin {
    Encode,
    Decode,
    Validate,
    LastError,
    LastErrorMessage,
}

impl StableJsonBuiltin {
    pub(super) const COUNT: usize = 5;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Encode => 0,
            Self::Decode => 1,
            Self::Validate => 2,
            Self::LastError => 3,
            Self::LastErrorMessage => 4,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Encode => "phrust_native_json_encode",
            Self::Decode => "phrust_native_json_decode",
            Self::Validate => "phrust_native_json_validate",
            Self::LastError => "phrust_native_json_last_error",
            Self::LastErrorMessage => "phrust_native_json_last_error_msg",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Encode,
            Self::Decode,
            Self::Validate,
            Self::LastError,
            Self::LastErrorMessage,
        ]
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Encode | Self::Validate => arity >= 1 && arity <= 3,
            Self::Decode => arity >= 2 && arity <= 4,
            Self::LastError | Self::LastErrorMessage => arity == 0,
        }
    }
}

pub(super) fn stable_builtin_json(target: &RegionCallTarget) -> Option<StableJsonBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "json_encode" => Some(StableJsonBuiltin::Encode),
        "json_decode" => Some(StableJsonBuiltin::Decode),
        "json_validate" => Some(StableJsonBuiltin::Validate),
        "json_last_error" => Some(StableJsonBuiltin::LastError),
        "json_last_error_msg" => Some(StableJsonBuiltin::LastErrorMessage),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableFormatBuiltin {
    Sprintf,
    Printf,
    Vsprintf,
    Vprintf,
}

impl StableFormatBuiltin {
    pub(super) const COUNT: usize = 4;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Sprintf => 0,
            Self::Printf => 1,
            Self::Vsprintf => 2,
            Self::Vprintf => 3,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Sprintf => "phrust_native_sprintf",
            Self::Printf => "phrust_native_printf",
            Self::Vsprintf => "phrust_native_vsprintf",
            Self::Vprintf => "phrust_native_vprintf",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::Sprintf, Self::Printf, Self::Vsprintf, Self::Vprintf]
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Sprintf | Self::Printf => arity >= 1 && arity <= 6,
            Self::Vsprintf | Self::Vprintf => arity == 2,
        }
    }
}

pub(super) fn stable_builtin_format(target: &RegionCallTarget) -> Option<StableFormatBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "sprintf" => Some(StableFormatBuiltin::Sprintf),
        "printf" => Some(StableFormatBuiltin::Printf),
        "vsprintf" => Some(StableFormatBuiltin::Vsprintf),
        "vprintf" => Some(StableFormatBuiltin::Vprintf),
        _ => None,
    }
}

/// Exact stateless digest/checksum operations. Each selector resolves to one
/// fixed native symbol; it is never passed to a runtime dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableHashBuiltin {
    Md5,
    Sha1,
    Crc32,
    Hash,
    HashHmac,
    HashEquals,
}

impl StableHashBuiltin {
    pub(super) const COUNT: usize = 6;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Md5 => 0,
            Self::Sha1 => 1,
            Self::Crc32 => 2,
            Self::Hash => 3,
            Self::HashHmac => 4,
            Self::HashEquals => 5,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Md5 => "phrust_native_md5",
            Self::Sha1 => "phrust_native_sha1",
            Self::Crc32 => "phrust_native_crc32",
            Self::Hash => "phrust_native_hash",
            Self::HashHmac => "phrust_native_hash_hmac",
            Self::HashEquals => "phrust_native_hash_equals",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Md5 | Self::Sha1 => arity == 1 || arity == 2,
            Self::Crc32 => arity == 1,
            Self::Hash => arity >= 2 && arity <= 4,
            Self::HashHmac => arity == 3 || arity == 4,
            Self::HashEquals => arity == 2,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Md5,
            Self::Sha1,
            Self::Crc32,
            Self::Hash,
            Self::HashHmac,
            Self::HashEquals,
        ]
    }
}

pub(super) fn stable_builtin_hash(target: &RegionCallTarget) -> Option<StableHashBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "md5" => Some(StableHashBuiltin::Md5),
        "sha1" => Some(StableHashBuiltin::Sha1),
        "crc32" => Some(StableHashBuiltin::Crc32),
        "hash" => Some(StableHashBuiltin::Hash),
        "hash_hmac" => Some(StableHashBuiltin::HashHmac),
        "hash_equals" => Some(StableHashBuiltin::HashEquals),
        _ => None,
    }
}

/// Exact byte-to-byte codec operations over authoritative native strings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableByteCodecBuiltin {
    Base64Encode,
    Base64Decode,
    Bin2Hex,
    Hex2Bin,
    QuotedPrintableDecode,
    UrlEncode,
    RawUrlEncode,
    UrlDecode,
    RawUrlDecode,
    UuEncode,
    UuDecode,
    AddCSlashes,
    StripCSlashes,
    StripSlashes,
    QuoteMeta,
}

impl StableByteCodecBuiltin {
    pub(super) const COUNT: usize = 15;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Base64Encode => 0,
            Self::Base64Decode => 1,
            Self::Bin2Hex => 2,
            Self::Hex2Bin => 3,
            Self::QuotedPrintableDecode => 4,
            Self::UrlEncode => 5,
            Self::RawUrlEncode => 6,
            Self::UrlDecode => 7,
            Self::RawUrlDecode => 8,
            Self::UuEncode => 9,
            Self::UuDecode => 10,
            Self::AddCSlashes => 11,
            Self::StripCSlashes => 12,
            Self::StripSlashes => 13,
            Self::QuoteMeta => 14,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Base64Encode => "phrust_native_base64_encode",
            Self::Base64Decode => "phrust_native_base64_decode",
            Self::Bin2Hex => "phrust_native_bin2hex",
            Self::Hex2Bin => "phrust_native_hex2bin",
            Self::QuotedPrintableDecode => "phrust_native_quoted_printable_decode",
            Self::UrlEncode => "phrust_native_urlencode",
            Self::RawUrlEncode => "phrust_native_rawurlencode",
            Self::UrlDecode => "phrust_native_urldecode",
            Self::RawUrlDecode => "phrust_native_rawurldecode",
            Self::UuEncode => "phrust_native_convert_uuencode",
            Self::UuDecode => "phrust_native_convert_uudecode",
            Self::AddCSlashes => "phrust_native_addcslashes",
            Self::StripCSlashes => "phrust_native_stripcslashes",
            Self::StripSlashes => "phrust_native_stripslashes",
            Self::QuoteMeta => "phrust_native_quotemeta",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Base64Decode => arity == 1 || arity == 2,
            Self::AddCSlashes => arity == 2,
            Self::Base64Encode
            | Self::Bin2Hex
            | Self::Hex2Bin
            | Self::QuotedPrintableDecode
            | Self::UrlEncode
            | Self::RawUrlEncode
            | Self::UrlDecode
            | Self::RawUrlDecode
            | Self::UuEncode
            | Self::UuDecode
            | Self::StripCSlashes
            | Self::StripSlashes
            | Self::QuoteMeta => arity == 1,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Base64Encode,
            Self::Base64Decode,
            Self::Bin2Hex,
            Self::Hex2Bin,
            Self::QuotedPrintableDecode,
            Self::UrlEncode,
            Self::RawUrlEncode,
            Self::UrlDecode,
            Self::RawUrlDecode,
            Self::UuEncode,
            Self::UuDecode,
            Self::AddCSlashes,
            Self::StripCSlashes,
            Self::StripSlashes,
            Self::QuoteMeta,
        ]
    }
}

pub(super) fn stable_builtin_byte_codec(
    target: &RegionCallTarget,
) -> Option<StableByteCodecBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "base64_encode" => Some(StableByteCodecBuiltin::Base64Encode),
        "base64_decode" => Some(StableByteCodecBuiltin::Base64Decode),
        "bin2hex" => Some(StableByteCodecBuiltin::Bin2Hex),
        "hex2bin" => Some(StableByteCodecBuiltin::Hex2Bin),
        "quoted_printable_decode" => Some(StableByteCodecBuiltin::QuotedPrintableDecode),
        "urlencode" => Some(StableByteCodecBuiltin::UrlEncode),
        "rawurlencode" => Some(StableByteCodecBuiltin::RawUrlEncode),
        "urldecode" => Some(StableByteCodecBuiltin::UrlDecode),
        "rawurldecode" => Some(StableByteCodecBuiltin::RawUrlDecode),
        "convert_uuencode" => Some(StableByteCodecBuiltin::UuEncode),
        "convert_uudecode" => Some(StableByteCodecBuiltin::UuDecode),
        "addcslashes" => Some(StableByteCodecBuiltin::AddCSlashes),
        "stripcslashes" => Some(StableByteCodecBuiltin::StripCSlashes),
        "stripslashes" => Some(StableByteCodecBuiltin::StripSlashes),
        "quotemeta" => Some(StableByteCodecBuiltin::QuoteMeta),
        _ => None,
    }
}

/// Exact native searches and comparisons over authoritative byte strings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableStringSearchCompareBuiltin {
    StrStr,
    StrIStr,
    StrRChr,
    StrPBrk,
    SubstrCompare,
    StrNatCmp,
    StrNatCaseCmp,
}

impl StableStringSearchCompareBuiltin {
    pub(super) const COUNT: usize = 7;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::StrStr => 0,
            Self::StrIStr => 1,
            Self::StrRChr => 2,
            Self::StrPBrk => 3,
            Self::SubstrCompare => 4,
            Self::StrNatCmp => 5,
            Self::StrNatCaseCmp => 6,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::StrStr => "phrust_native_strstr",
            Self::StrIStr => "phrust_native_stristr",
            Self::StrRChr => "phrust_native_strrchr",
            Self::StrPBrk => "phrust_native_strpbrk",
            Self::SubstrCompare => "phrust_native_substr_compare",
            Self::StrNatCmp => "phrust_native_strnatcmp",
            Self::StrNatCaseCmp => "phrust_native_strnatcasecmp",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::StrStr | Self::StrIStr | Self::StrRChr => arity == 2 || arity == 3,
            Self::StrPBrk | Self::StrNatCmp | Self::StrNatCaseCmp => arity == 2,
            Self::SubstrCompare => arity >= 3 && arity <= 5,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::StrStr,
            Self::StrIStr,
            Self::StrRChr,
            Self::StrPBrk,
            Self::SubstrCompare,
            Self::StrNatCmp,
            Self::StrNatCaseCmp,
        ]
    }
}

pub(super) fn stable_builtin_string_search_compare(
    target: &RegionCallTarget,
) -> Option<StableStringSearchCompareBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "strstr" => Some(StableStringSearchCompareBuiltin::StrStr),
        "stristr" => Some(StableStringSearchCompareBuiltin::StrIStr),
        "strrchr" => Some(StableStringSearchCompareBuiltin::StrRChr),
        "strpbrk" => Some(StableStringSearchCompareBuiltin::StrPBrk),
        "substr_compare" => Some(StableStringSearchCompareBuiltin::SubstrCompare),
        "strnatcmp" => Some(StableStringSearchCompareBuiltin::StrNatCmp),
        "strnatcasecmp" => Some(StableStringSearchCompareBuiltin::StrNatCaseCmp),
        _ => None,
    }
}

/// Exact native byte-rewrite handlers selected at compilation time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableStringRewriteBuiltin {
    UcWords,
    StrPad,
    StrTr,
    StripTags,
    SubstrReplace,
}

impl StableStringRewriteBuiltin {
    pub(super) const COUNT: usize = 5;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::UcWords => 0,
            Self::StrPad => 1,
            Self::StrTr => 2,
            Self::StripTags => 3,
            Self::SubstrReplace => 4,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::UcWords => "phrust_native_ucwords",
            Self::StrPad => "phrust_native_str_pad",
            Self::StrTr => "phrust_native_strtr",
            Self::StripTags => "phrust_native_strip_tags",
            Self::SubstrReplace => "phrust_native_substr_replace",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::UcWords | Self::StripTags => arity == 1 || arity == 2,
            Self::StrPad => arity >= 2 && arity <= 4,
            Self::SubstrReplace => arity == 3 || arity == 4,
            Self::StrTr => arity == 2 || arity == 3,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::UcWords,
            Self::StrPad,
            Self::StrTr,
            Self::StripTags,
            Self::SubstrReplace,
        ]
    }
}

pub(super) fn stable_builtin_string_rewrite(
    target: &RegionCallTarget,
) -> Option<StableStringRewriteBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "ucwords" => Some(StableStringRewriteBuiltin::UcWords),
        "str_pad" => Some(StableStringRewriteBuiltin::StrPad),
        "strtr" => Some(StableStringRewriteBuiltin::StrTr),
        "strip_tags" => Some(StableStringRewriteBuiltin::StripTags),
        "substr_replace" => Some(StableStringRewriteBuiltin::SubstrReplace),
        _ => None,
    }
}

/// Exact stateless HTML entity codecs over authoritative native bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableHtmlCodecBuiltin {
    SpecialChars,
    Entities,
    EntityDecode,
    SpecialCharsDecode,
}

impl StableHtmlCodecBuiltin {
    pub(super) const COUNT: usize = 4;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::SpecialChars => 0,
            Self::Entities => 1,
            Self::EntityDecode => 2,
            Self::SpecialCharsDecode => 3,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::SpecialChars => "phrust_native_htmlspecialchars",
            Self::Entities => "phrust_native_htmlentities",
            Self::EntityDecode => "phrust_native_html_entity_decode",
            Self::SpecialCharsDecode => "phrust_native_htmlspecialchars_decode",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::SpecialChars | Self::Entities => arity >= 1 && arity <= 4,
            Self::EntityDecode => arity >= 1 && arity <= 3,
            Self::SpecialCharsDecode => arity == 1 || arity == 2,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::SpecialChars,
            Self::Entities,
            Self::EntityDecode,
            Self::SpecialCharsDecode,
        ]
    }
}

pub(super) fn stable_builtin_html_codec(
    target: &RegionCallTarget,
) -> Option<StableHtmlCodecBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "htmlspecialchars" => Some(StableHtmlCodecBuiltin::SpecialChars),
        "htmlentities" => Some(StableHtmlCodecBuiltin::Entities),
        "html_entity_decode" => Some(StableHtmlCodecBuiltin::EntityDecode),
        "htmlspecialchars_decode" => Some(StableHtmlCodecBuiltin::SpecialCharsDecode),
        _ => None,
    }
}

/// Exact URL/query transforms over authoritative native strings and arrays.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableUrlQueryBuiltin {
    ParseUrl,
    ParseStr,
    HttpBuildQuery,
}

impl StableUrlQueryBuiltin {
    pub(super) const COUNT: usize = 3;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::ParseUrl => 0,
            Self::ParseStr => 1,
            Self::HttpBuildQuery => 2,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::ParseUrl => "phrust_native_parse_url",
            Self::ParseStr => "phrust_native_parse_str",
            Self::HttpBuildQuery => "phrust_native_http_build_query",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::ParseUrl => arity == 1 || arity == 2,
            Self::ParseStr => arity == 2,
            Self::HttpBuildQuery => arity >= 1 && arity <= 4,
        }
    }

    pub(super) const fn argument_is_by_reference(self, index: usize) -> bool {
        matches!(self, Self::ParseStr) && index == 1
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::ParseUrl, Self::ParseStr, Self::HttpBuildQuery]
    }
}

pub(super) fn stable_builtin_url_query(target: &RegionCallTarget) -> Option<StableUrlQueryBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "parse_url" => Some(StableUrlQueryBuiltin::ParseUrl),
        "parse_str" => Some(StableUrlQueryBuiltin::ParseStr),
        "http_build_query" => Some(StableUrlQueryBuiltin::HttpBuildQuery),
        _ => None,
    }
}

/// Exact prepared path/filesystem handlers selected at compile time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StablePathBuiltin {
    Basename,
    Dirname,
    Realpath,
    FileExists,
    IsFile,
    IsDir,
    IsReadable,
    IsWritable,
    Filesize,
    Filemtime,
    FileGetContents,
    Fopen,
    Fwrite,
    Fclose,
    Fread,
    Fgets,
    Fgetc,
    Feof,
    Fflush,
    Fseek,
    Ftell,
    Ftruncate,
    Rewind,
    StreamGetContents,
    StreamCopyToStream,
}

impl StablePathBuiltin {
    pub(super) const COUNT: usize = 25;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Basename => 0,
            Self::Dirname => 1,
            Self::Realpath => 2,
            Self::FileExists => 3,
            Self::IsFile => 4,
            Self::IsDir => 5,
            Self::IsReadable => 6,
            Self::IsWritable => 7,
            Self::Filesize => 8,
            Self::Filemtime => 9,
            Self::FileGetContents => 10,
            Self::Fopen => 11,
            Self::Fwrite => 12,
            Self::Fclose => 13,
            Self::Fread => 14,
            Self::Fgets => 15,
            Self::Fgetc => 16,
            Self::Feof => 17,
            Self::Fflush => 18,
            Self::Fseek => 19,
            Self::Ftell => 20,
            Self::Ftruncate => 21,
            Self::Rewind => 22,
            Self::StreamGetContents => 23,
            Self::StreamCopyToStream => 24,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Basename => "phrust_native_basename",
            Self::Dirname => "phrust_native_dirname",
            Self::Realpath => "phrust_native_realpath",
            Self::FileExists => "phrust_native_file_exists",
            Self::IsFile => "phrust_native_is_file",
            Self::IsDir => "phrust_native_is_dir",
            Self::IsReadable => "phrust_native_is_readable",
            Self::IsWritable => "phrust_native_is_writable",
            Self::Filesize => "phrust_native_filesize",
            Self::Filemtime => "phrust_native_filemtime",
            Self::FileGetContents => "phrust_native_file_get_contents",
            Self::Fopen => "phrust_native_fopen",
            Self::Fwrite => "phrust_native_fwrite",
            Self::Fclose => "phrust_native_fclose",
            Self::Fread => "phrust_native_fread",
            Self::Fgets => "phrust_native_fgets",
            Self::Fgetc => "phrust_native_fgetc",
            Self::Feof => "phrust_native_feof",
            Self::Fflush => "phrust_native_fflush",
            Self::Fseek => "phrust_native_fseek",
            Self::Ftell => "phrust_native_ftell",
            Self::Ftruncate => "phrust_native_ftruncate",
            Self::Rewind => "phrust_native_rewind",
            Self::StreamGetContents => "phrust_native_stream_get_contents",
            Self::StreamCopyToStream => "phrust_native_stream_copy_to_stream",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Basename | Self::Dirname => arity == 1 || arity == 2,
            Self::Realpath
            | Self::FileExists
            | Self::IsFile
            | Self::IsDir
            | Self::IsReadable
            | Self::IsWritable
            | Self::Filesize
            | Self::Filemtime => arity == 1,
            Self::FileGetContents => arity >= 1 && arity <= 5,
            // Optional fopen include-path/context shapes retain their one
            // baseline continuation until those capabilities are published.
            Self::Fopen => arity == 2,
            Self::Fwrite => arity == 2 || arity == 3,
            Self::Fclose | Self::Fgetc | Self::Feof | Self::Fflush | Self::Ftell | Self::Rewind => {
                arity == 1
            }
            Self::Fread | Self::Ftruncate => arity == 2,
            Self::Fgets => arity == 1 || arity == 2,
            Self::Fseek => arity == 2 || arity == 3,
            Self::StreamGetContents => arity >= 1 && arity <= 3,
            Self::StreamCopyToStream => arity >= 2 && arity <= 4,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Basename,
            Self::Dirname,
            Self::Realpath,
            Self::FileExists,
            Self::IsFile,
            Self::IsDir,
            Self::IsReadable,
            Self::IsWritable,
            Self::Filesize,
            Self::Filemtime,
            Self::FileGetContents,
            Self::Fopen,
            Self::Fwrite,
            Self::Fclose,
            Self::Fread,
            Self::Fgets,
            Self::Fgetc,
            Self::Feof,
            Self::Fflush,
            Self::Fseek,
            Self::Ftell,
            Self::Ftruncate,
            Self::Rewind,
            Self::StreamGetContents,
            Self::StreamCopyToStream,
        ]
    }
}

pub(super) fn stable_builtin_path(target: &RegionCallTarget) -> Option<StablePathBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "basename" => Some(StablePathBuiltin::Basename),
        "dirname" => Some(StablePathBuiltin::Dirname),
        "realpath" => Some(StablePathBuiltin::Realpath),
        "file_exists" => Some(StablePathBuiltin::FileExists),
        "is_file" => Some(StablePathBuiltin::IsFile),
        "is_dir" => Some(StablePathBuiltin::IsDir),
        "is_readable" => Some(StablePathBuiltin::IsReadable),
        "is_writable" => Some(StablePathBuiltin::IsWritable),
        "filesize" => Some(StablePathBuiltin::Filesize),
        "filemtime" => Some(StablePathBuiltin::Filemtime),
        "file_get_contents" => Some(StablePathBuiltin::FileGetContents),
        "fopen" => Some(StablePathBuiltin::Fopen),
        "fwrite" => Some(StablePathBuiltin::Fwrite),
        "fclose" => Some(StablePathBuiltin::Fclose),
        "fread" => Some(StablePathBuiltin::Fread),
        "fgets" => Some(StablePathBuiltin::Fgets),
        "fgetc" => Some(StablePathBuiltin::Fgetc),
        "feof" => Some(StablePathBuiltin::Feof),
        "fflush" => Some(StablePathBuiltin::Fflush),
        "fseek" => Some(StablePathBuiltin::Fseek),
        "ftell" => Some(StablePathBuiltin::Ftell),
        "ftruncate" => Some(StablePathBuiltin::Ftruncate),
        "rewind" => Some(StablePathBuiltin::Rewind),
        "stream_get_contents" => Some(StablePathBuiltin::StreamGetContents),
        "stream_copy_to_stream" => Some(StablePathBuiltin::StreamCopyToStream),
        _ => None,
    }
}

/// Exact request-local output-buffer operations selected at compile time.
///
/// The default output-buffer operations consume only the authoritative
/// request output stack and native string plane. `ob_start()` callback,
/// chunk-size, and flags shapes retain their one baseline continuation until
/// a native output-handler stack is published.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableOutputBufferBuiltin {
    Start,
    GetClean,
    GetContents,
    GetFlush,
    GetLength,
    GetLevel,
    EndFlush,
    EndClean,
}

impl StableOutputBufferBuiltin {
    pub(super) const COUNT: usize = 8;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Start => 0,
            Self::GetClean => 1,
            Self::GetContents => 2,
            Self::GetFlush => 3,
            Self::GetLength => 4,
            Self::GetLevel => 5,
            Self::EndFlush => 6,
            Self::EndClean => 7,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Start => "phrust_native_ob_start",
            Self::GetClean => "phrust_native_ob_get_clean",
            Self::GetContents => "phrust_native_ob_get_contents",
            Self::GetFlush => "phrust_native_ob_get_flush",
            Self::GetLength => "phrust_native_ob_get_length",
            Self::GetLevel => "phrust_native_ob_get_level",
            Self::EndFlush => "phrust_native_ob_end_flush",
            Self::EndClean => "phrust_native_ob_end_clean",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        // Optional ob_start arguments select output-handler semantics that are
        // deliberately baseline-only until their state has a native record.
        arity == 0
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Start,
            Self::GetClean,
            Self::GetContents,
            Self::GetFlush,
            Self::GetLength,
            Self::GetLevel,
            Self::EndFlush,
            Self::EndClean,
        ]
    }
}

pub(super) fn stable_builtin_output_buffer(
    target: &RegionCallTarget,
) -> Option<StableOutputBufferBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "ob_start" => Some(StableOutputBufferBuiltin::Start),
        "ob_get_clean" => Some(StableOutputBufferBuiltin::GetClean),
        "ob_get_contents" => Some(StableOutputBufferBuiltin::GetContents),
        "ob_get_flush" => Some(StableOutputBufferBuiltin::GetFlush),
        "ob_get_length" => Some(StableOutputBufferBuiltin::GetLength),
        "ob_get_level" => Some(StableOutputBufferBuiltin::GetLevel),
        "ob_end_flush" => Some(StableOutputBufferBuiltin::EndFlush),
        "ob_end_clean" => Some(StableOutputBufferBuiltin::EndClean),
        _ => None,
    }
}

pub(super) fn stable_builtin_length(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\').to_ascii_lowercase();
    if normalized.contains('\\') {
        return None;
    }
    match normalized.as_str() {
        "strlen" => Some(0),
        "count" => Some(1),
        _ => None,
    }
}

pub(super) fn stable_builtin_array_key_exists(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\')
        && (normalized.eq_ignore_ascii_case("array_key_exists")
            || normalized.eq_ignore_ascii_case("key_exists"))
}

pub(super) fn stable_builtin_string_predicate(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "str_contains" => Some(0),
        "str_starts_with" => Some(1),
        "str_ends_with" => Some(2),
        _ => None,
    }
}

/// ASCII-only case conversion builtins whose PHP 8 semantics can be emitted
/// directly over the request-owned native string arena.  The numeric value is
/// an internal lowering selector, never a runtime helper operation ID.
pub(super) fn stable_builtin_ascii_case(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "strtolower" => Some(0),
        "strtoupper" => Some(1),
        _ => None,
    }
}

/// Byte-preserving transforms over one native string. The selector chooses
/// reverse, lowercase-first-byte, or uppercase-first-byte behavior.
pub(super) fn stable_builtin_string_transform(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "strrev" => Some(0),
        "lcfirst" => Some(1),
        "ucfirst" => Some(2),
        _ => None,
    }
}

pub(super) fn stable_builtin_str_repeat(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("str_repeat")
}

pub(super) fn stable_builtin_addslashes(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("addslashes")
}

pub(super) fn stable_builtin_substr_count(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("substr_count")
}

/// Native byte comparisons. Bit zero selects ASCII case folding; bit one
/// selects the explicit maximum-length variants.
pub(super) fn stable_builtin_string_compare(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "strcmp" => Some(0),
        "strcasecmp" => Some(1),
        "strncmp" => Some(2),
        "strncasecmp" => Some(3),
        _ => None,
    }
}

/// Byte-position builtins with an exact positional native lowering. The low
/// bit selects ASCII case folding; the high bit selects reverse search.
pub(super) fn stable_builtin_string_position(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "strpos" => Some(0),
        "stripos" => Some(1),
        "strrpos" => Some(2),
        "strripos" => Some(3),
        _ => None,
    }
}

pub(super) fn stable_builtin_ord(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("ord")
}

pub(super) fn stable_builtin_chr(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("chr")
}

/// Native byte-slice transformations. `substr` has its own argument plan;
/// trim selectors encode left/right default-mask trimming.
pub(super) fn stable_builtin_default_trim(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "trim" => Some(0),
        "ltrim" => Some(1),
        "rtrim" => Some(2),
        _ => None,
    }
}

pub(super) fn stable_builtin_substr(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("substr")
}

/// Direct array projections whose result is another authoritative native
/// array. The selector chooses source keys or source values.
pub(super) fn stable_builtin_array_projection(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "array_keys" => Some(0),
        "array_values" => Some(1),
        _ => None,
    }
}

/// Direct constructors whose result is an authoritative native array.
pub(super) fn stable_builtin_array_constructor(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "array_fill" => Some(0),
        "array_fill_keys" => Some(1),
        "array_combine" => Some(2),
        "array_flip" => Some(3),
        _ => None,
    }
}

/// Representation-complete array shape operations. The selector covers the
/// remaining pure constructors/transforms that still entered the baseline
/// prepared-builtin executor as one family.
pub(super) fn stable_builtin_array_shape(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "range" => Some(0),
        "array_pad" => Some(1),
        "array_chunk" => Some(2),
        "array_column" => Some(3),
        "array_unique" => Some(4),
        _ => None,
    }
}

/// Key-preserving, callback-free array sorts over authoritative direct
/// entries. Each operation has a fixed ABI; comparison mode remains a
/// PHP-visible argument and unsupported modes take one baseline continuation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableArrayPreservingSortBuiltin {
    Asort,
    Arsort,
    Ksort,
    Krsort,
    Natsort,
    Natcasesort,
}

impl StableArrayPreservingSortBuiltin {
    pub(super) const COUNT: usize = 6;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Asort => 0,
            Self::Arsort => 1,
            Self::Ksort => 2,
            Self::Krsort => 3,
            Self::Natsort => 4,
            Self::Natcasesort => 5,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Asort => "phrust_native_asort",
            Self::Arsort => "phrust_native_arsort",
            Self::Ksort => "phrust_native_ksort",
            Self::Krsort => "phrust_native_krsort",
            Self::Natsort => "phrust_native_natsort",
            Self::Natcasesort => "phrust_native_natcasesort",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Natsort | Self::Natcasesort => arity == 1,
            _ => arity == 1 || arity == 2,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Asort,
            Self::Arsort,
            Self::Ksort,
            Self::Krsort,
            Self::Natsort,
            Self::Natcasesort,
        ]
    }
}

pub(super) fn stable_builtin_array_preserving_sort(
    target: &RegionCallTarget,
) -> Option<StableArrayPreservingSortBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "asort" => Some(StableArrayPreservingSortBuiltin::Asort),
        "arsort" => Some(StableArrayPreservingSortBuiltin::Arsort),
        "ksort" => Some(StableArrayPreservingSortBuiltin::Ksort),
        "krsort" => Some(StableArrayPreservingSortBuiltin::Krsort),
        "natsort" => Some(StableArrayPreservingSortBuiltin::Natsort),
        "natcasesort" => Some(StableArrayPreservingSortBuiltin::Natcasesort),
        _ => None,
    }
}

/// Introspection over the active native PHP call frame. The frame already
/// carries authoritative native encodings; these fixed handlers expose that
/// view without entering the generic builtin dispatcher or materializing
/// Rust `Value` trees.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableFrameIntrospectionBuiltin {
    NumArgs,
    GetArg,
    GetArgs,
}

impl StableFrameIntrospectionBuiltin {
    pub(super) const COUNT: usize = 3;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::NumArgs => 0,
            Self::GetArg => 1,
            Self::GetArgs => 2,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::NumArgs => "phrust_native_func_num_args",
            Self::GetArg => "phrust_native_func_get_arg",
            Self::GetArgs => "phrust_native_func_get_args",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::GetArg => arity == 1,
            Self::NumArgs | Self::GetArgs => arity == 0,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::NumArgs, Self::GetArg, Self::GetArgs]
    }
}

pub(super) fn stable_builtin_frame_introspection(
    target: &RegionCallTarget,
) -> Option<StableFrameIntrospectionBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "func_num_args" => Some(StableFrameIntrospectionBuiltin::NumArgs),
        "func_get_arg" => Some(StableFrameIntrospectionBuiltin::GetArg),
        "func_get_args" => Some(StableFrameIntrospectionBuiltin::GetArgs),
        _ => None,
    }
}

/// Stable object identity reads over authoritative direct object and closure
/// owners. Hash and integer forms have separate fixed ABIs; neither operation
/// enters generic SPL dispatch or materializes declared object properties.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableObjectIdentityBuiltin {
    Hash,
    Id,
}

impl StableObjectIdentityBuiltin {
    pub(super) const COUNT: usize = 2;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Hash => 0,
            Self::Id => 1,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Hash => "phrust_native_spl_object_hash",
            Self::Id => "phrust_native_spl_object_id",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::Hash, Self::Id]
    }
}

pub(super) fn stable_builtin_object_identity(
    target: &RegionCallTarget,
) -> Option<StableObjectIdentityBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "spl_object_hash" => Some(StableObjectIdentityBuiltin::Hash),
        "spl_object_id" => Some(StableObjectIdentityBuiltin::Id),
        _ => None,
    }
}

/// PHP wire serialization over authoritative native scalar/array graphs.
/// Object hooks, reference records, request-specific float precision, and
/// malformed-input diagnostics retain one baseline continuation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableSerializationBuiltin {
    Serialize,
    Unserialize,
}

impl StableSerializationBuiltin {
    pub(super) const COUNT: usize = 2;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Serialize => 0,
            Self::Unserialize => 1,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Serialize => "phrust_native_serialize",
            Self::Unserialize => "phrust_native_unserialize",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::Serialize, Self::Unserialize]
    }
}

pub(super) fn stable_builtin_serialization(
    target: &RegionCallTarget,
) -> Option<StableSerializationBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "serialize" => Some(StableSerializationBuiltin::Serialize),
        "unserialize" => Some(StableSerializationBuiltin::Unserialize),
        _ => None,
    }
}

/// Stable object-property projections over the authoritative native object
/// layout. Visible and mangled forms have separate fixed ABIs so optimizing
/// code never enters generic builtin dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableObjectVarsBuiltin {
    Visible,
    Mangled,
}

impl StableObjectVarsBuiltin {
    pub(super) const COUNT: usize = 2;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Visible => 0,
            Self::Mangled => 1,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Visible => "phrust_native_get_object_vars",
            Self::Mangled => "phrust_native_get_mangled_object_vars",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::Visible, Self::Mangled]
    }
}

pub(super) fn stable_builtin_object_vars(
    target: &RegionCallTarget,
) -> Option<StableObjectVarsBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "get_object_vars" => Some(StableObjectVarsBuiltin::Visible),
        "get_mangled_object_vars" => Some(StableObjectVarsBuiltin::Mangled),
        _ => None,
    }
}

pub(super) fn stable_builtin_get_class(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("get_class")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableClassLineageBuiltin {
    ParentClass,
    IsSubclassOf,
}

impl StableClassLineageBuiltin {
    pub(super) const COUNT: usize = 2;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::ParentClass => 0,
            Self::IsSubclassOf => 1,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::ParentClass => "phrust_native_get_parent_class",
            Self::IsSubclassOf => "phrust_native_is_subclass_of",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::ParentClass => arity == 1,
            Self::IsSubclassOf => arity == 2 || arity == 3,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::ParentClass, Self::IsSubclassOf]
    }
}

pub(super) fn stable_builtin_class_lineage(
    target: &RegionCallTarget,
) -> Option<StableClassLineageBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "get_parent_class" => Some(StableClassLineageBuiltin::ParentClass),
        "is_subclass_of" => Some(StableClassLineageBuiltin::IsSubclassOf),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableExtensionQueryBuiltin {
    IsLoaded,
    LoadedNames,
}

impl StableExtensionQueryBuiltin {
    pub(super) const COUNT: usize = 2;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::IsLoaded => 0,
            Self::LoadedNames => 1,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::IsLoaded => "phrust_native_extension_loaded",
            Self::LoadedNames => "phrust_native_get_loaded_extensions",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::IsLoaded => arity == 1,
            Self::LoadedNames => arity <= 1,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::IsLoaded, Self::LoadedNames]
    }
}

pub(super) fn stable_builtin_extension_query(
    target: &RegionCallTarget,
) -> Option<StableExtensionQueryBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "extension_loaded" => Some(StableExtensionQueryBuiltin::IsLoaded),
        "get_loaded_extensions" => Some(StableExtensionQueryBuiltin::LoadedNames),
        _ => None,
    }
}

/// Stable reads from the request-published native INI registry. Mutating
/// configuration remains a separate semantic family because it also updates
/// execution-coordinator state such as include paths and diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableIniQueryBuiltin {
    IniGet,
    CfgVar,
    IncludePath,
}

impl StableIniQueryBuiltin {
    pub(super) const COUNT: usize = 3;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::IniGet => 0,
            Self::CfgVar => 1,
            Self::IncludePath => 2,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::IniGet => "phrust_native_ini_get",
            Self::CfgVar => "phrust_native_get_cfg_var",
            Self::IncludePath => "phrust_native_get_include_path",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::IniGet | Self::CfgVar => arity == 1,
            Self::IncludePath => arity == 0,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [Self::IniGet, Self::CfgVar, Self::IncludePath]
    }
}

pub(super) fn stable_builtin_ini_query(target: &RegionCallTarget) -> Option<StableIniQueryBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "ini_get" => Some(StableIniQueryBuiltin::IniGet),
        "get_cfg_var" => Some(StableIniQueryBuiltin::CfgVar),
        "get_include_path" => Some(StableIniQueryBuiltin::IncludePath),
        _ => None,
    }
}

/// Stable reads from request-published execution context.
///
/// Each variant lowers to its own exact ABI. The enum is compile-time
/// metadata only: generated code never supplies an operation ID and the
/// optimizing artifact cannot reach the generic builtin dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableRequestQueryBuiltin {
    TempDir,
    CurrentDirectory,
    Environment,
    SapiName,
    Uname,
    CurrentUser,
    IncludedFiles,
}

impl StableRequestQueryBuiltin {
    pub(super) const COUNT: usize = 7;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::TempDir => 0,
            Self::CurrentDirectory => 1,
            Self::Environment => 2,
            Self::SapiName => 3,
            Self::Uname => 4,
            Self::CurrentUser => 5,
            Self::IncludedFiles => 6,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::TempDir => "phrust_native_sys_get_temp_dir",
            Self::CurrentDirectory => "phrust_native_getcwd",
            Self::Environment => "phrust_native_getenv",
            Self::SapiName => "phrust_native_php_sapi_name",
            Self::Uname => "phrust_native_php_uname",
            Self::CurrentUser => "phrust_native_get_current_user",
            Self::IncludedFiles => "phrust_native_get_included_files",
        }
    }

    pub(super) const fn accepts_arity(self, arity: usize) -> bool {
        match self {
            Self::Environment | Self::Uname => arity <= 1,
            Self::TempDir
            | Self::CurrentDirectory
            | Self::SapiName
            | Self::CurrentUser
            | Self::IncludedFiles => arity == 0,
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::TempDir,
            Self::CurrentDirectory,
            Self::Environment,
            Self::SapiName,
            Self::Uname,
            Self::CurrentUser,
            Self::IncludedFiles,
        ]
    }
}

pub(super) fn stable_builtin_request_query(
    target: &RegionCallTarget,
) -> Option<StableRequestQueryBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "sys_get_temp_dir" => Some(StableRequestQueryBuiltin::TempDir),
        "getcwd" => Some(StableRequestQueryBuiltin::CurrentDirectory),
        "getenv" => Some(StableRequestQueryBuiltin::Environment),
        "php_sapi_name" => Some(StableRequestQueryBuiltin::SapiName),
        "php_uname" => Some(StableRequestQueryBuiltin::Uname),
        "get_current_user" => Some(StableRequestQueryBuiltin::CurrentUser),
        "get_included_files" | "get_required_files" => {
            Some(StableRequestQueryBuiltin::IncludedFiles)
        }
        _ => None,
    }
}

/// Immutable declaration-name inventories published by the compiled unit.
///
/// Values are constructed directly as native arrays. Runtime-valued
/// constants deliberately remain a separate storage family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StableDeclarationInventoryBuiltin {
    Functions,
    Classes,
    Interfaces,
    Traits,
}

impl StableDeclarationInventoryBuiltin {
    pub(super) const COUNT: usize = 4;

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Functions => 0,
            Self::Classes => 1,
            Self::Interfaces => 2,
            Self::Traits => 3,
        }
    }

    pub(super) const fn symbol(self) -> &'static str {
        match self {
            Self::Functions => "phrust_native_get_defined_functions",
            Self::Classes => "phrust_native_get_declared_classes",
            Self::Interfaces => "phrust_native_get_declared_interfaces",
            Self::Traits => "phrust_native_get_declared_traits",
        }
    }

    pub(super) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Functions,
            Self::Classes,
            Self::Interfaces,
            Self::Traits,
        ]
    }
}

pub(super) fn stable_builtin_declaration_inventory(
    target: &RegionCallTarget,
) -> Option<StableDeclarationInventoryBuiltin> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "get_defined_functions" => Some(StableDeclarationInventoryBuiltin::Functions),
        "get_declared_classes" => Some(StableDeclarationInventoryBuiltin::Classes),
        "get_declared_interfaces" => Some(StableDeclarationInventoryBuiltin::Interfaces),
        "get_declared_traits" => Some(StableDeclarationInventoryBuiltin::Traits),
        _ => None,
    }
}

pub(super) fn stable_builtin_constant_inventory(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("get_defined_constants")
}

pub(super) fn stable_builtin_compact(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("compact")
}

/// Non-callback array set and overlay operations over authoritative direct
/// entries. Callback comparators and recursive overlays remain distinct
/// baseline semantics instead of being smuggled through this fixed family.
pub(super) fn stable_builtin_array_set(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "array_diff" => Some(0),
        "array_diff_assoc" => Some(1),
        "array_diff_key" => Some(2),
        "array_intersect" => Some(3),
        "array_intersect_assoc" => Some(4),
        "array_intersect_key" => Some(5),
        "array_replace" => Some(6),
        _ => None,
    }
}

/// Callback-neutral array transforms. The selector distinguishes
/// `array_map(null, $array)` from `array_filter($array[, null])`; callable
/// forms deliberately take the single baseline continuation until callback
/// invocation itself is native inside the generated loop.
pub(super) fn stable_builtin_callback_neutral_array(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "array_map" => Some(0),
        "array_filter" => Some(1),
        _ => None,
    }
}

/// Strict native array membership operations. The selector distinguishes a
/// boolean membership result from the matching key result.
pub(super) fn stable_builtin_array_lookup(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "in_array" => Some(0),
        "array_search" => Some(1),
        _ => None,
    }
}

/// Array-key queries that preserve the source key representation.
pub(super) fn stable_builtin_array_edge_key(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "array_key_first" => Some(0),
        "array_key_last" => Some(1),
        _ => None,
    }
}

/// PHP array internal-pointer operations. Read-only selectors consume the
/// authoritative native slot; mutating selectors require an exact caller
/// local and update that slot after COW separation.
pub(super) fn stable_builtin_array_pointer(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "current" => Some(0),
        "key" => Some(1),
        "next" => Some(2),
        "reset" => Some(3),
        "prev" => Some(4),
        "end" => Some(5),
        _ => None,
    }
}

/// Exact local-mutating array deque operations over authoritative entries.
/// Pop/shift move one element owner into the result; push/unshift retain the
/// prepared positional values once for the array. Numeric-key reindexing for
/// the front operations happens directly in the stable entry range.
pub(super) fn stable_builtin_array_stack(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "array_pop" => Some(0),
        "array_push" => Some(1),
        "array_shift" => Some(2),
        "array_unshift" => Some(3),
        _ => None,
    }
}

pub(super) fn stable_builtin_array_is_list(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("array_is_list")
}

pub(super) fn stable_builtin_implode(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\')
        && (normalized.eq_ignore_ascii_case("implode") || normalized.eq_ignore_ascii_case("join"))
}

pub(super) fn stable_builtin_explode(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("explode")
}

pub(super) fn stable_builtin_array_slice(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("array_slice")
}

pub(super) fn stable_builtin_array_reverse(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("array_reverse")
}

pub(super) fn stable_builtin_array_merge(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("array_merge")
}

pub(super) fn stable_builtin_str_replace(target: &RegionCallTarget) -> bool {
    let RegionCallTarget::Function { name, .. } = target else {
        return false;
    };
    let normalized = name.trim_start_matches('\\');
    !normalized.contains('\\') && normalized.eq_ignore_ascii_case("str_replace")
}

pub(super) fn stable_builtin_string_span(target: &RegionCallTarget) -> Option<u32> {
    let RegionCallTarget::Function { name, .. } = target else {
        return None;
    };
    let normalized = name.trim_start_matches('\\');
    if normalized.contains('\\') {
        return None;
    }
    match normalized.to_ascii_lowercase().as_str() {
        "strspn" => Some(0),
        "strcspn" => Some(1),
        _ => None,
    }
}

pub(super) fn native_argument_flags(argument: &php_ir::instruction::IrCallArg) -> u32 {
    let mut flags = crate::JitNativeArgFlags::default();
    if argument.name.is_some() {
        flags = flags.union(crate::JitNativeArgFlags::NAMED);
    }
    if argument.unpack {
        flags = flags.union(crate::JitNativeArgFlags::UNPACK);
    }
    if argument.by_ref_local.is_some()
        || argument.by_ref_dim.is_some()
        || argument.by_ref_property.is_some()
        || argument.by_ref_property_dim.is_some()
    {
        flags = flags.union(crate::JitNativeArgFlags::BY_REFERENCE);
    }
    if argument.value_kind == php_ir::instruction::IrCallArgValueKind::IndirectTemporary {
        flags = flags.union(crate::JitNativeArgFlags::INDIRECT_TEMPORARY);
    }
    flags.0
}

pub(super) fn native_argument_has_location(argument: &php_ir::instruction::IrCallArg) -> bool {
    argument.by_ref_local.is_some()
        || argument.by_ref_dim.is_some()
        || argument.by_ref_property.is_some()
        || argument.by_ref_property_dim.is_some()
}

pub(super) fn known_user_argument_requires_reference(
    call: &RegionNativeCall,
    index: usize,
    function_params: &BTreeMap<FunctionId, NativeFunctionMetadata>,
    external_function_signatures: &[crate::JitExternalFunctionSignature],
    caller: FunctionId,
) -> Option<bool> {
    let argument = call.args.get(index)?;
    if let Some(requirement) = call.declared_argument_reference_requirement(index) {
        return Some(requirement);
    }
    if matches!(call.target, RegionCallTarget::Method { .. }) {
        // A dynamic receiver may resolve to any visible userland or internal
        // class. Method-name-only arginfo is therefore not authoritative:
        // an unrelated internal `get()` must not classify a userland
        // `get(&$value)` as by-value. The resolved dispatcher publishes the
        // exact parameter flags and the caller restores speculative local
        // bindings after the call.
        return None;
    }
    if let RegionCallTarget::Function {
        name,
        function: None,
    } = &call.target
    {
        let normalized = name.trim_start_matches('\\');
        let has_local_metadata = function_params.values().any(|metadata| {
            metadata
                .name
                .trim_start_matches('\\')
                .eq_ignore_ascii_case(normalized)
        });
        if !has_local_metadata {
            let signature = external_function_signatures.iter().find(|signature| {
                signature.published
                    && signature
                        .name
                        .trim_start_matches('\\')
                        .eq_ignore_ascii_case(normalized)
            })?;
            let parameter = argument.name.as_deref().map_or_else(
                || {
                    signature.params.get(index).or_else(|| {
                        signature
                            .params
                            .last()
                            .filter(|parameter| parameter.variadic)
                    })
                },
                |name| {
                    signature
                        .params
                        .iter()
                        .find(|parameter| parameter.name == name)
                        .or_else(|| {
                            signature
                                .params
                                .last()
                                .filter(|parameter| parameter.variadic)
                        })
                },
            );
            return Some(parameter.is_some_and(|parameter| parameter.by_ref));
        }
    }
    let method_matches = |candidate: &str, method: &str| {
        candidate
            .rsplit_once("::")
            .is_some_and(|(_, candidate_method)| candidate_method.eq_ignore_ascii_case(method))
    };
    let metadata = match &call.target {
        RegionCallTarget::Function {
            name,
            function: None,
        } => {
            let normalized = name.trim_start_matches('\\');
            vec![function_params.values().find(|metadata| {
                metadata
                    .name
                    .trim_start_matches('\\')
                    .eq_ignore_ascii_case(normalized)
            })?]
        }
        RegionCallTarget::Function {
            function: Some(function),
            ..
        } => vec![function_params.get(function)?],
        RegionCallTarget::StaticMethod { class_name, method } => {
            let resolved_class = if matches!(class_name.as_str(), "self" | "static") {
                function_params
                    .get(&caller)
                    .and_then(|metadata| metadata.name.rsplit_once("::").map(|(class, _)| class))
            } else {
                Some(class_name.trim_start_matches('\\'))
            };
            let exact = resolved_class.and_then(|class| {
                function_params.values().find(|metadata| {
                    metadata.name.rsplit_once("::").is_some_and(
                        |(candidate_class, candidate_method)| {
                            candidate_class
                                .trim_start_matches('\\')
                                .eq_ignore_ascii_case(class)
                                && candidate_method.eq_ignore_ascii_case(method)
                        },
                    )
                })
            });
            exact.map_or_else(
                || {
                    function_params
                        .values()
                        .filter(|metadata| method_matches(&metadata.name, method))
                        .collect()
                },
                |metadata| vec![metadata],
            )
        }
        RegionCallTarget::Method { .. } => unreachable!("handled before metadata lookup"),
        RegionCallTarget::Constructor { class_name, .. } => function_params
            .values()
            .filter(|metadata| {
                metadata
                    .name
                    .rsplit_once("::")
                    .is_some_and(|(class, method)| {
                        class
                            .trim_start_matches('\\')
                            .eq_ignore_ascii_case(class_name.trim_start_matches('\\'))
                            && method.eq_ignore_ascii_case("__construct")
                    })
            })
            .collect(),
        _ => return None,
    };
    let mut requirements = metadata.into_iter().map(|metadata| {
        let parameters = &metadata.params;
        argument
            .name
            .as_deref()
            .map_or_else(
                || {
                    parameters
                        .get(index)
                        .or_else(|| parameters.last().filter(|parameter| parameter.variadic))
                },
                |name| {
                    parameters
                        .iter()
                        .find(|parameter| parameter.name == name)
                        .or_else(|| parameters.last().filter(|parameter| parameter.variadic))
                },
            )
            .is_some_and(|parameter| parameter.by_ref)
    });
    let requirement = requirements.next()?;
    requirements
        .all(|candidate| candidate == requirement)
        .then_some(requirement)
}
