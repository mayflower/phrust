--TEST--
wp.request-filesystem: gzip file handle helpers
--DESCRIPTION--
Generated zlib file-handle coverage for gzopen, gzwrite, gzread, gzclose, and
whole-buffer gzip decoding.
--SKIPIF--
<?php
if (!extension_loaded("zlib")) die("skip zlib extension not available");
?>
--FILE--
<?php
$path = __DIR__ . "/wp-request-filesystem.gz";
@unlink($path);
$handle = gzopen($path, "wb");
var_dump(is_resource($handle));
var_dump(gzwrite($handle, "wordpress package"));
var_dump(gzclose($handle));
$handle = gzopen($path, "rb");
var_dump(gzread($handle, 100));
var_dump(gzclose($handle));
var_dump(gzdecode(file_get_contents($path)));
unlink($path);
?>
--CLEAN--
<?php
@unlink(__DIR__ . "/wp-request-filesystem.gz");
?>
--EXPECT--
bool(true)
int(17)
bool(true)
string(17) "wordpress package"
bool(true)
string(17) "wordpress package"
