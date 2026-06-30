<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_REFERENCE_SEMANTICS
class StaticRefBox {
    public static $value = 1;
}

function bump_static_ref(&$value) {
    $value++;
}

bump_static_ref(StaticRefBox::$value);
echo StaticRefBox::$value;
