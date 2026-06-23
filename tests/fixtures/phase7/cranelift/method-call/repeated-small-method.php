<?php
class Phase7DirectMethodValue {
    public function value(int $x): int {
        return $x + 3;
    }
}

$object = new Phase7DirectMethodValue();
$sum = 0;
for ($i = 0; $i < 12; $i++) {
    $sum = $sum + $object->value($i);
}
echo $sum, "\n";
