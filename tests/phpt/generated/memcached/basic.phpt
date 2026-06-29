--TEST--
memcached extension disabled external-service surface
--SKIPIF--
<?php if (!extension_loaded("memcached")) die("skip memcached extension not loaded"); ?>
--FILE--
<?php
var_dump(extension_loaded("memcached"));
var_dump(class_exists("Memcached", false));
?>
--EXPECT--
bool(true)
bool(true)
