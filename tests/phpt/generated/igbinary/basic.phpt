--TEST--
igbinary primitive, array, alias, and introspection surface
--SKIPIF--
<?php if (!extension_loaded("igbinary")) die("skip igbinary extension not loaded"); ?>
--DESCRIPTION--
contract source: igbinary extension compatibility prompt pack
generator version: manual-extension-pack
reason: covers active extension/function surface plus deterministic primitive and array igbinary round trips
--FILE--
<?php
var_dump(extension_loaded("igbinary"));
var_dump(function_exists("igbinary_serialize"));
var_dump(function_exists("igbinary_unserialize"));

echo bin2hex(igbinary_serialize(null)), "\n";
echo bin2hex(igbinary_serialize(false)), "\n";
echo bin2hex(igbinary_serialize(true)), "\n";
echo bin2hex(igbinary_serialize(123)), "\n";
echo bin2hex(igbinary_serialize(-123)), "\n";
echo bin2hex(igbinary_serialize("")), "\n";
echo bin2hex(igbinary_serialize("first")), "\n";
echo bin2hex(igbinary_serialize(["first", true])), "\n";

var_dump(igbinary_unserialize(hex2bin("0000000200")));
var_dump(igbinary_unserialize(hex2bin("0000000205")));
var_dump(igbinary_unserialize(hex2bin("000000021402060011056669727374060105")));
var_dump(igbinary_unserialize(igbinary_serialize(["a" => 1, "b" => [false, null]])));
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
0000000200
0000000204
0000000205
00000002067b
00000002077b
000000020d
0000000211056669727374
000000021402060011056669727374060105
NULL
bool(true)
array(2) {
  [0]=>
  string(5) "first"
  [1]=>
  bool(true)
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
