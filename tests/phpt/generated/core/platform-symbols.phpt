--TEST--
core extension platform symbols and INI compatibility slice
--FILE--
<?php
var_dump(PHP_VERSION);
var_dump(PHP_VERSION_ID);
var_dump(defined("E_ERROR"), E_ERROR);
var_dump(defined("E_ALL"), E_ALL);
var_dump(extension_loaded("core"));
var_dump(extension_loaded("standard"));
var_dump(function_exists("ini_get"));
var_dump(function_exists("ini_set"));
var_dump(function_exists("ini_get_all"));
var_dump(function_exists("get_cfg_var"));
var_dump(function_exists("extension_loaded"));
var_dump(function_exists("php_sapi_name"));
var_dump(php_sapi_name());
var_dump(ini_get("memory_limit"));
var_dump(ini_get("serialize_precision"));
$old = ini_set("memory_limit", "256M");
var_dump($old, ini_get("memory_limit"));
var_dump(ini_set("does.not.exist", "x"));
$all = ini_get_all(null, false);
var_dump(is_array($all));
var_dump(isset($all["memory_limit"]));
var_dump(isset($all["serialize_precision"]));
$constants = get_defined_constants(true);
var_dump(isset($constants["Core"]));
var_dump(isset($constants["Core"]["PHP_VERSION"]));
var_dump(isset($constants["Core"]["E_ERROR"]));
$functions = get_defined_functions();
var_dump(isset($functions["internal"]));
var_dump(in_array("ini_get", $functions["internal"], true));
var_dump(in_array("php_sapi_name", $functions["internal"], true));
foreach ([
    "Throwable",
    "Exception",
    "Error",
    "TypeError",
    "ValueError",
    "ErrorException",
    "ParseError",
    "ArithmeticError",
    "DivisionByZeroError",
] as $symbol) {
    var_dump($symbol, class_exists($symbol, false) || interface_exists($symbol, false));
}
?>
--EXPECT--
string(5) "8.5.7"
int(80507)
bool(true)
int(1)
bool(true)
int(30719)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
string(3) "cli"
string(4) "128M"
string(2) "-1"
string(4) "128M"
string(4) "256M"
bool(false)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
string(9) "Throwable"
bool(true)
string(9) "Exception"
bool(true)
string(5) "Error"
bool(true)
string(9) "TypeError"
bool(true)
string(10) "ValueError"
bool(true)
string(14) "ErrorException"
bool(true)
string(10) "ParseError"
bool(true)
string(15) "ArithmeticError"
bool(true)
string(19) "DivisionByZeroError"
bool(true)
