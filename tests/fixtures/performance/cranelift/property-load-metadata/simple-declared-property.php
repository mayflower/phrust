<?php
class PerfPropertyMetadataSimple {
    public int $value = 7;
}

function perf_property_metadata_simple(PerfPropertyMetadataSimple $object): int {
    return $object->value;
}

$object = new PerfPropertyMetadataSimple();
echo perf_property_metadata_simple($object), "\n";
