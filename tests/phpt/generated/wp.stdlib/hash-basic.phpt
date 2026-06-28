--TEST--
wp.stdlib: hash helpers
--DESCRIPTION--
Generated hash coverage for WordPress integrity and timing-safe comparison
helpers.
--FILE--
<?php
echo hash("sha256", "abc"), "\n";
echo bin2hex(hash("md5", "abc", true)), "\n";
echo hash_hmac("sha256", "data", "key"), "\n";
var_dump(hash_equals("same", "same"));
var_dump(hash_equals("same", "diff"));
var_dump(in_array("sha512", hash_algos(), true));
?>
--EXPECT--
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
900150983cd24fb0d6963f7d28e17f72
5031fe3d989c6d1537a013fa6e739da23463fdaec3b70137d828e36ace221bd0
bool(true)
bool(false)
bool(true)
