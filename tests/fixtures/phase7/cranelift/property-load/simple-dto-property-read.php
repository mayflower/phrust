<?php
class Phase7PropertyLoadSimpleDto {
    public int $value = 7;
}

function phase7_property_load_simple(Phase7PropertyLoadSimpleDto $object): int {
    return $object->value;
}

$object = new Phase7PropertyLoadSimpleDto();
echo phase7_property_load_simple($object), "\n";
