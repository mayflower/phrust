<?php
class PerfCraneliftMethodMetadataFinal {
    final public function value(): int {
        return 7;
    }
}

$object = new PerfCraneliftMethodMetadataFinal();
$sum = 0;
for ($i = 0; $i < 4; $i++) {
    $sum += $object->value();
}
echo $sum, "\n";
