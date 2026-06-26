--TEST--
Generated standard.strings: integer string offsets read and isset
--DESCRIPTION--
module: standard.strings
generated timestamp: 20260626T000000Z
generator version: phpt-standard-strings-v1
reason: $s[$i] reads a single byte (negative indices count from the end, chained offsets work) and isset() is true only for in-range offsets (tests/strings/offsets_chaining_1.phpt, offsets_chaining_3.phpt)
--FILE--
<?php
$s = "foobar";
var_dump($s[0], $s[3], $s[-1], $s[0][0][0]);
var_dump(isset($s[0]), isset($s[5]), isset($s[6]), isset($s[-1]), isset($s[0][0]));
?>
--EXPECT--
string(1) "f"
string(1) "b"
string(1) "r"
string(1) "f"
bool(true)
bool(true)
bool(false)
bool(true)
bool(true)
