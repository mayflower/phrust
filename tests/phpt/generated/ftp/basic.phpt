--TEST--
ftp default-disabled connection basics
--SKIPIF--
<?php if (!extension_loaded("ftp")) die("skip ftp extension not loaded"); ?>
--FILE--
<?php
var_dump(@ftp_connect("127.0.0.1", 1, 1));
var_dump(@ftp_ssl_connect("127.0.0.1", 1, 1));
?>
--EXPECT--
bool(false)
bool(false)
