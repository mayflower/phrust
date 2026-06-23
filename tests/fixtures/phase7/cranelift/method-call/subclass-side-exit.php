<?php
class Phase7DirectBase {
    public function value(int $x): int {
        return $x + 1;
    }
}

class Phase7DirectChild extends Phase7DirectBase {
    public function value(int $x): int {
        return $x + 10;
    }
}

$objects = [new Phase7DirectBase(), new Phase7DirectChild(), new Phase7DirectBase()];
$sum = 0;
foreach ($objects as $index => $object) {
    $sum = $sum + $object->value($index);
}
echo $sum, "\n";
