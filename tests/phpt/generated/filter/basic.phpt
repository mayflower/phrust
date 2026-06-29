--TEST--
filter validation and sanitization basics
--SKIPIF--
<?php if (!extension_loaded("filter")) die("skip filter extension not loaded"); ?>
--FILE--
<?php
echo filter_var("42", FILTER_VALIDATE_INT), "\n";
var_dump(filter_var("1.25", FILTER_VALIDATE_FLOAT));
var_dump(filter_var("x", FILTER_VALIDATE_INT));
echo filter_var("a<b>@example.com", FILTER_SANITIZE_EMAIL), "\n";
?>
--EXPECT--
42
float(1.25)
bool(false)
ab@example.com
