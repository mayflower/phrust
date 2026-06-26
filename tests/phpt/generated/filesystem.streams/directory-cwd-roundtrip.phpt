--TEST--
filesystem.streams: directory and cwd roundtrip
--DESCRIPTION--
Generated directory baseline covering mkdir, rmdir, is_dir, getcwd, and chdir
state persistence.
--FILE--
<?php
$base = getcwd();
$dir = __DIR__ . "/directory-cwd-roundtrip-dir";
@rmdir($dir);
var_dump(mkdir($dir));
var_dump(is_dir($dir));
var_dump(chdir($dir));
var_dump(basename(getcwd()) === "directory-cwd-roundtrip-dir");
var_dump(chdir($base));
var_dump(rmdir($dir));
var_dump(is_dir($dir));
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(false)
