<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_PROPERTY_REFERENCE
class MagicGetReference {
    private string $value = "old";

    public function &__get(string $name): mixed {
        return $this->value;
    }
}

$object = new MagicGetReference();
$ref =& $object->missing;
$ref = "new";
echo $object->missing;
