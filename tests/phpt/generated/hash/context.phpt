--TEST--
hash context lifecycle
--SKIPIF--
<?php if (!extension_loaded("hash")) die("skip hash extension not loaded"); ?>
--FILE--
<?php
var_dump(class_exists("HashContext", false));
var_dump(defined("HASH_HMAC"));
var_dump(HASH_HMAC);

$ctx = hash_init("sha256");
var_dump($ctx instanceof HashContext);
var_dump(hash_update($ctx, "ab"));
$copy = hash_copy($ctx);
var_dump(hash_update($ctx, "c"));
var_dump(hash_final($ctx));
var_dump(hash_final($copy));

$hmac = hash_init("sha256", HASH_HMAC, "key");
var_dump(hash_update($hmac, "data"));
var_dump(hash_final($hmac));
try {
    hash_update($hmac, "x");
} catch (Throwable $e) {
    echo get_class($e), "\n";
}

$path = __DIR__ . "/hash-update-payload.txt";
file_put_contents($path, "abcdef");
$file_ctx = hash_init("sha256");
var_dump(hash_update_file($file_ctx, $path));
var_dump(hash_final($file_ctx));

$stream = fopen($path, "r");
$stream_ctx = hash_init("sha256");
var_dump(hash_update_stream($stream_ctx, $stream));
var_dump(hash_final($stream_ctx));

rewind($stream);
$limited_stream_ctx = hash_init("sha256");
var_dump(hash_update_stream($limited_stream_ctx, $stream, 3));
var_dump(hash_final($limited_stream_ctx));
fclose($stream);
unlink($path);

var_dump(hash_pbkdf2("sha256", "password", "salt", 1000, 0, false));
var_dump(hash_pbkdf2("sha256", "password", "salt", 1000, 16, false));
var_dump(bin2hex(hash_pbkdf2("sha256", "password", "salt", 1000, 16, true)));
var_dump(bin2hex(hash_hkdf("sha256", "input key", 16, "info", "salt")));
var_dump(strlen(hash_hkdf("sha256", "input key", 0, "", "")));
?>
--EXPECT--
bool(true)
bool(true)
int(1)
bool(true)
bool(true)
bool(true)
string(64) "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
string(64) "fb8e20fc2e4c3f248c60c39bd652f3c1347298bb977b8b4d5903b85055620603"
bool(true)
string(64) "5031fe3d989c6d1537a013fa6e739da23463fdaec3b70137d828e36ace221bd0"
TypeError
bool(true)
string(64) "bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721"
int(6)
string(64) "bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721"
int(3)
string(64) "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
string(64) "632c2812e46d4604102ba7618e9d6d7d2f8128f6266b4a03264d2a0460b7dcb3"
string(16) "632c2812e46d4604"
string(32) "632c2812e46d4604102ba7618e9d6d7d"
string(32) "a55ecbb7581875a5a23aaee7f492ec48"
int(32)
