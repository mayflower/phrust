--TEST--
wp.db-network: selected OpenSSL helper behavior
--DESCRIPTION--
coverage for digest, random bytes, method listing, and selected RSA/SHA256
verification behavior.
--SKIPIF--
<?php
if (!extension_loaded("openssl")) {
    die("skip openssl extension is not loaded");
}
?>
--FILE--
<?php
$bytes = openssl_random_pseudo_bytes(8);
var_dump(strlen($bytes));
var_dump(openssl_digest("abc", "sha256"));
var_dump(in_array("sha256", openssl_get_md_methods(), true));
$publicKey = <<<'PEM'
-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDLXp6PkCtbpV+P1gwFQWH6Ez0U
83uEmS8IGnpeI8Fk8rY/vHOZzZZaxRCw+loyc342qCDIQheMOCNm5Fkevz06q757
/oooiLR3yryYGKiKG1IZIiplmtsC95oKrzUSKk60wuI1mbgpMUP5LKi/Tvxes5Pm
kUtXfimz2qgkeUcPpQIDAQAB
-----END PUBLIC KEY-----
PEM;
$signature = base64_decode(
    "HonyonljJhIXsVVzuSVTSJlOBAsBQpvkXx24d5jmyETYEBFSZBbcJkJJAq5fD1GX" .
    "V+tcY3UEH0rt2+l9WPdTAFnykcfiEiRfyQ4VuS4pGDvuyRv/K0qIIv8XPfY4+jwef" .
    "68g9gp+6GItQzCAeG67hVq/qVfC7tUmNsBkxlHo2kQ="
);
var_dump(openssl_verify("data", $signature, $publicKey, OPENSSL_ALGO_SHA256));
var_dump(openssl_verify("wrong", $signature, $publicKey, OPENSSL_ALGO_SHA256));
?>
--EXPECT--
int(8)
string(64) "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
bool(true)
int(1)
int(0)
