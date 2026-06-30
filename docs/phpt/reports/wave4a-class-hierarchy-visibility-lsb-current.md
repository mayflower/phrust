# Wave 4A Class Hierarchy Visibility LSB Current State

## Scope

- Branch: `wave4a-class-hierarchy-visibility-lsb`
- Reference target: PHP `8.5.7`
- Reference binary used: `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`
- PHP source tree used for integrity checks: `/Volumes/CrucialMusic/src/phrust/third_party/php-src`

The branch-local `third_party/php-src/sapi/cli/php` binary is not available in
this checkout, so the sibling pinned php-src oracle above was used for the
comparison run.

## PHPT Inventory

Command:

```bash
REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php \
PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src \
PHPT_REUSE_LAST=0 \
PHPT_DEV_REUSE_TARGET_PASS=0 \
nix develop -c just phpt-dev-module MODULE=objects.classes
```

| Run | Reference | Target | Source integrity | Result |
| --- | ---: | ---: | ---: | --- |
| Branch baseline | 246 pass, 0 non-green | 244 pass, 2 fail | 24475 checked, 0 skipped | red |
| Current | 246 pass, 0 non-green | 246 pass, 0 non-green | 24475 checked, 0 skipped | green |

## Moved Rows

The current branch moved these selected upstream PHPTs from target FAIL to PASS:

- `tests/classes/autoload_021.phpt`
- `tests/classes/constants_error_003.phpt`

## Implemented Behavior

- Invalid dynamic class names such as `../BUG` no longer invoke SPL autoload
  callbacks before the final `Class "../BUG" not found` fatal.
- Userland by-reference argument failures that reject a non-referenceable class
  constant are rendered from the caller site without adding a synthetic callee
  frame to the uncaught stack trace.
- Internal by-reference builtin fatals now use the call-site source span when
  rendering the fatal location.

## Remaining Gaps

The selected `objects.classes` module is green for this branch. No new
`objects.classes` known-gap entries or PHPT promotions were added.
