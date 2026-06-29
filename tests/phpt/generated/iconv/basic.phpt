--TEST--
iconv encoding state and Latin-1 basics
--SKIPIF--
<?php if (!extension_loaded("iconv")) die("skip iconv extension not loaded"); ?>
--FILE--
<?php
$utf8 = "caf\xC3\xA9";
echo bin2hex(iconv("UTF-8", "ISO-8859-1", $utf8)), "\n";
echo iconv_strlen($utf8, "UTF-8"), "\n";
var_dump(iconv_set_encoding("internal_encoding", "ISO-8859-1"));
echo iconv_get_encoding("internal_encoding"), "\n";
?>
--EXPECT--
636166e9
4
bool(true)
ISO-8859-1
