--TEST--
APCu request-local cache basics
--SKIPIF--
<?php if (!extension_loaded("apcu")) die("skip apcu extension not loaded"); ?>
--FILE--
<?php
var_dump(apcu_enabled());
var_dump(apcu_store("k", "v"));
var_dump(apcu_fetch("k", $ok));
var_dump($ok);
var_dump(apcu_add("k", "other"));
var_dump(apcu_exists("k"));
var_dump(apcu_delete("k"));
var_dump(apcu_fetch("k"));
?>
--EXPECT--
bool(true)
bool(true)
string(1) "v"
bool(true)
bool(false)
bool(true)
bool(true)
bool(false)
