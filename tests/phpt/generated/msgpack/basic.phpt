--TEST--
msgpack primitive, list, map, alias, and introspection surface
--SKIPIF--
<?php if (!extension_loaded("msgpack")) die("skip msgpack extension not loaded"); ?>
--DESCRIPTION--
contract source: msgpack extension compatibility prompt pack
generator version: manual-extension-pack
reason: covers active extension/function/constant surface plus deterministic primitive, list, and map MessagePack round trips
--FILE--
<?php
var_dump(extension_loaded("msgpack"));
var_dump(function_exists("msgpack_pack"));
var_dump(function_exists("msgpack_serialize"));
var_dump(function_exists("msgpack_unserialize"));
var_dump(function_exists("msgpack_unpack"));
var_dump(class_exists("MessagePack", false));
var_dump(class_exists("MessagePackUnpacker", false));

echo bin2hex(msgpack_pack(null)), "\n";
echo bin2hex(msgpack_pack(true)), "\n";
echo bin2hex(msgpack_pack([1, "two"])), "\n";
echo bin2hex(msgpack_pack(["a" => 1, "b" => [false, null]])), "\n";

var_dump(msgpack_unpack(hex2bin("c0")));
var_dump(msgpack_unpack(hex2bin("c3")));
var_dump(msgpack_unpack(hex2bin("9201a374776f")));
var_dump(msgpack_unserialize(msgpack_serialize(["a" => 1, "b" => [false, null]])));
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
c0
c3
9201a374776f
82a16101a16292c2c0
NULL
bool(true)
array(2) {
  [0]=>
  int(1)
  [1]=>
  string(3) "two"
}
array(2) {
  ["a"]=>
  int(1)
  ["b"]=>
  array(2) {
    [0]=>
    bool(false)
    [1]=>
    NULL
  }
}
