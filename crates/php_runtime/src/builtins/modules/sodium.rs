//! Real-crypto sodium MVP for common dependency probes.

use super::core::{arity_error, int_arg, string_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinError, BuiltinResult,
    RuntimeSourceSpan,
};
use base64::{Engine, engine::general_purpose};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "sodium_bin2hex",
        builtin_sodium_bin2hex,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "sodium_crypto_generichash",
        builtin_sodium_crypto_generichash,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "sodium_crypto_generichash_keygen",
        builtin_sodium_crypto_generichash_keygen,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "sodium_crypto_sign_detached",
        builtin_sodium_crypto_sign_detached,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "sodium_crypto_sign_verify_detached",
        builtin_sodium_crypto_sign_verify_detached,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "sodium_hex2bin",
        builtin_sodium_hex2bin,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "sodium_base642bin",
        builtin_sodium_base642bin,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "sodium_bin2base64",
        builtin_sodium_bin2base64,
        BuiltinCompatibility::Php,
    ),
];

const SODIUM_CRYPTO_GENERICHASH_BYTES: usize = 32;
const SODIUM_CRYPTO_GENERICHASH_BYTES_MIN: usize = 16;
const SODIUM_CRYPTO_GENERICHASH_BYTES_MAX: usize = 64;
const SODIUM_CRYPTO_GENERICHASH_KEYBYTES: usize = 32;
const SODIUM_CRYPTO_SIGN_BYTES: usize = 64;
const SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES: usize = 32;
const SODIUM_CRYPTO_SIGN_SECRETKEYBYTES: usize = 64;
const SODIUM_BASE64_VARIANT_ORIGINAL: i64 = 1;
const SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING: i64 = 3;
const SODIUM_BASE64_VARIANT_URLSAFE: i64 = 5;
const SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING: i64 = 7;

fn builtin_sodium_crypto_generichash(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error(
            "sodium_crypto_generichash",
            "one to three arguments",
        ));
    }
    let message = string_arg("sodium_crypto_generichash", &args[0])?;
    let key = args
        .get(1)
        .map(|value| string_arg("sodium_crypto_generichash", value))
        .transpose()?;
    let length = args
        .get(2)
        .map(|value| int_arg("sodium_crypto_generichash", value))
        .transpose()?
        .unwrap_or(SODIUM_CRYPTO_GENERICHASH_BYTES as i64);
    if !(SODIUM_CRYPTO_GENERICHASH_BYTES_MIN as i64..=SODIUM_CRYPTO_GENERICHASH_BYTES_MAX as i64)
        .contains(&length)
    {
        return Err(value_error(
            "sodium_crypto_generichash",
            "output length must be between 16 and 64 bytes",
        ));
    }
    let mut params = blake2b_simd::Params::new();
    params.hash_length(length as usize);
    if let Some(key) = key.as_ref()
        && !key.is_empty()
    {
        if key.len() > SODIUM_CRYPTO_GENERICHASH_BYTES_MAX {
            return Err(value_error(
                "sodium_crypto_generichash",
                "key length must be at most 64 bytes",
            ));
        }
        params.key(key.as_bytes());
    }
    Ok(Value::string(
        params.hash(message.as_bytes()).as_bytes().to_vec(),
    ))
}

fn builtin_sodium_crypto_generichash_keygen(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !args.is_empty() {
        return Err(arity_error(
            "sodium_crypto_generichash_keygen",
            "zero arguments",
        ));
    }
    random_bytes(
        "sodium_crypto_generichash_keygen",
        SODIUM_CRYPTO_GENERICHASH_KEYBYTES,
    )
}

fn builtin_sodium_crypto_sign_verify_detached(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 3 {
        return Err(arity_error(
            "sodium_crypto_sign_verify_detached",
            "three arguments",
        ));
    }
    let signature = string_arg("sodium_crypto_sign_verify_detached", &args[0])?;
    let message = string_arg("sodium_crypto_sign_verify_detached", &args[1])?;
    let public_key = string_arg("sodium_crypto_sign_verify_detached", &args[2])?;
    if signature.len() != SODIUM_CRYPTO_SIGN_BYTES
        || public_key.len() != SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES
    {
        return Ok(Value::Bool(false));
    }
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key.as_bytes().try_into().unwrap())
    else {
        return Ok(Value::Bool(false));
    };
    let Ok(signature) = Signature::from_slice(signature.as_bytes()) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(
        verifying_key.verify(message.as_bytes(), &signature).is_ok(),
    ))
}

fn builtin_sodium_crypto_sign_detached(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 2 {
        return Err(arity_error("sodium_crypto_sign_detached", "two arguments"));
    }
    let message = string_arg("sodium_crypto_sign_detached", &args[0])?;
    let secret_key = string_arg("sodium_crypto_sign_detached", &args[1])?;
    if secret_key.len() != SODIUM_CRYPTO_SIGN_SECRETKEYBYTES {
        return Err(value_error(
            "sodium_crypto_sign_detached",
            "secret key must be 64 bytes",
        ));
    }
    let seed: &[u8; 32] = secret_key.as_bytes()[..32].try_into().unwrap();
    let signing_key = SigningKey::from_bytes(seed);
    Ok(Value::string(
        signing_key.sign(message.as_bytes()).to_bytes().to_vec(),
    ))
}

fn builtin_sodium_bin2hex(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("sodium_bin2hex", "one argument"));
    }
    let input = string_arg("sodium_bin2hex", &args[0])?;
    Ok(Value::string(hex_encode(input.as_bytes())))
}

fn builtin_sodium_hex2bin(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("sodium_hex2bin", "one or two arguments"));
    }
    let input = string_arg("sodium_hex2bin", &args[0])?;
    let ignore = args
        .get(1)
        .map(|value| string_arg("sodium_hex2bin", value))
        .transpose()?;
    let bytes = input
        .as_bytes()
        .iter()
        .copied()
        .filter(|byte| {
            ignore
                .as_ref()
                .is_some_and(|ignore| ignore.as_bytes().contains(byte))
                || !byte.is_ascii_whitespace()
        })
        .filter(|byte| {
            !ignore
                .as_ref()
                .is_some_and(|ignore| ignore.as_bytes().contains(byte))
        })
        .collect::<Vec<_>>();
    hex_decode(&bytes)
        .map(Value::string)
        .ok_or_else(|| value_error("sodium_hex2bin", "input must be hexadecimal"))
}

fn builtin_sodium_bin2base64(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 2 {
        return Err(arity_error("sodium_bin2base64", "two arguments"));
    }
    let input = string_arg("sodium_bin2base64", &args[0])?;
    let variant = int_arg("sodium_bin2base64", &args[1])?;
    Ok(Value::string(
        base64_engine(variant)?.encode(input.as_bytes()),
    ))
}

fn builtin_sodium_base642bin(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("sodium_base642bin", "two or three arguments"));
    }
    let input = string_arg("sodium_base642bin", &args[0])?;
    let variant = int_arg("sodium_base642bin", &args[1])?;
    base64_engine(variant)?
        .decode(input.as_bytes())
        .map(Value::string)
        .map_err(|_| value_error("sodium_base642bin", "input must be valid base64"))
}

fn base64_engine(variant: i64) -> Result<&'static general_purpose::GeneralPurpose, BuiltinError> {
    match variant {
        SODIUM_BASE64_VARIANT_ORIGINAL => Ok(&general_purpose::STANDARD),
        SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING => Ok(&general_purpose::STANDARD_NO_PAD),
        SODIUM_BASE64_VARIANT_URLSAFE => Ok(&general_purpose::URL_SAFE),
        SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING => Ok(&general_purpose::URL_SAFE_NO_PAD),
        _ => Err(value_error("sodium_base64", "unsupported base64 variant")),
    }
}

fn random_bytes(name: &str, length: usize) -> BuiltinResult {
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes)
        .map_err(|_| BuiltinError::new("E_PHP_RUNTIME_RANDOM", format!("{name} failed")))?;
    Ok(Value::string(bytes))
}

fn hex_encode(bytes: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize]);
        output.push(HEX[(byte & 0x0f) as usize]);
    }
    output
}

fn hex_decode(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        output.push(hex_value(pair[0])? << 4 | hex_value(pair[1])?);
    }
    Some(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn value_error(name: &str, message: impl Into<String>) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_BUILTIN_VALUE",
        format!("{name}(): {}", message.into()),
    )
}
