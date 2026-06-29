--TEST--
wp.core-builtins: URL, filter, hash, password, and serialization helpers
--DESCRIPTION--
Generated WordPress-oriented coverage for common helper builtins used by core
configuration, request parsing, integrity checks, and cache payloads.
--FILE--
<?php
parse_str("a=1&b=two", $parsed);
var_dump($parsed);
echo parse_url("https://user:pass@example.com:8443/path?q=1#frag", PHP_URL_HOST), "\n";
echo http_build_query(["a" => 1, "b" => "two"]), "\n";
echo urlencode("a b~"), "\n";
echo urldecode("a+b%7E"), "\n";
echo rawurlencode("a b~"), "\n";
echo rawurldecode("a%20b~"), "\n";
echo hash("sha256", "abc"), "\n";
var_dump(hash_equals(hash_hmac("sha256", "data", "key"), hash_hmac("sha256", "data", "key")));
var_dump(strlen(random_bytes(8)));
$random = random_int(10, 12);
var_dump($random >= 10 && $random <= 12);
$hash = password_hash("secret", PASSWORD_BCRYPT, ["cost" => 4]);
var_dump(password_verify("secret", $hash));
var_dump(password_needs_rehash($hash, PASSWORD_BCRYPT, ["cost" => 4]));
$payload = ["id" => 7, "name" => "post"];
var_dump(unserialize(serialize($payload)));
?>
--EXPECT--
array(2) {
  ["a"]=>
  string(1) "1"
  ["b"]=>
  string(3) "two"
}
example.com
a=1&b=two
a+b%7E
a b~
a%20b~
a b~
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
bool(true)
int(8)
bool(true)
bool(true)
bool(false)
array(2) {
  ["id"]=>
  int(7)
  ["name"]=>
  string(4) "post"
}
