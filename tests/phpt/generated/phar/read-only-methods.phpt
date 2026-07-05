--TEST--
phar: read-only archive metadata methods
--DESCRIPTION--
Generated PHAR coverage for read-only object methods backed by parsed manifest
metadata and local archive state.
--EXTENSIONS--
phar
--FILE--
<?php
$path = __DIR__ . '/fixture-methods.phar';
$hex = '3c3f706870205f5f48414c545f434f4d50494c455228293b203f3e0a6b000000020000001101000000000c000000666978747572652e70686172000000000d0000006c69622f68656c6c6f2e7068702e000000800092652e00000000000000000000000000000008000000646174612e7478740700000080009265070000000000000000000000000000003c3f706870206563686f202766726f6d2d706861727c273b0a72657475726e2027696e636c7564652d6f6b273b0a7061796c6f6164';
file_put_contents($path, hex2bin($hex));

$archive = new Phar($path);
var_dump($archive->count());
var_dump($archive->offsetExists("data.txt"));
var_dump($archive->offsetExists("./lib/hello.php"));
var_dump($archive->offsetExists("missing.txt"));
var_dump(basename($archive->getPath()));
var_dump($archive->getAlias() !== "");
var_dump(str_contains($archive->getStub(), "__HALT_COMPILER"));
unlink($path);
?>
--EXPECT--
int(2)
bool(true)
bool(true)
bool(false)
string(20) "fixture-methods.phar"
bool(true)
bool(true)
