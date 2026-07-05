--TEST--
hash_file and hash_hmac_file basic behavior
--SKIPIF--
<?php if (!extension_loaded("hash")) die("skip hash extension not loaded"); ?>
--FILE--
<?php
$path = __DIR__ . "/payload.txt";
file_put_contents($path, "data");
echo hash_file("sha256", $path), "\n";
echo hash_hmac_file("sha256", $path, "key"), "\n";
echo bin2hex(hash_file("md5", $path, true)), "\n";
unlink($path);
?>
--EXPECT--
3a6eb0790f39ac87c94f3856b2dd2c5d110e6811602261a9a923d3bb23adc8b7
5031fe3d989c6d1537a013fa6e739da23463fdaec3b70137d828e36ace221bd0
8d777f385d3dfec8815d20f7496026dc
