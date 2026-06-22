<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_STATIC_PROPERTY
class Counter {
    public static int $value = 1;
}

echo Counter::$value, "\n";
