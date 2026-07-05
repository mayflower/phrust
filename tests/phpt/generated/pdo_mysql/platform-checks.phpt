--TEST--
pdo_mysql: platform checks expose MySQL PDO driver surface
--DESCRIPTION--
Generated coverage for the bounded PDO MySQL platform surface: extension
visibility, PDO driver discovery, and generated Pdo\Mysql class metadata.
--EXTENSIONS--
pdo
pdo_mysql
--FILE--
<?php
var_dump(extension_loaded("pdo"));
var_dump(extension_loaded("pdo_mysql"));
var_dump(class_exists("PDO", false));
var_dump(class_exists("PDOStatement", false));
var_dump(class_exists("PDOException", false));
var_dump(class_exists("Pdo\\Mysql", false));
var_dump(function_exists("pdo_drivers"));
var_dump(in_array("mysql", PDO::getAvailableDrivers(), true));
var_dump(in_array("mysql", pdo_drivers(), true));
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
