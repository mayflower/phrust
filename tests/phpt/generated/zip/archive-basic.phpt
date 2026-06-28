--TEST--
zip: ZipArchive read, list, extract, and invalid archive MVP
--DESCRIPTION--
Generated ZipArchive coverage for plugin/theme archive read and extract flows.
--SKIPIF--
<?php
if (!extension_loaded("zip")) die("skip zip extension not available");
?>
--FILE--
<?php
$dir = __DIR__ . "/zip-archive-basic";
$zipPath = $dir . "/fixture.zip";
$badPath = $dir . "/bad.zip";
$out = $dir . "/out";
@unlink($out . "/dir/nested.txt");
@rmdir($out . "/dir");
@rmdir($out);
@unlink($zipPath);
@unlink($badPath);
@rmdir($dir);
mkdir($dir);
$bytes =
    "\x50\x4b\x03\x04\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\x8b\x73\x95\xac\x0b\x00\x00\x00\x09\x00\x00\x00\x09\x00\x00\x00\x68\x65\x6c\x6c\x6f\x2e\x74\x78\x74\xcb\x48\xcd\xc9\xc9\x57\xa8\xca\x2c\x00\x00" .
    "\x50\x4b\x03\x04\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\xe9\xc2\xc9\xaa\x08\x00\x00\x00\x06\x00\x00\x00\x0e\x00\x00\x00\x64\x69\x72\x2f\x6e\x65\x73\x74\x65\x64\x2e\x74\x78\x74\xcb\x4b\x2d\x2e\x49\x4d\x01\x00" .
    "\x50\x4b\x01\x02\x14\x03\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\x8b\x73\x95\xac\x0b\x00\x00\x00\x09\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x00\x00\x00\x00\x68\x65\x6c\x6c\x6f\x2e\x74\x78\x74" .
    "\x50\x4b\x01\x02\x14\x03\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\xe9\xc2\xc9\xaa\x08\x00\x00\x00\x06\x00\x00\x00\x0e\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x32\x00\x00\x00\x64\x69\x72\x2f\x6e\x65\x73\x74\x65\x64\x2e\x74\x78\x74" .
    "\x50\x4b\x05\x06\x00\x00\x00\x00\x02\x00\x02\x00\x73\x00\x00\x00\x66\x00\x00\x00\x00\x00";
file_put_contents($zipPath, $bytes);
file_put_contents($badPath, "not a zip");
$zip = new ZipArchive();
var_dump($zip->open($zipPath));
var_dump($zip->count());
var_dump($zip->numFiles);
var_dump($zip->getNameIndex(0));
var_dump($zip->getNameIndex(99));
var_dump($zip->getFromName("hello.txt"));
var_dump($zip->locateName("missing.txt"));
$stat = $zip->statName("dir/nested.txt");
echo $stat["name"], "|", $stat["size"], "\n";
var_dump($zip->extractTo($out, "dir/nested.txt"));
var_dump(file_get_contents($out . "/dir/nested.txt"));
var_dump($zip->close());
var_dump($zip->open($badPath));
?>
--CLEAN--
<?php
$dir = __DIR__ . "/zip-archive-basic";
@unlink($dir . "/out/dir/nested.txt");
@rmdir($dir . "/out/dir");
@rmdir($dir . "/out");
@unlink($dir . "/fixture.zip");
@unlink($dir . "/bad.zip");
@rmdir($dir);
?>
--EXPECT--
bool(true)
int(2)
int(2)
string(9) "hello.txt"
bool(false)
string(9) "hello zip"
bool(false)
dir/nested.txt|6
bool(true)
string(6) "nested"
bool(true)
bool(false)
