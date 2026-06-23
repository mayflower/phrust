<?php
class Phase7CraneliftMethodMetadataFinal {
    final public function value(): int {
        return 7;
    }
}

$object = new Phase7CraneliftMethodMetadataFinal();
$sum = 0;
for ($i = 0; $i < 4; $i++) {
    $sum += $object->value();
}
echo $sum, "\n";
