<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_semantics fixture_id=WP_A_PERSISTENT_STATE wp_area=persistent_state
// Reduced WordPress language/VM fixture: static locals and class static properties persist during one VM request.
function next_static_local()
{
    static $value = 0;
    $value++;
    return $value;
}

class PersistentStore
{
    public static $value = 0;

    public static function next()
    {
        self::$value++;
        return self::$value;
    }
}

echo next_static_local(), "|", next_static_local(), "|";
echo PersistentStore::next(), "|", PersistentStore::next(), "\n";
