--TEST--
zlib: gzip, zlib, and raw deflate roundtrips
--DESCRIPTION--
Generated compression coverage for WordPress archive and HTTP payload helpers.
--SKIPIF--
<?php
if (!extension_loaded("zlib")) die("skip zlib extension not available");
?>
--FILE--
<?php
$payload = "media archive payload";
var_dump(gzdecode(gzencode($payload, 1)));
var_dump(gzuncompress(gzcompress($payload, 1)));
var_dump(gzinflate(gzdeflate($payload, 1)));
var_dump(zlib_decode(zlib_encode($payload, ZLIB_ENCODING_GZIP, 1)));
var_dump(zlib_decode(zlib_encode($payload, ZLIB_ENCODING_DEFLATE, 1)));
var_dump(zlib_decode(zlib_encode($payload, ZLIB_ENCODING_RAW, 1)));
var_dump(gzinflate("not deflate"));
?>
--EXPECT--
string(21) "media archive payload"
string(21) "media archive payload"
string(21) "media archive payload"
string(21) "media archive payload"
string(21) "media archive payload"
string(21) "media archive payload"
bool(false)
