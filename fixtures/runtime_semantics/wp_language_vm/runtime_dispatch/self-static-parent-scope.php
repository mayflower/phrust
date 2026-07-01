<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_dispatch fixture_id=WP_A_SELF_STATIC_PARENT wp_area=class_scope_resolution
// Reduced WordPress language/VM fixture: self/static/parent share the same scope model for methods, constants, and static properties.
class ScopeBase
{
    public const NAME = "base";
    public static $name = "base-prop";

    public static function label()
    {
        return "base-method";
    }
}

class ScopeChild extends ScopeBase
{
    public const NAME = "child";
    public static $name = "child-prop";

    public static function label()
    {
        return "child-method";
    }

    public static function report()
    {
        echo self::label(), "|", static::label(), "|", parent::label(), "|";
        echo self::NAME, "|", static::NAME, "|", parent::NAME, "|";
        echo self::$name, "|", static::$name, "|", parent::$name, "\n";
    }
}

class ScopeGrandChild extends ScopeChild
{
    public const NAME = "grand";
    public static $name = "grand-prop";

    public static function label()
    {
        return "grand-method";
    }
}

ScopeChild::report();
ScopeGrandChild::report();
