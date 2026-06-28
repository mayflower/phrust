--TEST--
wp.stdlib: mbstring and iconv UTF-8 basics
--DESCRIPTION--
Generated text encoding coverage for bounded UTF-8/ASCII mbstring and iconv
helpers used by WordPress fallback paths.
--SKIPIF--
<?php
if (!extension_loaded("mbstring")) die("skip mbstring extension not available");
if (!extension_loaded("iconv")) die("skip iconv extension not available");
?>
--FILE--
<?php
$text = "Aé日";
var_dump(mb_strlen($text, "UTF-8"));
var_dump(mb_strpos($text, "日", 0, "UTF-8"));
var_dump(mb_substr($text, 1, 2, "UTF-8"));
var_dump(iconv("UTF-8", "UTF-8", $text));
var_dump(iconv_strlen($text, "UTF-8"));
var_dump(iconv_strpos($text, "é", 0, "UTF-8"));
var_dump(iconv_substr($text, -2, 1, "UTF-8"));
?>
--EXPECT--
int(3)
int(2)
string(5) "é日"
string(6) "Aé日"
int(3)
int(1)
string(2) "é"
