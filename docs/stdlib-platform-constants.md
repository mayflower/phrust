# Standard library Platform Constants

Reference target: PHP 8.5.7 (`php-8.5.7`).

`php_std::constants` owns Standard library core/platform constant metadata:

- `PHP_VERSION`, `PHP_VERSION_ID`, `PHP_MAJOR_VERSION`,
  `PHP_MINOR_VERSION`, `PHP_RELEASE_VERSION`
- `DIRECTORY_SEPARATOR`, `PATH_SEPARATOR`, `PHP_OS`, `PHP_OS_FAMILY`,
  `PHP_EOL`
- baseline error constants such as `E_ERROR`, `E_WARNING`, and `E_ALL`
- extension-specific stubs such as `JSON_ERROR_NONE` and `PREG_NO_ERROR`

Version values come from the Foundation reference target. Platform-dependent
values are derived from the Rust build target and normalized by the Standard library diff
harness when surfaced in PHP-visible tests.
