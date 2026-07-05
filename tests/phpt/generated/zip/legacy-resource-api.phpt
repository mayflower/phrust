--TEST--
zip: legacy procedural resource API
--DESCRIPTION--
Generated coverage for deprecated zip_open/read and zip_entry_* resource helpers.
--SKIPIF--
<?php
if (!extension_loaded("zip")) die("skip zip extension not available");
?>
--FILE--
<?php
error_reporting(E_ALL & ~E_DEPRECATED);
$dir = __DIR__ . "/zip-legacy-resource-api";
$zipPath = $dir . "/fixture.zip";
@unlink($zipPath);
@rmdir($dir);
mkdir($dir);
$bytes =
    "\x50\x4b\x03\x04\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\x8b\x73\x95\xac\x0b\x00\x00\x00\x09\x00\x00\x00\x09\x00\x00\x00\x68\x65\x6c\x6c\x6f\x2e\x74\x78\x74\xcb\x48\xcd\xc9\xc9\x57\xa8\xca\x2c\x00\x00" .
    "\x50\x4b\x03\x04\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\xe9\xc2\xc9\xaa\x08\x00\x00\x00\x06\x00\x00\x00\x0e\x00\x00\x00\x64\x69\x72\x2f\x6e\x65\x73\x74\x65\x64\x2e\x74\x78\x74\xcb\x4b\x2d\x2e\x49\x4d\x01\x00" .
    "\x50\x4b\x01\x02\x14\x03\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\x8b\x73\x95\xac\x0b\x00\x00\x00\x09\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x00\x00\x00\x00\x68\x65\x6c\x6c\x6f\x2e\x74\x78\x74" .
    "\x50\x4b\x01\x02\x14\x03\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\xe9\xc2\xc9\xaa\x08\x00\x00\x00\x06\x00\x00\x00\x0e\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x32\x00\x00\x00\x64\x69\x72\x2f\x6e\x65\x73\x74\x65\x64\x2e\x74\x78\x74" .
    "\x50\x4b\x05\x06\x00\x00\x00\x00\x02\x00\x02\x00\x73\x00\x00\x00\x66\x00\x00\x00\x00\x00";
file_put_contents($zipPath, $bytes);

var_dump(function_exists("zip_open"));
$zip = zip_open($zipPath);
var_dump(is_resource($zip));
$entry = zip_read($zip);
var_dump(is_resource($entry));
var_dump(zip_entry_name($entry));
var_dump(zip_entry_filesize($entry));
var_dump(zip_entry_compressedsize($entry));
var_dump(zip_entry_compressionmethod($entry));
var_dump(zip_entry_open($zip, $entry));
var_dump(zip_entry_read($entry, 20));
var_dump(zip_entry_close($entry));
$entry = zip_read($zip);
var_dump(zip_entry_name($entry));
var_dump(zip_entry_read($entry, 20));
var_dump(zip_read($zip));
var_dump(zip_close($zip));
unlink($zipPath);
rmdir($dir);
?>
--CLEAN--
<?php
$dir = __DIR__ . "/zip-legacy-resource-api";
@unlink($dir . "/fixture.zip");
@rmdir($dir);
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
string(9) "hello.txt"
int(9)
int(11)
string(8) "deflated"
bool(true)
string(9) "hello zip"
bool(true)
string(14) "dir/nested.txt"
string(6) "nested"
bool(false)
NULL
