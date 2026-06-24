<?php
class PerfPropertyLoadUninitializedDto {
    public int $value;
}

function perf_property_load_uninitialized(PerfPropertyLoadUninitializedDto $object): int {
    return $object->value;
}

echo perf_property_load_uninitialized(new PerfPropertyLoadUninitializedDto()), "\n";
