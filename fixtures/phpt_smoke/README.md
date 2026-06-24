# runtime PHPT Smoke Fixtures

These fixtures are small, local `.phpt` files for the runtime smoke runner.
They are not copied from PHP's upstream test suite and are not intended to
exercise the full `run-tests.php` feature set.

`just phpt-smoke` parses the supported sections, writes each `--FILE--` body to
`target/runtime/phpt-smoke/generated/`, runs it through `php-vm`, and compares
stdout with `--EXPECT--` or the small `--EXPECTF--` subset implemented by
`php_testkit::phpt`.

Supported sections:

| Section | Behavior |
| --- | --- |
| `--TEST--` | Optional display name |
| `--FILE--` | Required PHP source body |
| `--EXPECT--` | Exact expected stdout after one trailing newline is trimmed |
| `--EXPECTF--` | Pattern expected stdout with only `%%`, `%s`, `%S`, `%d`, `%i`, and `%w` |
| `--SKIPIF--` | Fixture is skipped; the skip PHP is not executed |
| `--INI--` | Fixture is reported as a known gap |

Unsupported sections such as `--ARGS--`, `--ENV--`, `--EXTENSIONS--`,
`--CLEAN--`, `--POST--`, `--GET--`, `--EXPECTREGEX--`, and `--EXPECTHEADERS--`
are skipped rather than interpreted. Developers may point the runner at a local
PHP checkout test with `run-phpt-smoke --extra-phpt path/to/test.phpt`, but CI
uses only this directory.
