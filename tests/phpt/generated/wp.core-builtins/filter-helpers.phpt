--TEST--
wp.core-builtins: filter helpers
--DESCRIPTION--
Generated filter coverage for WordPress-style validation when the reference
binary has the filter extension available.
--SKIPIF--
<?php
if (!extension_loaded("filter")) die("skip filter extension not available");
?>
--GET--
email=user%40example.com&enabled=yes
--POST--
count=42
--COOKIE--
sid=abc123
--FILE--
<?php
var_dump(filter_var("user@example.com", FILTER_VALIDATE_EMAIL));
var_dump(filter_var("https://example.com/path?q=1", FILTER_VALIDATE_URL, FILTER_FLAG_PATH_REQUIRED | FILTER_FLAG_QUERY_REQUIRED));
var_dump(filter_var("maybe", FILTER_VALIDATE_BOOLEAN, FILTER_NULL_ON_FAILURE));
var_dump(filter_input(INPUT_GET, "email", FILTER_VALIDATE_EMAIL));
var_dump(filter_input(INPUT_GET, "enabled", FILTER_VALIDATE_BOOLEAN));
var_dump(filter_input(INPUT_POST, "count", FILTER_VALIDATE_INT));
var_dump(filter_input(INPUT_COOKIE, "sid"));
var_dump(filter_input(INPUT_SERVER, "REQUEST_METHOD"));
?>
--EXPECT--
string(16) "user@example.com"
string(28) "https://example.com/path?q=1"
NULL
string(16) "user@example.com"
bool(true)
int(42)
string(6) "abc123"
string(4) "POST"
