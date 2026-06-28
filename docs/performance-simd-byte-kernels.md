# SIMD and Byte-Kernel Facade

Date: 2026-06-28.

`php_source::byte_kernel` provides the first v2 byte-kernel facade for
source/text scanning. The API is safe, byte-oriented, and intentionally small:

- find one byte;
- find any of two or three bytes;
- count PHP source line breaks (`\n`, `\r\n`, and standalone `\r`);
- detect all-ASCII byte slices;
- classify ASCII identifier-continuation chunks;
- ASCII-only lowercase and uppercase copy/in-place helpers.

The search helpers use `memchr` behind the facade. Every optimized helper has a
scalar reference function and property-style tests that cover empty inputs,
single-byte inputs, vector-width-adjacent lengths, large inputs, all byte values,
delimiter positions, and invalid UTF-8 byte sequences.

## Policy

The public API exposes no unsafe functions. Any future architecture-specific
implementation must remain behind the same safe facade and keep scalar reference
parity tests.

The facade does not change PHP-visible behavior by itself. Lexer, source-map,
runtime string, and output call sites must opt in through later focused prompts
with token/span/diagnostic or runtime parity evidence.

SIMD and byte kernels accelerate byte-heavy loops. They do not replace VM
semantic optimization, interpreter feedback, inline caches, superinstructions,
or PHP runtime helpers.

## FPE-03 Integration

The first call-site integration uses the facade in source and lexer code only:

- `LineIndex` jumps between LF/CR bytes with `find_any2` while preserving the
  existing CRLF-as-one-line rule.
- The lexer cursor now supports safe byte-count advancement after line
  accounting, avoiding per-byte cursor bumps for bulk spans.
- Inline HTML skips to the next `<` byte before rechecking PHP open-tag shapes.
- Line comments jump to the next newline or `?` byte, then preserve the existing
  `?>` close-tag stop condition.
- Block comments jump to the next `*` byte before checking for `*/`.
- Identifier consumers use `ascii_identifier_continue_chunk_len` for ASCII runs
  and keep the previous byte-by-byte handling for PHP non-ASCII identifier
  bytes.
- Constant single- and double-quoted string scanning uses byte-kernel delimiter
  search while preserving escape and interpolation handling.

Skipped loops remain intentionally conservative where stop conditions depend on
interpolation state, heredoc indentation/labels, numeric literal separators, or
PHP whitespace/cast grammar. Those loops should move only with focused parity
tests and benchmark evidence.

Current benchmark support for this layer is the local advisory
`just bench-lexer` Criterion-style throughput smoke. It is not a compatibility
gate and should not be used for standalone speed claims.
