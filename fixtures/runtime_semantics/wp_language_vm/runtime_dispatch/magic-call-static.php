<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_dispatch fixture_id=WP_A_MAGIC_CALL_STATIC wp_area=magic_call_static
// Reduced WordPress language/VM fixture: __callStatic handles missing and inaccessible static methods without replacing accessible methods.
class MagicStaticBox
{
    public static function real($value)
    {
        return "real:$value";
    }

    private static function hidden($value)
    {
        return "hidden:$value";
    }

    public static function __callStatic($name, $args)
    {
        echo "__callStatic:$name:", count($args), ":", $args[0], "\n";
    }
}

echo MagicStaticBox::real("ok"), "\n";
MagicStaticBox::missing("a", "b");
MagicStaticBox::hidden("secret");
