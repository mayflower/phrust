<?php
class PerfPropertyLoadBaseDto {
    public int $value = 5;
}

class PerfPropertyLoadChildDto extends PerfPropertyLoadBaseDto {
    public int $other = 1;
}

function perf_property_load_base(PerfPropertyLoadBaseDto $object): int {
    return $object->value;
}

echo perf_property_load_base(new PerfPropertyLoadBaseDto()), "|";
echo perf_property_load_base(new PerfPropertyLoadChildDto()), "\n";
