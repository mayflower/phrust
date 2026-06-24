<?php
class PerfPropertyLoadSimpleDto {
    public int $value = 7;
}

function perf_property_load_simple(PerfPropertyLoadSimpleDto $object): int {
    return $object->value;
}

$object = new PerfPropertyLoadSimpleDto();
echo perf_property_load_simple($object), "\n";
