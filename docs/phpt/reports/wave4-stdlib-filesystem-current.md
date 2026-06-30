# Wave 4 stdlib/filesystem PHPT promotion current report

Branch: `phpt/wave4-stdlib-filesystem-promotion`

Prompt pack: `~/Downloads/wave4_branch_2_stdlib_filesystem_promotion.md`

Reference oracle used for Prompt 2.1 inventory:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`
- `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src`
- PHP oracle: 8.5.7 CLI debug build

## Prompt 2.1 baseline gates

All required inventory gates were run with fresh target execution (`PHPT_REUSE_LAST=0`, `PHPT_DEV_REUSE_TARGET_PASS=0`).

| Module | Selected PHPTs before Prompt 2.1 promotion | Result |
| --- | ---: | --- |
| `standard.arrays` | 17 | PASS, 0 non-green |
| `standard.strings` | 16 | PASS, 0 non-green |
| `standard.variables` | 27 | PASS, 0 non-green |
| `standard.serialization` | 5 | PASS, 0 non-green |
| `filesystem.streams` | 11 | PASS, 0 non-green |
| `wp.stdlib` | 7 | PASS, 0 non-green |
| `wp.request-filesystem` | 7 | PASS, 0 non-green |

## First promoted upstream batch

Prompt 2.1 requires the first batch to promote 10-30 upstream PHPTs without behavior changes. The selected first batch promotes 14 real upstream php-src PHPT rows that already pass both the reference and target in focused temporary manifests.

### `standard.arrays`

| PHPT | Owner function(s) | Probe result |
| --- | --- | --- |
| `ext/standard/tests/array/array_fill.phpt` | `array_fill` | PASS reference, PASS target |
| `ext/standard/tests/array/array_key_first.phpt` | `array_key_first` | PASS reference, PASS target |
| `ext/standard/tests/array/array_key_last.phpt` | `array_key_last` | PASS reference, PASS target |
| `ext/standard/tests/array/array_pad.phpt` | `array_pad` | PASS reference, PASS target |
| `ext/standard/tests/array/array_pop.phpt` | `array_pop` | PASS reference, PASS target |
| `ext/standard/tests/array/array_push.phpt` | `array_push` | PASS reference, PASS target |
| `ext/standard/tests/array/array_unshift.phpt` | `array_unshift` | PASS reference, PASS target |

Selected manifest increased from 17 to 24 rows.

### `standard.strings`

| PHPT | Owner function(s) | Probe result |
| --- | --- | --- |
| `ext/standard/tests/strings/str_starts_with.phpt` | `str_starts_with` | PASS reference, PASS target |
| `ext/standard/tests/strings/str_ends_with.phpt` | `str_ends_with` | PASS reference, PASS target |
| `ext/standard/tests/strings/strrev.phpt` | `strrev` | PASS reference, PASS target |
| `ext/standard/tests/strings/strtolower.phpt` | `strtolower` | PASS reference, PASS target |
| `ext/standard/tests/strings/ltrim.phpt` | `ltrim` | PASS reference, PASS target |
| `ext/standard/tests/strings/rtrim.phpt` | `rtrim` | PASS reference, PASS target |
| `ext/standard/tests/strings/trim.phpt` | `trim` | PASS reference, PASS target |

Selected manifest increased from 16 to 23 rows. `ext/standard/tests/strings/str_contains.phpt` also passed in the probe, but it was already selected before this prompt and is not counted as newly promoted.

## Prompt 2.2 array promotion

Prompt 2.2 promoted 11 additional upstream array PHPTs after fixing PHP-visible `array_chunk` and `array_flip` behavior:

- `array_chunk(..., preserve_keys: false)` now rebuilds each chunk as a packed list, including chunks from string-keyed input.
- `array_flip` now flips only integer and string values; bool, null, arrays, objects, resources, and other unsupported values warn and are skipped.

Promoted upstream rows:

| PHPT | Owner function(s) | Result |
| --- | --- | --- |
| `ext/standard/tests/array/array_chunk_basic1.phpt` | `array_chunk` | PASS reference, PASS target |
| `ext/standard/tests/array/array_chunk_basic2.phpt` | `array_chunk` | PASS reference, PASS target |
| `ext/standard/tests/array/array_flip.phpt` | `array_flip` | PASS reference, PASS target |
| `ext/standard/tests/array/array_flip_basic.phpt` | `array_flip` | PASS reference, PASS target |
| `ext/standard/tests/array/array_flip_variation2.phpt` | `array_flip` | PASS reference, PASS target |
| `ext/standard/tests/array/array_flip_variation3.phpt` | `array_flip` | PASS reference, PASS target |
| `ext/standard/tests/array/array_flip_variation4.phpt` | `array_flip` | PASS reference, PASS target |
| `ext/standard/tests/array/array_flip_variation5.phpt` | `array_flip` | PASS reference, PASS target |
| `ext/standard/tests/array/array_diff_assoc.phpt` | `array_diff_assoc` | PASS reference, PASS target |
| `ext/standard/tests/array/array_intersect_basic.phpt` | `array_intersect` | PASS reference, PASS target |
| `ext/standard/tests/array/array_intersect_assoc_basic.phpt` | `array_intersect_assoc` | PASS reference, PASS target |

Selected `standard.arrays` coverage increased from 24 to 35 rows.

Prompt 2.2 probe non-promotions:

- `ext/standard/tests/array/array_diff.phpt` and `ext/standard/tests/array/array_replace_recursive.phpt`: BORK under the pinned PHP 8.5.7 reference because those exact paths do not exist.
- `ext/standard/tests/array/array_intersect_key.phpt`: target fatal, `array_intersect_key` is not registered.
- `ext/standard/tests/array/array_replace.phpt`: target lacks the upstream endless-recursion detection behavior.
- `ext/standard/tests/array/array_rand.phpt`: target returns internal value-error text instead of PHP-compatible argument messages.

## Prompt 2.3 string, URL, HTML, and formatting promotion

Prompt 2.3 promoted 14 additional upstream PHPTs after fixing `http_build_query` argument handling:

- `http_build_query` now applies the numeric prefix argument to top-level integer keys.
- `http_build_query` now honors the custom argument separator argument.

Promoted upstream rows:

| PHPT | Owner function(s) | Result |
| --- | --- | --- |
| `ext/standard/tests/http/http_build_query/http_build_query.phpt` | `http_build_query` | PASS reference, PASS target |
| `ext/standard/tests/strings/substr_replace.phpt` | `substr_replace` | PASS reference, PASS target |
| `ext/standard/tests/strings/sprintf_basic2.phpt` | `sprintf` | PASS reference, PASS target |
| `ext/standard/tests/strings/printf_basic2.phpt` | `printf` | PASS reference, PASS target |
| `ext/standard/tests/strings/wordwrap.phpt` | `wordwrap` | PASS reference, PASS target |
| `ext/standard/tests/strings/substr_replace_array.phpt` | `substr_replace` | PASS reference, PASS target |
| `ext/standard/tests/strings/sprintf_basic3.phpt` | `sprintf` | PASS reference, PASS target |
| `ext/standard/tests/strings/sprintf_basic4.phpt` | `sprintf` | PASS reference, PASS target |
| `ext/standard/tests/strings/sprintf_basic5.phpt` | `sprintf` | PASS reference, PASS target |
| `ext/standard/tests/strings/sprintf_basic6.phpt` | `sprintf` | PASS reference, PASS target |
| `ext/standard/tests/strings/vsprintf_basic1.phpt` | `vsprintf` | PASS reference, PASS target |
| `ext/standard/tests/strings/vsprintf_basic2.phpt` | `vsprintf` | PASS reference, PASS target |
| `ext/standard/tests/strings/wordwrap_basic.phpt` | `wordwrap` | PASS reference, PASS target |
| `ext/standard/tests/strings/wordwrap_error.phpt` | `wordwrap` | PASS reference, PASS target |

Selected `standard.strings` coverage increased from 23 to 37 rows.

Prompt 2.3 probe non-promotions:

- Stale/nonexistent php-src paths: `ext/standard/tests/http/parse_url_basic_001.phpt`, `ext/standard/tests/http/parse_url_basic_002.phpt`, `ext/standard/tests/http/parse_url_basic_003.phpt`, `ext/standard/tests/strings/html_entity_decode.phpt`, `ext/standard/tests/strings/str_replace.phpt`, `ext/standard/tests/strings/vsprintf_basic.phpt`.
- Reference skip: `ext/standard/tests/strings/htmlentities.phpt` is non-UTF8 and remains a runner malformed/non-UTF8 gap.
- `ext/standard/tests/http/http_build_query/http_build_query_object_just_stringable.phpt`: target lacks object `__toString` conversion in this builtin path.
- `ext/standard/tests/http/http_build_query/http_build_query_object_key_val_stringable.phpt`: target does not yet accept object input for query building.
- `ext/standard/tests/strings/htmlspecialchars_basic.phpt`: target still escapes quotes for cases whose flags leave double quotes unescaped.
- `ext/standard/tests/strings/str_ireplace.phpt`: target does not register `str_ireplace`.
- `ext/standard/tests/strings/str_replace_basic.phpt` and `ext/standard/tests/strings/str_replace_variation1.phpt`: target string-casts resource/array search values where PHP reports type or conversion diagnostics.
- `ext/standard/tests/strings/htmlspecialchars_decode_basic.phpt`: target decodes `&#039;` in a flags combination where PHP preserves it.

## Prompt 2.4 serialization and debug-output promotion

Prompt 2.4 promoted 24 additional upstream PHPTs without runtime behavior changes:

- 6 variable/debug-output rows for NUL-byte strings plus integer and array
  `var_dump`/`print_r` rendering.
- 18 serialization rows from the upstream serialize corpus that already pass
  the selected runtime surface.

### `standard.variables`

| PHPT | Owner function(s) | Result |
| --- | --- | --- |
| `ext/standard/tests/general_functions/var_dump_strings_nul_bytes.phpt` | `var_dump` | PASS reference, PASS target |
| `ext/standard/tests/general_functions/var_dump_ints.phpt` | `var_dump` | PASS reference, PASS target |
| `ext/standard/tests/general_functions/var_dump_arrays.phpt` | `var_dump` | PASS reference, PASS target |
| `ext/standard/tests/general_functions/print_r_strings_nul_bytes.phpt` | `print_r` | PASS reference, PASS target |
| `ext/standard/tests/general_functions/print_r_ints.phpt` | `print_r` | PASS reference, PASS target |
| `ext/standard/tests/general_functions/print_r_arrays.phpt` | `print_r` | PASS reference, PASS target |

Selected `standard.variables` coverage increased from 27 to 33 rows.

### `standard.serialization`

| PHPT | Owner function(s) | Result |
| --- | --- | --- |
| `ext/standard/tests/serialize/bug23298.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug24063.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug31442.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug37947.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug42919.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug43614.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug46882.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug55798.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug68594.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug74300.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/bug81142.phpt` | `serialize`, `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/serialization_precision_001.phpt` | `serialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/serialize_globals_var_refs.phpt` | `serialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/shm_corruption_coercion_unserialize_options.phpt` | `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/sleep_deref.phpt` | `serialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/unserializeS.phpt` | `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/unserialize_allowed_classes_option_stringable_value.phpt` | `unserialize` | PASS reference, PASS target |
| `ext/standard/tests/serialize/unserialize_neg_iv_edge_cases.phpt` | `unserialize` | PASS reference, PASS target |

Selected `standard.serialization` coverage increased from 5 to 23 rows.

Prompt 2.4 probe non-promotions:

- `ext/standard/tests/general_functions/is_int.phpt` and `ext/standard/tests/serialize/serialization_miscTypes_001.phpt`: 32-bit-only reference skips on this host.
- `ext/standard/tests/general_functions/get_debug_type_basic.phpt`: target still lacks anonymous class execution in this path.
- `ext/standard/tests/general_functions/gettype_settype_basic.phpt`: target does not register `settype`.
- `ext/standard/tests/serialize/001.phpt` and `ext/standard/tests/serialize/serialization_arrays_001.phpt`: target lacks array-dimension to array-dimension reference binding.
- `ext/standard/tests/serialize/003.phpt`: target float serialization formatting differs for selected exponent/precision cases.
- `ext/standard/tests/serialize/005.phpt`: target does not emit Serializable interface deprecation output in the expected order.
- `ext/standard/tests/serialize/serialization_objects_001.phpt`, `serialization_objects_004.phpt`, and `serialization_objects_incomplete.phpt`: target object visibility, object identity reference records, and incomplete-class behavior remain broader serialization gaps.
- `ext/standard/tests/serialize/unserialize_allowed_classes_option_invalid_array.phpt`, `unserialize_allowed_classes_option_invalid_value.phpt`, and `unserialize_allowed_classes_option_invalid_class_names.phpt`: target `allowed_classes` validation diagnostics remain outside the selected gate.

## Prompt 2.5 filesystem, stat, temp, glob, and streams promotion

Prompt 2.5 promoted 14 upstream filesystem/stream PHPTs without runtime behavior
changes. The selected rows cover local file reads/writes, `readfile`, `chmod`,
`glob`, `lstat`/`stat` variations, `tempnam`, `touch`, and
`stream_get_contents`.

Promoted upstream rows:

| PHPT | Owner function(s) | Result |
| --- | --- | --- |
| `ext/standard/tests/file/file_get_contents_basic.phpt` | `file_get_contents` | PASS reference, PASS target |
| `ext/standard/tests/file/file_get_contents_basic001.phpt` | `file_get_contents` | PASS reference, PASS target |
| `ext/standard/tests/file/file_get_contents_file_put_contents_basic.phpt` | `file_get_contents`, `file_put_contents` | PASS reference, PASS target |
| `ext/standard/tests/file/file_get_contents_variation7.phpt` | `file_get_contents` | PASS reference, PASS target |
| `ext/standard/tests/file/file_put_contents_variation1.phpt` | `file_put_contents` | PASS reference, PASS target |
| `ext/standard/tests/file/readfile_basic.phpt` | `readfile` | PASS reference, PASS target |
| `ext/standard/tests/file/readfile_variation9.phpt` | `readfile` | PASS reference, PASS target |
| `ext/standard/tests/file/chmod_variation1.phpt` | `chmod` | PASS reference, PASS target |
| `ext/standard/tests/file/glob_basic.phpt` | `glob` | PASS reference, PASS target |
| `ext/standard/tests/file/lstat_stat_variation1.phpt` | `lstat`, `stat` | PASS reference, PASS target |
| `ext/standard/tests/file/lstat_stat_variation2.phpt` | `lstat`, `stat` | PASS reference, PASS target |
| `ext/standard/tests/file/tempnam_variation5.phpt` | `tempnam` | PASS reference, PASS target |
| `ext/standard/tests/file/touch_variation2.phpt` | `touch` | PASS reference, PASS target |
| `ext/standard/tests/streams/stream_get_contents_001.phpt` | `stream_get_contents` | PASS reference, PASS target |

Selected `filesystem.streams` coverage increased from 11 to 25 rows.

Prompt 2.5 probe non-promotions:

- Stale/nonexistent named backlog paths in pinned php-src 8.5.7:
  `file_get_contents.phpt`, `file_put_contents_variation.phpt`,
  `readfile.phpt`, `stat.phpt`, `lstat.phpt`, `clearstatcache.phpt`,
  `tempnam.phpt`, `tmpfile.phpt`, `sys_get_temp_dir.phpt`, and the unnumbered
  stream paths from the initial backlog.
- `ext/standard/tests/file/chmod_basic.phpt`: reference itself fails on this
  host because the expected high mode bits do not match observed filesystem
  behavior.
- Broader target misses in the reference-pass probe include unsupported
  `symlink`, `link`, `sleep`, `fstat`, `fileinode`, `fileatime`,
  `get_included_files`, `set_include_path`, `stream_context_get_params`,
  `stream_context_set_options`, and socket constants; PHP-compatible warning
  text and extended argument validation remain outside the selected gate.
- `stream_get_meta_data_*` rows remain non-promoted because target metadata
  arrays still differ from PHP's exact shape for file streams.

## Prompt 2.6 extension-adjacent stdlib promotion

Prompt 2.6 promoted 9 upstream PHPTs without runtime behavior changes across
the extension-adjacent selected surfaces. The default PHP oracle lacks several
extension builds for the selected module harness, so the module gates are
recorded as green target passes with reference skips where the reference binary
reports the required extension is not loaded.

| Module | PHPT | Owner function(s) | Result |
| --- | --- | --- | --- |
| `zlib` | `ext/zlib/tests/gzcompress_basic1.phpt` | `gzcompress` | SKIP reference, PASS target |
| `zlib` | `ext/zlib/tests/gzdeflate_basic1.phpt` | `gzdeflate` | SKIP reference, PASS target |
| `zlib` | `ext/zlib/tests/gzdeflate_variation1.phpt` | `gzdeflate` | SKIP reference, PASS target |
| `zlib` | `ext/zlib/tests/gzencode_basic1.phpt` | `gzencode` | SKIP reference, PASS target |
| `zlib` | `ext/zlib/tests/gzuncompress_basic1.phpt` | `gzuncompress` | SKIP reference, PASS target |
| `zip` | `ext/zip/tests/oo_extract.phpt` | `ZipArchive::extractTo` | SKIP reference, PASS target |
| `exif` | `ext/exif/tests/exif_imagetype_basic.phpt` | `exif_imagetype` | SKIP reference, PASS target |
| `exif` | `ext/exif/tests/exif_imagetype_error.phpt` | `exif_imagetype` | SKIP reference, PASS target |
| `iconv` | `ext/iconv/tests/iconv_strlen_basic.phpt` | `iconv_strlen` | SKIP reference, PASS target |

Selected coverage increased as follows:

| Module | Before | After |
| --- | ---: | ---: |
| `zlib` | 1 | 6 |
| `zip` | 1 | 2 |
| `exif` | 1 | 3 |
| `iconv` | 1 | 2 |

Prompt 2.6 probe non-promotions:

- Stale/nonexistent named backlog paths in pinned php-src 8.5.7:
  `ext/zlib/tests/gzcompress_basic.phpt`,
  `ext/zlib/tests/gzdeflate_basic.phpt`,
  `ext/zlib/tests/gzinflate_basic.phpt`,
  `ext/zlib/tests/zlib_decode.phpt`, and
  `ext/exif/tests/exif_imagetype.phpt`.
- `ext/zlib/tests/gzencode_variation1.phpt`: reference skip on Darwin because
  the tested gzip OS header is non-Darwin-specific.
- `ext/zlib/tests/gzinflate_length.phpt`: target still lacks the exact
  insufficient-memory warning/output behavior.
- `ext/zip/tests/oo_open.phpt`: target lacks the complete `ZipArchive::CREATE`
  mutation surface and constant parity.
- `ext/fileinfo/tests/finfo_buffer_basic.phpt` and
  `ext/fileinfo/tests/finfo_file_basic.phpt`: target does not provide full
  libmagic description parity or `FILEINFO_CONTINUE`.
- `ext/filter/tests/001.phpt` and `ext/filter/tests/002.phpt`: selected target
  filter outputs do not yet match upstream expectations.
- `ext/hash/tests/hash_hmac_basic.phpt`: target hash registry remains narrower
  than upstream algorithms.
- `ext/hash/tests/hash_equals.phpt`: target scalar/null coercion and type
  diagnostics still differ from PHP.
- Upstream mbstring rows were not promoted because the required mbstring-enabled
  reference binary `/tmp/php-src-mbstring-oracle/sapi/cli/php` is not present;
  the default project oracle is documented as built without mbstring.

Prompt 2.6 verification:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=zlib`: PASS, 6 reference SKIP, 6 target PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=zip`: PASS, 2 reference SKIP, 2 target PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=exif`: PASS, 3 reference SKIP, 3 target PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=iconv`: PASS, 2 reference SKIP, 2 target PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=fileinfo`: PASS, 1 reference SKIP, 1 target PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=filter`: PASS, 1 reference SKIP, 1 target PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.stdlib`: PASS, 2 reference PASS / 5 SKIP, 7 target PASS
- `REFERENCE_PHP=/tmp/php-src-mbstring-oracle/sapi/cli/php ... nix develop -c just phpt-dev-module MODULE=mbstring`: SKIPPED, `/tmp/php-src-mbstring-oracle/sapi/cli/php` is not executable on this host.

## Prompt 2.7 closeout summary

This branch promoted 86 upstream php-src PHPT rows into selected module gates:

- Prompt 2.1: 14 upstream rows.
- Prompt 2.2: 11 upstream rows.
- Prompt 2.3: 14 upstream rows.
- Prompt 2.4: 24 upstream rows.
- Prompt 2.5: 14 upstream rows.
- Prompt 2.6: 9 upstream rows.

Selected module before/after outcomes:

| Module | Before branch | After branch |
| --- | ---: | ---: |
| `standard.arrays` | 17 PASS | 35 PASS |
| `standard.strings` | 16 PASS | 37 PASS |
| `standard.variables` | 27 PASS | 32 PASS / 1 SKIP |
| `standard.serialization` | 5 PASS | 23 PASS |
| `filesystem.streams` | 11 PASS | 25 PASS |
| `wp.stdlib` | 7 selected green | 2 reference PASS / 5 reference SKIP; 7 target PASS |
| `wp.request-filesystem` | 7 selected green | 4 reference PASS / 3 reference SKIP; 6 target PASS / 1 target SKIP |
| `zlib` | 1 generated target PASS | 6 target PASS; reference SKIP because default oracle lacks zlib |
| `zip` | 1 generated target PASS | 2 target PASS; reference SKIP because default oracle lacks zip |
| `fileinfo` | 1 generated target PASS | unchanged, 1 target PASS; reference SKIP because default oracle lacks fileinfo |
| `exif` | 1 generated target PASS | 3 target PASS; reference SKIP because default oracle lacks exif |
| `filter` | 1 generated target PASS | unchanged, 1 target PASS; reference SKIP because default oracle lacks filter |
| `iconv` | 1 generated target PASS | 2 target PASS; reference SKIP because default oracle lacks iconv |
| `mbstring` | 3 generated fixtures | unchanged; upstream promotion skipped because `/tmp/php-src-mbstring-oracle/sapi/cli/php` is unavailable |

Remaining gaps are captured in the prompt-specific non-promotion notes above
and in the candidate backlog below. The next best promotion candidates are the
near-miss rows in the arrays/callback, string query/HTML, serialization object
matrix, stream metadata/context, and extension-adjacent sections.

## Candidate backlog by category

The following upstream PHPTs remain close candidates for Prompts 2.3-2.6 and later array follow-up. They were selected from implemented builtin surfaces, existing full-known-failure ownership, and Prompt 2.1/2.2 probes. Items marked near miss already have fresh probe evidence and should be prioritized.

### Arrays/callback/sort

- `ext/standard/tests/array/array_combine.phpt` - near miss: parser/frontend currently rejects selected source syntax.
- `ext/standard/tests/array/array_change_key_case.phpt`
- `ext/standard/tests/array/array_change_key_case_variation.phpt`
- `ext/standard/tests/array/array_intersect_key.phpt` - near miss: builtin registration gap.
- `ext/standard/tests/array/array_replace.phpt` - near miss: recursion detection mismatch.
- `ext/standard/tests/array/array_reverse.phpt`
- `ext/standard/tests/array/array_rand.phpt` - near miss: PHP-compatible value-error messages.
- `ext/standard/tests/array/array_reduce.phpt`
- `ext/standard/tests/array/array_map.phpt`
- `ext/standard/tests/array/array_filter.phpt`
- `ext/standard/tests/array/array_walk.phpt`
- `ext/standard/tests/array/usort.phpt`

### String/URL/HTML/query/formatting

- `ext/standard/tests/http/http_build_query/http_build_query_object_just_stringable.phpt` - near miss: object `__toString` conversion gap.
- `ext/standard/tests/http/http_build_query/http_build_query_object_key_val_stringable.phpt` - near miss: object input gap.
- `ext/standard/tests/http/parse_url_basic_001.phpt`
- `ext/standard/tests/http/parse_url_basic_002.phpt`
- `ext/standard/tests/http/parse_url_basic_003.phpt`
- `ext/standard/tests/strings/htmlspecialchars_basic.phpt` - near miss: quote flag handling mismatch.
- `ext/standard/tests/strings/htmlentities.phpt`
- `ext/standard/tests/strings/html_entity_decode.phpt`
- `ext/standard/tests/strings/str_replace.phpt`
- `ext/standard/tests/strings/str_ireplace.phpt`
- `ext/standard/tests/strings/vsprintf_basic.phpt`

### Variables/debug output

- `ext/standard/tests/general_functions/print_r_strings.phpt`
- `ext/standard/tests/general_functions/print_r_strings_nul_bytes.phpt`
- `ext/standard/tests/general_functions/var_dump_strings.phpt`
- `ext/standard/tests/general_functions/var_dump_strings_nul_bytes.phpt`
- `ext/standard/tests/general_functions/get_debug_type.phpt`
- `ext/standard/tests/general_functions/gettype.phpt`
- `ext/standard/tests/general_functions/is_array.phpt`
- `ext/standard/tests/general_functions/is_bool.phpt`
- `ext/standard/tests/general_functions/is_int.phpt`
- `ext/standard/tests/general_functions/is_string.phpt`

### Serialization/unserialization

- `ext/standard/tests/serialize/serialize.phpt`
- `ext/standard/tests/serialize/serialization_objects_001.phpt`
- `ext/standard/tests/serialize/serialization_arrays_001.phpt`
- `ext/standard/tests/serialize/unserialize_basic.phpt`
- `ext/standard/tests/serialize/unserialize_allowed_classes.phpt`
- `ext/standard/tests/serialize/unserialize_callback_func.phpt`

### Filesystem/stat/temp/glob/permissions

- `ext/standard/tests/file/file_get_contents.phpt`
- `ext/standard/tests/file/file_put_contents_variation.phpt`
- `ext/standard/tests/file/readfile.phpt`
- `ext/standard/tests/file/stat.phpt`
- `ext/standard/tests/file/lstat.phpt`
- `ext/standard/tests/file/clearstatcache.phpt`
- `ext/standard/tests/file/touch.phpt`
- `ext/standard/tests/file/chmod_basic.phpt`
- `ext/standard/tests/file/glob_basic.phpt`
- `ext/standard/tests/file/tempnam.phpt`
- `ext/standard/tests/file/tmpfile.phpt`
- `ext/standard/tests/file/sys_get_temp_dir.phpt`

### Streams/context/metadata/wrapper

- `ext/standard/tests/streams/stream_get_meta_data.phpt`
- `ext/standard/tests/streams/stream_context_create.phpt`
- `ext/standard/tests/streams/stream_context_get_options.phpt`
- `ext/standard/tests/streams/stream_context_set_option.phpt`
- `ext/standard/tests/streams/stream_get_contents.phpt`
- `ext/standard/tests/streams/fseek.phpt`

### Zlib/zip/fileinfo/exif/filter/hash/iconv/mbstring

- `ext/zlib/tests/gzencode_variation1.phpt`
- `ext/zlib/tests/gzcompress_basic.phpt`
- `ext/zlib/tests/gzuncompress_basic1.phpt`
- `ext/zlib/tests/gzdeflate_basic.phpt`
- `ext/zlib/tests/gzinflate_basic.phpt`
- `ext/zlib/tests/zlib_decode.phpt`
- `ext/zip/tests/oo_open.phpt`
- `ext/zip/tests/oo_extract.phpt`
- `ext/fileinfo/tests/finfo_buffer_basic.phpt`
- `ext/fileinfo/tests/finfo_file_basic.phpt`
- `ext/exif/tests/exif_imagetype.phpt`
- `ext/filter/tests/001.phpt`
- `ext/filter/tests/002.phpt`
- `ext/hash/tests/hash_hmac_basic.phpt`
- `ext/hash/tests/hash_equals.phpt`
- `ext/iconv/tests/iconv_strlen_basic.phpt`
- `ext/mbstring/tests/mb_strlen.phpt`

## Prompt 2.1 probe notes

Rejected candidates from the first probe:

- Stale/nonexistent php-src paths: `ext/standard/tests/array/array_is_list.phpt`, `ext/standard/tests/array/array_reverse.phpt`, `ext/standard/tests/array/array_shift.phpt`, `ext/standard/tests/strings/strtoupper.phpt`, `ext/standard/tests/http/parse_url_basic.phpt`, `ext/standard/tests/http/parse_str_basic.phpt`.
- Reference skip: `ext/standard/tests/strings/strlen.phpt` is non-UTF8 and remains a runner malformed/non-UTF8 gap.

## Verification log

Prompt 2.1 required gates before promotion:

- `nix develop -c just phpt-dev-module MODULE=standard.arrays`: PASS, 17 selected, 0 non-green.
- `nix develop -c just phpt-dev-module MODULE=standard.strings`: PASS, 16 selected, 0 non-green.
- `nix develop -c just phpt-dev-module MODULE=standard.variables`: PASS, 27 selected, 0 non-green.
- `nix develop -c just phpt-dev-module MODULE=standard.serialization`: PASS, 5 selected, 0 non-green.
- `nix develop -c just phpt-dev-module MODULE=filesystem.streams`: PASS, 11 selected, 0 non-green.
- `nix develop -c just phpt-dev-module MODULE=wp.stdlib`: PASS, 7 selected, 0 non-green.
- `nix develop -c just phpt-dev-module MODULE=wp.request-filesystem`: PASS, 7 selected, 0 non-green.

Post-Prompt 2.1 promotion gates:

- `nix develop -c just phpt-dev-module MODULE=standard.arrays`: PASS, 24 selected, reference 24 PASS, target 24 PASS.
- `nix develop -c just phpt-dev-module MODULE=standard.strings`: PASS, 23 selected, reference 23 PASS, target 23 PASS.
- `nix develop -c just verify-phpt`: PASS; known-gap manifest validated 85 entries, PHPT foundation passed, `php_phpt_tools` built and tested, PHPT baseline verified 21,548 corpus entries with 20,428 known non-green fingerprints.

All post-promotion PHPT gates used `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src`; source integrity reported 24,475 entries and 0 skipped host-generated entries.

Prompt 2.2 acceptance gates:

- `nix develop -c cargo fmt --check`: PASS.
- `nix develop -c cargo test -p php_runtime`: PASS, 262 tests.
- `nix develop -c cargo test -p php_vm`: PASS, 483 tests.
- `nix develop -c just phpt-dev-module MODULE=standard.arrays`: PASS, 35 selected, reference 35 PASS, target 35 PASS.

Prompt 2.2 gates used `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php` and `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src`; source integrity reported 24,475 entries and 0 skipped host-generated entries.

Prompt 2.3 acceptance gates:

- `nix develop -c cargo fmt --check`: PASS.
- `nix develop -c cargo test -p php_runtime`: PASS, 262 tests.
- `nix develop -c cargo test -p php_vm`: PASS, 483 tests.
- `nix develop -c just phpt-dev-module MODULE=standard.strings`: PASS, 37 selected, reference 37 PASS, target 37 PASS.

Prompt 2.3 gates used `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php` and `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src`; source integrity reported 24,475 entries and 0 skipped host-generated entries.

Prompt 2.4 acceptance gates:

- `nix develop -c cargo test -p php_runtime serialization`: PASS, 6 tests.
- `nix develop -c just phpt-dev-module MODULE=standard.variables`: PASS, 33 selected, reference 32 PASS / 1 SKIP, target 32 PASS / 1 SKIP.
- `nix develop -c just phpt-dev-module MODULE=standard.serialization`: PASS, 23 selected, reference 23 PASS, target 23 PASS.

Prompt 2.4 gates used `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php` and `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src`; source integrity reported 24,475 entries and 0 skipped host-generated entries.

Prompt 2.5 acceptance gates:

- `nix develop -c cargo test -p php_runtime resource`: PASS, 7 tests.
- `nix develop -c cargo test -p php_vm include`: PASS, 31 tests.
- `nix develop -c just phpt-dev-module MODULE=filesystem.streams`: PASS, 25 selected, reference 25 PASS, target 25 PASS.
- `nix develop -c just phpt-dev-module MODULE=wp.request-filesystem`: PASS, 7 selected, reference 4 PASS / 3 SKIP, target 6 PASS / 1 SKIP, no non-green target outcomes.

Prompt 2.5 gates used `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php` and `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src`; source integrity reported 24,475 entries and 0 skipped host-generated entries. Before rerunning the 2.5 gates, stale PHPT work artifacts under `/private/tmp/phrust-phpt-work` were removed because Nix could not write to the full Data volume.

Prompt 2.6 acceptance gates:

- `nix develop -c cargo test -p php_runtime`: PASS, 262 tests.
- `nix develop -c just phpt-dev-module MODULE=zlib`: PASS, 6 selected, reference 6 SKIP, target 6 PASS.
- `nix develop -c just phpt-dev-module MODULE=zip`: PASS, 2 selected, reference 2 SKIP, target 2 PASS.
- `nix develop -c just phpt-dev-module MODULE=exif`: PASS, 3 selected, reference 3 SKIP, target 3 PASS.
- `nix develop -c just phpt-dev-module MODULE=iconv`: PASS, 2 selected, reference 2 SKIP, target 2 PASS.
- `nix develop -c just phpt-dev-module MODULE=fileinfo`: PASS, 1 selected, reference 1 SKIP, target 1 PASS.
- `nix develop -c just phpt-dev-module MODULE=filter`: PASS, 1 selected, reference 1 SKIP, target 1 PASS.
- `nix develop -c just phpt-dev-module MODULE=wp.stdlib`: PASS, 7 selected, reference 2 PASS / 5 SKIP, target 7 PASS.
- `nix develop -c just phpt-dev-module MODULE=mbstring`: SKIPPED, `/tmp/php-src-mbstring-oracle/sapi/cli/php` is not executable on this host and the default project oracle is built without mbstring.

Prompt 2.6 gates used `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php` and `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src` except the skipped mbstring gate, which requires the documented mbstring-enabled oracle. Source integrity reported 24,475 entries and 0 skipped host-generated entries on all executed PHPT module gates.

Prompt 2.7 closeout gates:

- `nix develop -c cargo fmt --check`: PASS.
- `nix develop -c just verify-runtime`: PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`: PASS; stdlib diff total 43, pass 37, fail 0, skip 0, known_gap 6; streams diff total 2, pass 2, fail 0, skip 0; json-pcre-date diff total 3, pass 3, fail 0, skip 0; spl-reflection diff total 2, pass 2, fail 0, skip 0.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just verify-phpt`: PASS; known-gap manifest validated 85 entries, PHPT foundation passed, baseline verified 21,548 corpus entries with 20,428 known non-green fingerprints, source integrity verified 24,475 entries with 0 skipped host-generated entries, and `php_phpt_tools` tests passed.

Required Prompt 2.7 module gates:

- `standard.arrays`: PASS, 35 selected, reference 35 PASS, target 35 PASS.
- `standard.strings`: PASS, 37 selected, reference 37 PASS, target 37 PASS.
- `standard.variables`: PASS, 33 selected, reference 32 PASS / 1 SKIP, target 32 PASS / 1 SKIP.
- `standard.serialization`: PASS, 23 selected, reference 23 PASS, target 23 PASS.
- `filesystem.streams`: PASS, 25 selected, reference 25 PASS, target 25 PASS.
- `wp.stdlib`: PASS, 7 selected, reference 2 PASS / 5 SKIP, target 7 PASS.
- `wp.request-filesystem`: PASS, 7 selected, reference 4 PASS / 3 SKIP, target 6 PASS / 1 SKIP, no non-green target outcomes.
