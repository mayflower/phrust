--TEST--
wp.stdlib: zlib gzip and deflate helpers
--DESCRIPTION--
Generated zlib coverage for gzip and zlib payload roundtrips.
--SKIPIF--
<?php
if (!extension_loaded("zlib")) die("skip zlib extension not available");
?>
--FILE--
<?php
$payload = "wordpress update package";
$gzip = gzencode($payload);
$zlib = gzcompress($payload);
var_dump(gzdecode($gzip));
var_dump(gzuncompress($zlib));
var_dump(zlib_decode($gzip));
var_dump(zlib_decode($zlib));
?>
--EXPECT--
string(24) "wordpress update package"
string(24) "wordpress update package"
string(24) "wordpress update package"
string(24) "wordpress update package"
