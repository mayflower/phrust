<?php
class Phase7CraneliftMethodMetadataNormal {
    public function value(): int {
        return 3;
    }
}

$object = new Phase7CraneliftMethodMetadataNormal();
$sum = 0;
for ($i = 0; $i < 5; $i++) {
    $sum += $object->value();
}
echo $sum, "\n";
