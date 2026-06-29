--TEST--
wp.core-builtins: ini, environment, and config helpers
--DESCRIPTION--
Generated WordPress-oriented coverage for request-independent INI, environment,
SAPI, and memory helper behavior.
--FILE--
<?php
$previous = ini_set("default_charset", "UTF-8");
var_dump(is_string($previous));
var_dump(ini_get("default_charset"));
$all = ini_get_all(null, false);
var_dump(isset($all["default_charset"]));
var_dump(get_cfg_var("__phrust_missing_cfg_var__"));

putenv("PHRUST_WP_CORE_BUILTINS_ENV=ok");
var_dump(getenv("PHRUST_WP_CORE_BUILTINS_ENV"));
var_dump(is_string(php_sapi_name()));
var_dump(memory_get_usage() >= 0);
var_dump(memory_get_peak_usage() >= memory_get_usage());
?>
--EXPECT--
bool(true)
string(5) "UTF-8"
bool(true)
bool(false)
string(2) "ok"
bool(true)
bool(true)
bool(true)
