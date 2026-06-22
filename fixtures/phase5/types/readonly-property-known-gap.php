<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_READONLY_PROPERTY
class Box {
    public readonly int $value;

    public function __construct(int $value) {
        $this->value = $value;
    }
}

$box = new Box(1);
echo $box->value, "\n";
