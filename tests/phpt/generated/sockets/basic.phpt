--TEST--
sockets default-disabled creation basics
--SKIPIF--
<?php if (!extension_loaded("sockets")) die("skip sockets extension not loaded"); ?>
--FILE--
<?php
var_dump(@socket_create(999999, 999999, 999999));
echo is_string(socket_strerror(0)) ? "string\n" : "not-string\n";
?>
--EXPECT--
bool(false)
string
