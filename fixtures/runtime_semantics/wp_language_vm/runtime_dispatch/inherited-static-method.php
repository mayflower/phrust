<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_dispatch fixture_id=WP_A_INHERITED_STATIC_METHOD wp_area=static_dispatch
// Reduced WordPress language/VM fixture: inherited static methods preserve declaring class and called class.
class ParentStatic
{
    public static function label()
    {
        echo "parent|";
        return static::class . "|" . self::class;
    }
}

class ChildStatic extends ParentStatic
{
}

echo ChildStatic::label(), "\n";
