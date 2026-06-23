<?php
class Phase7PropertyMetadataSimple {
    public int $value = 7;
}

function phase7_property_metadata_simple(Phase7PropertyMetadataSimple $object): int {
    return $object->value;
}

$object = new Phase7PropertyMetadataSimple();
echo phase7_property_metadata_simple($object), "\n";
