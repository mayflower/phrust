<?php
class Phase7PropertyLoadUninitializedDto {
    public int $value;
}

function phase7_property_load_uninitialized(Phase7PropertyLoadUninitializedDto $object): int {
    return $object->value;
}

echo phase7_property_load_uninitialized(new Phase7PropertyLoadUninitializedDto()), "\n";
