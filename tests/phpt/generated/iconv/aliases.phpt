--TEST--
iconv common ASCII and Latin-1 aliases
--SKIPIF--
<?php if (!extension_loaded("iconv")) die("skip iconv extension not loaded"); ?>
--FILE--
<?php
echo bin2hex(iconv("CP819", "UTF-8", "\xE4")), "\n";
echo bin2hex(iconv("ISO-IR-100", "UTF-8", "\xE4")), "\n";
var_dump(iconv("ANSI_X3.4-1968", "UTF-8", "abc"));
var_dump(iconv_strlen("abc", "CP367"));
?>
--EXPECT--
c3a4
c3a4
string(3) "abc"
int(3)
