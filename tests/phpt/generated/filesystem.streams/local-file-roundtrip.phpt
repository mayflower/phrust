--TEST--
filesystem.streams: local file roundtrip
--DESCRIPTION--
Generated local filesystem baseline covering root-constrained file writes,
reads, metadata predicates, and unlink behavior.
--FILE--
<?php
$path = __DIR__ . "/local-file-roundtrip.tmp";
@unlink($path);
var_dump(file_put_contents($path, "abc"));
var_dump(file_get_contents($path));
var_dump(file_exists($path));
var_dump(is_file($path));
var_dump(filesize($path));
var_dump(unlink($path));
var_dump(file_exists($path));
?>
--EXPECT--
int(3)
string(3) "abc"
bool(true)
bool(true)
int(3)
bool(true)
bool(false)
