--TEST--
redis extension disabled external-service surface
--SKIPIF--
<?php if (!extension_loaded("redis")) die("skip redis extension not loaded"); ?>
--FILE--
<?php
var_dump(extension_loaded("redis"));
var_dump(class_exists("Redis", false));
?>
--EXPECT--
bool(true)
bool(true)
