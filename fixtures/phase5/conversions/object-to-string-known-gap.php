<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_OBJECT_TO_STRING_GAP
class StringableBox
{
    public function __toString()
    {
        return "box";
    }
}

$box = new StringableBox();
echo (string) $box, "\n";
