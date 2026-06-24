<?php
class PerfDirectBase {
    public function value(int $x): int {
        return $x + 1;
    }
}

class PerfDirectChild extends PerfDirectBase {
    public function value(int $x): int {
        return $x + 10;
    }
}

$objects = [new PerfDirectBase(), new PerfDirectChild(), new PerfDirectBase()];
$sum = 0;
foreach ($objects as $index => $object) {
    $sum = $sum + $object->value($index);
}
echo $sum, "\n";
