<?php
class PerfDirectMethodValue {
    public function value(int $x): int {
        return $x + 3;
    }
}

$object = new PerfDirectMethodValue();
$sum = 0;
for ($i = 0; $i < 12; $i++) {
    $sum = $sum + $object->value($i);
}
echo $sum, "\n";
