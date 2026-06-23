<?php
class Phase7PropertyLoadLoopDto {
    public int $value = 3;
}

function phase7_property_load_loop_read(Phase7PropertyLoadLoopDto $object): int {
    return $object->value;
}

$object = new Phase7PropertyLoadLoopDto();
$sum = 0;
for ($i = 0; $i < 64; $i++) {
    $sum += phase7_property_load_loop_read($object);
}

echo $sum, "\n";
