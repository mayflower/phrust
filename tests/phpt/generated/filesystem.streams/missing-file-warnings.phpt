--TEST--
filesystem.streams: missing file warnings
--DESCRIPTION--
Generated warning baseline covering missing local file reads and opens.
--INI--
display_errors=1
--FILE--
<?php
$path = __DIR__ . "/missing-file-warning.tmp";
@unlink($path);
var_dump(file_get_contents($path));
var_dump(fopen($path, "r"));
?>
--EXPECTF--
Warning: file_get_contents(%s/missing-file-warning.tmp): Failed to open stream: No such file or directory in %s on line %d
bool(false)

Warning: fopen(%s/missing-file-warning.tmp): Failed to open stream: No such file or directory in %s on line %d
bool(false)
