<?php
// runtime-semantics: category=clone_with expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_PROPERTY_REFERENCE
class CloneReferenceProperty {
    public mixed $value;
}

$source = "old";
$original = new CloneReferenceProperty();
$original->value =& $source;
$copy = clone $original;
$copy->value = "new";
echo $source;
