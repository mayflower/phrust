<?php
class PerfPropertyLoadLoopDto {
    public int $value = 3;
}

function perf_property_load_loop_read(PerfPropertyLoadLoopDto $object): int {
    return $object->value;
}

$object = new PerfPropertyLoadLoopDto();
$sum = 0;
for ($i = 0; $i < 64; $i++) {
    $sum += perf_property_load_loop_read($object);
}

echo $sum, "\n";
