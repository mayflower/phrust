--TEST--
Generated standard.strings: strtok consumes the terminating delimiter
--DESCRIPTION--
module: standard.strings
generated timestamp: 20260626T000000Z
generator version: phpt-standard-strings-v1
reason: each strtok() advances past the delimiter that ended the previous token, so later calls with a different delimiter set do not re-read it (tests/strings/001.phpt)
--FILE--
<?php
$str = "testing 1/2\\3";
var_dump(strtok($str, " "));
var_dump(strtok("/"));
var_dump(strtok("\\"));
var_dump(strtok("."));
?>
--EXPECT--
string(7) "testing"
string(1) "1"
string(1) "2"
string(1) "3"
