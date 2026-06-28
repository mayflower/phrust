<?php
function bytecode_property_write_target($object) {
    $object->value = 2;
    return $object->value;
}

echo "property-write-fenced\n";
