--TEST--
iconv_substr returns empty string when requested range is empty
--SKIPIF--
<?php if (!extension_loaded("iconv")) die("skip iconv extension not loaded"); ?>
--FILE--
<?php
var_dump(iconv_substr("foo", 2, -2, "UTF-8"));
var_dump(iconv_substr("abc", 5, 1, "UTF-8"));
?>
--EXPECT--
string(0) ""
string(0) ""
