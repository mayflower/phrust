--TEST--
phar: read-only local phar:// MVP
--DESCRIPTION--
Generated Branch 4 PHAR coverage for extension visibility, read-only phar:// stream reads, include from PHAR, and local archive construction.
--EXTENSIONS--
phar
--FILE--
<?php
$path = __DIR__ . '/fixture.phar';
$hex = '3c3f706870205f5f48414c545f434f4d50494c455228293b203f3e0a6b000000020000001101000000000c000000666978747572652e70686172000000000d0000006c69622f68656c6c6f2e7068702e000000800092652e00000000000000000000000000000008000000646174612e7478740700000080009265070000000000000000000000000000003c3f706870206563686f202766726f6d2d706861727c273b0a72657475726e2027696e636c7564652d6f6b273b0a7061796c6f6164';
file_put_contents($path, hex2bin($hex));

var_dump(extension_loaded("phar"));
var_dump(class_exists("Phar", false));
var_dump(class_exists("PharData", false));
var_dump(class_exists("PharFileInfo", false));
echo file_get_contents("phar://" . $path . "/data.txt"), "\n";
$handle = fopen("phar://" . $path . "/data.txt", "rb");
echo stream_get_contents($handle), "\n";
fclose($handle);
$include = "phar://" . $path . "/lib/hello.php";
$result = include $include;
echo $result, "\n";
$archive = new Phar($path);
var_dump($archive instanceof Phar);
unlink($path);
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
payload
payload
from-phar|include-ok
bool(true)
