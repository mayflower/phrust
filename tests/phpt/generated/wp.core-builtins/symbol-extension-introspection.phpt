--TEST--
wp.core-builtins: symbol and extension introspection
--DESCRIPTION--
Generated WordPress-oriented core builtin coverage for function, constant,
extension, and version introspection helpers.
--FILE--
<?php
function wp_core_builtin_user_fn() {}
class WpCoreBuiltinParent { public function marker() {} }
class WpCoreBuiltinChild extends WpCoreBuiltinParent {}
interface WpCoreBuiltinContract {}
trait WpCoreBuiltinTrait {}
enum WpCoreBuiltinEnum { case One; }

define("WP_CORE_BUILTINS_MARKER", "ready");
var_dump(defined("WP_CORE_BUILTINS_MARKER"));
var_dump(constant("WP_CORE_BUILTINS_MARKER"));

foreach ([
    "define",
    "defined",
    "constant",
    "function_exists",
    "method_exists",
    "class_exists",
    "interface_exists",
    "trait_exists",
    "enum_exists",
    "is_subclass_of",
    "extension_loaded",
    "get_loaded_extensions",
    "phpversion",
] as $name) {
    echo $name, "=", function_exists($name) ? "yes" : "no", "\n";
}

var_dump(function_exists("wp_core_builtin_user_fn"));
var_dump(method_exists("WpCoreBuiltinChild", "marker"));
var_dump(class_exists("WpCoreBuiltinChild", false));
var_dump(interface_exists("WpCoreBuiltinContract", false));
var_dump(trait_exists("WpCoreBuiltinTrait", false));
var_dump(enum_exists("WpCoreBuiltinEnum", false));
var_dump(is_subclass_of("WpCoreBuiltinChild", "WpCoreBuiltinParent"));
var_dump(extension_loaded("standard"));
var_dump(extension_loaded("wp_core_missing_extension"));
var_dump(in_array("standard", get_loaded_extensions(), true));
var_dump(phpversion() === PHP_VERSION);
var_dump(is_string(phpversion("standard")));
var_dump(phpversion("wp_core_missing_extension"));
?>
--EXPECT--
bool(true)
string(5) "ready"
define=yes
defined=yes
constant=yes
function_exists=yes
method_exists=yes
class_exists=yes
interface_exists=yes
trait_exists=yes
enum_exists=yes
is_subclass_of=yes
extension_loaded=yes
get_loaded_extensions=yes
phpversion=yes
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(false)
bool(true)
bool(true)
bool(true)
bool(false)
