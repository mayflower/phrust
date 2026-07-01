<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_semantics fixture_id=WP_A_CLASS_STATIC_PROPERTY_LVALUES wp_area=static_property_lvalues
// Reduced WordPress language/VM fixture: class and dynamic-class static property lvalues support writes and read-after-write.
class StaticConfig
{
    public static $value = "a";
    public static $map = [];
}

StaticConfig::$value = "b";
StaticConfig::$value .= "c";
StaticConfig::$map["x"] = "y";
StaticConfig::$map["z"] = "w";

echo StaticConfig::$value, "|", StaticConfig::$map["x"], "|", StaticConfig::$map["z"], "\n";
