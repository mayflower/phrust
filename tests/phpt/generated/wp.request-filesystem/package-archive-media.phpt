--TEST--
wp.request-filesystem: package archive and media primitives
--DESCRIPTION--
Generated coverage for read-only PHAR file reads/includes, ZipArchive package
extraction, and fileinfo MIME detection used by WordPress update flows.
--SKIPIF--
<?php
if (!extension_loaded("phar")) die("skip phar extension not available");
if (!extension_loaded("zip")) die("skip zip extension not available");
if (!extension_loaded("fileinfo")) die("skip fileinfo extension not available");
?>
--FILE--
<?php
$dir = __DIR__ . "/wp-request-filesystem-package";
$pharPath = $dir . "/fixture.phar";
$zipPath = $dir . "/fixture.zip";
$out = $dir . "/out";
$pdf = $dir . "/doc.pdf";
@unlink($out . "/dir/nested.txt");
@rmdir($out . "/dir");
@rmdir($out);
@unlink($pharPath);
@unlink($zipPath);
@unlink($pdf);
@rmdir($dir);
mkdir($dir);
$pharHex = '3c3f706870205f5f48414c545f434f4d50494c455228293b203f3e0a6b000000020000001101000000000c000000666978747572652e70686172000000000d0000006c69622f68656c6c6f2e7068702e000000800092652e00000000000000000000000000000008000000646174612e7478740700000080009265070000000000000000000000000000003c3f706870206563686f202766726f6d2d706861727c273b0a72657475726e2027696e636c7564652d6f6b273b0a7061796c6f6164';
file_put_contents($pharPath, hex2bin($pharHex));
$pharFile = "phar://" . $pharPath . "/data.txt";
var_dump(file_exists($pharFile));
var_dump(is_file($pharFile));
var_dump(file_get_contents($pharFile));
$include = "phar://" . $pharPath . "/lib/hello.php";
var_dump(include $include);
$zipBytes =
    "\x50\x4b\x03\x04\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\x8b\x73\x95\xac\x0b\x00\x00\x00\x09\x00\x00\x00\x09\x00\x00\x00\x68\x65\x6c\x6c\x6f\x2e\x74\x78\x74\xcb\x48\xcd\xc9\xc9\x57\xa8\xca\x2c\x00\x00" .
    "\x50\x4b\x03\x04\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\xe9\xc2\xc9\xaa\x08\x00\x00\x00\x06\x00\x00\x00\x0e\x00\x00\x00\x64\x69\x72\x2f\x6e\x65\x73\x74\x65\x64\x2e\x74\x78\x74\xcb\x4b\x2d\x2e\x49\x4d\x01\x00" .
    "\x50\x4b\x01\x02\x14\x03\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\x8b\x73\x95\xac\x0b\x00\x00\x00\x09\x00\x00\x00\x09\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x00\x00\x00\x00\x68\x65\x6c\x6c\x6f\x2e\x74\x78\x74" .
    "\x50\x4b\x01\x02\x14\x03\x14\x00\x00\x00\x08\x00\xd0\xad\xdc\x5c\xe9\xc2\xc9\xaa\x08\x00\x00\x00\x06\x00\x00\x00\x0e\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x32\x00\x00\x00\x64\x69\x72\x2f\x6e\x65\x73\x74\x65\x64\x2e\x74\x78\x74" .
    "\x50\x4b\x05\x06\x00\x00\x00\x00\x02\x00\x02\x00\x73\x00\x00\x00\x66\x00\x00\x00\x00\x00";
file_put_contents($zipPath, $zipBytes);
$zip = new ZipArchive();
var_dump($zip->open($zipPath));
var_dump($zip->getFromName("hello.txt"));
var_dump($zip->extractTo($out, "dir/nested.txt"));
var_dump(file_get_contents($out . "/dir/nested.txt"));
var_dump($zip->close());
file_put_contents($pdf, "%PDF-1.7\n");
$finfo = finfo_open(FILEINFO_MIME_TYPE);
var_dump(finfo_file($finfo, $pdf));
var_dump(finfo_buffer($finfo, "PK\x03\x04fixture"));
var_dump(finfo_close($finfo));
?>
--CLEAN--
<?php
$dir = __DIR__ . "/wp-request-filesystem-package";
@unlink($dir . "/out/dir/nested.txt");
@rmdir($dir . "/out/dir");
@rmdir($dir . "/out");
@unlink($dir . "/fixture.phar");
@unlink($dir . "/fixture.zip");
@unlink($dir . "/doc.pdf");
@rmdir($dir);
?>
--EXPECT--
bool(true)
bool(true)
string(7) "payload"
from-phar|string(10) "include-ok"
bool(true)
string(9) "hello zip"
bool(true)
string(6) "nested"
bool(true)
string(15) "application/pdf"
string(15) "application/zip"
bool(true)
