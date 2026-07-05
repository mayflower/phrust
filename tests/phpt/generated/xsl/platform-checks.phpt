--TEST--
xsl: bounded platform facade
--DESCRIPTION--
Focused XML-family coverage for XSL platform visibility.
--SKIPIF--
<?php
if (basename(PHP_BINARY) !== "phrust-php") {
    die("skip phrust-only XSL facade fixture");
}
?>
--FILE--
<?php
var_dump(extension_loaded("xsl"));
var_dump(class_exists("XSLTProcessor", false));
var_dump(defined("XSL_CLONE_AUTO"));
var_dump(XSL_CLONE_AUTO);
var_dump(XSL_CLONE_NEVER);
var_dump(XSL_CLONE_ALWAYS);
var_dump(XSL_SECPREF_NONE);
var_dump(XSL_SECPREF_READ_FILE);
var_dump(XSL_SECPREF_WRITE_FILE);
var_dump(XSL_SECPREF_CREATE_DIRECTORY);
var_dump(XSL_SECPREF_READ_NETWORK);
var_dump(XSL_SECPREF_WRITE_NETWORK);
var_dump(XSL_SECPREF_DEFAULT);
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
int(0)
int(-1)
int(1)
int(0)
int(2)
int(4)
int(8)
int(16)
int(32)
int(44)
