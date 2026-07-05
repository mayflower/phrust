# calendar

- Strategy: php-src serial day number algorithm slice
- Selected manifest: `tests/phpt/manifests/modules/calendar.selected.jsonl`
- Selected fixtures:
  - `tests/phpt/generated/calendar/gregorian-julian.phpt`
  - `tests/phpt/generated/calendar/info-easter-unix.phpt`

## Implemented Surface

The runtime exposes calendar constants and the procedural calendar functions
from ext/calendar.

This slice implements php-src-derived Gregorian, Julian, Jewish, and French
republican serial day number conversions, `cal_days_in_month`, `cal_to_jd`,
`cal_from_jd`, `jdtogregorian`, `gregoriantojd`, `jdtojulian`,
`juliantojd`, `jdtojewish`, `jewishtojd`, `jdtofrench`, `frenchtojd`,
`jddayofweek`, `jdmonthname`, `jdtounix`, `unixtojd`, `cal_info`, and
`easter_days`.

`easter_date` is deterministic and uses midnight UTC-style arithmetic rather
than host-local timezone state.

## Gaps

`jdtojewish()` Hebrew-letter formatting flags are registered for reflection
and introspection but still return a stable known-gap diagnostic. The default
numeric Jewish date format is implemented.

The pinned php-src CLI used by this workspace does not load ext/calendar, so
the selected PHPT rows may skip on the reference side while still exercising
the phrust target.

## Target Gates

- `nix develop -c cargo test -p php_runtime calendar`
- `nix develop -c cargo test -p php_std calendar`
- `nix develop -c just phpt-dev-module MODULE=calendar`
