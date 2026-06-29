--TEST--
sodium real crypto and encoding basics
--SKIPIF--
<?php if (!extension_loaded("sodium")) die("skip sodium extension not loaded"); ?>
--FILE--
<?php
echo sodium_bin2hex(sodium_crypto_generichash("abc", "", 32)), "\n";
echo sodium_bin2base64("abc", SODIUM_BASE64_VARIANT_ORIGINAL), "\n";
var_dump(sodium_hex2bin("616263") === "abc");
?>
--EXPECT--
bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319
YWJj
bool(true)
