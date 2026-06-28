--TEST--
wp.stdlib: filter validation and sanitization helpers
--DESCRIPTION--
Generated filter coverage for common WordPress validation and sanitization
paths without SAPI-backed input globals.
--SKIPIF--
<?php
if (!extension_loaded("filter")) die("skip filter extension not available");
?>
--FILE--
<?php
var_dump(filter_var("user@example.com", FILTER_VALIDATE_EMAIL));
var_dump(filter_var("https://example.com/path?q=1", FILTER_VALIDATE_URL, FILTER_FLAG_PATH_REQUIRED | FILTER_FLAG_QUERY_REQUIRED));
var_dump(filter_var("127.0.0.1", FILTER_VALIDATE_IP, FILTER_FLAG_IPV4));
var_dump(filter_var("yes", FILTER_VALIDATE_BOOLEAN));
var_dump(filter_var("maybe", FILTER_VALIDATE_BOOLEAN, FILTER_NULL_ON_FAILURE));
var_dump(filter_var("bad <user>@example.com", FILTER_SANITIZE_EMAIL));
var_dump(filter_var("a1b-2c+3", FILTER_SANITIZE_NUMBER_INT));
var_dump(filter_input(INPUT_GET, "missing", FILTER_VALIDATE_EMAIL));
?>
--EXPECT--
string(16) "user@example.com"
string(28) "https://example.com/path?q=1"
string(9) "127.0.0.1"
bool(true)
NULL
string(19) "baduser@example.com"
string(5) "1-2+3"
NULL
