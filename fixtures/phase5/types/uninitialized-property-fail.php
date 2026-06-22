<?php
// phase5-runtime: category=types expect=known_gap known_gap=E_PHP_RUNTIME_UNINITIALIZED_PROPERTY_TEXT_COMPAT
class Box {
    public string|null $name;
}

$box = new Box();
echo $box->name, "\n";
