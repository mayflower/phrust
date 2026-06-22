<?php
// phase5-runtime: category=magic expect=pass
class MagicToStringEcho {
    public function __toString(): string {
        return "text";
    }
}

$value = new MagicToStringEcho();
echo $value . ":" . (string) $value;
