--TEST--
pdo_pgsql: platform checks expose PostgreSQL PDO driver surface
--DESCRIPTION--
Generated coverage for the bounded PDO PostgreSQL platform surface:
extension visibility, PDO driver discovery, and generated Pdo\Pgsql metadata.
--EXTENSIONS--
pdo
pdo_pgsql
--FILE--
<?php
var_dump(extension_loaded("pdo"));
var_dump(extension_loaded("pdo_pgsql"));
var_dump(class_exists("PDO", false));
var_dump(class_exists("PDOStatement", false));
var_dump(class_exists("PDOException", false));
var_dump(class_exists("Pdo\\Pgsql", false));
var_dump(class_exists("PDO_PGSql_Ext", false));
var_dump(function_exists("pdo_drivers"));
var_dump(in_array("pgsql", PDO::getAvailableDrivers(), true));
var_dump(in_array("pgsql", pdo_drivers(), true));
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
bool(true)
