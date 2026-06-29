--TEST--
wp.request-filesystem: local permission and stat mutations
--DESCRIPTION--
Generated local filesystem coverage for chmod, umask, stat ownership fields,
disk-space probes, copy, rename, unlink, mkdir, rmdir, realpath, and glob under
the deterministic PHPT filesystem root.
--FILE--
<?php
$dir = __DIR__ . "/wp-request-filesystem-local";
$nested = $dir . "/nested";
$path = $nested . "/source.txt";
$copy = $nested . "/copy.txt";
$renamed = $nested . "/renamed.txt";
@unlink($path);
@unlink($copy);
@unlink($renamed);
@rmdir($nested);
@rmdir($dir);
$old = umask(0077);
var_dump(is_int($old));
var_dump(mkdir($nested, 0777, true));
umask($old);
var_dump(touch($path));
var_dump(file_put_contents($path, "alpha"));
var_dump(chmod($path, 0640));
echo substr(decoct(fileperms($path)), -4), "\n";
var_dump(is_int(fileowner($path)));
var_dump(is_int(filegroup($path)));
var_dump(is_readable($path));
var_dump(is_writable($path));
$stat = stat($path);
var_dump($stat["size"]);
var_dump(filetype($path));
var_dump(copy($path, $copy));
var_dump(rename($copy, $renamed));
var_dump(file_exists($copy));
var_dump(file_get_contents($renamed));
$free = disk_free_space($dir);
$total = disk_total_space($dir);
var_dump((is_float($free) || is_int($free)) && $free > 0);
var_dump((is_float($total) || is_int($total)) && $total >= $free);
var_dump(is_string(realpath($path)));
var_dump(count(glob($nested . "/*.txt")));
var_dump(unlink($path));
var_dump(unlink($renamed));
var_dump(rmdir($nested));
var_dump(rmdir($dir));
?>
--CLEAN--
<?php
$dir = __DIR__ . "/wp-request-filesystem-local";
@unlink($dir . "/nested/source.txt");
@unlink($dir . "/nested/copy.txt");
@unlink($dir . "/nested/renamed.txt");
@rmdir($dir . "/nested");
@rmdir($dir);
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
int(5)
bool(true)
0640
bool(true)
bool(true)
bool(true)
bool(true)
int(5)
string(4) "file"
bool(true)
bool(true)
bool(false)
string(5) "alpha"
bool(true)
bool(true)
bool(true)
int(2)
bool(true)
bool(true)
bool(true)
bool(true)
