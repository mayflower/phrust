<?php
class BytecodeMethodTarget {
    public function value($input) {
        return $input + 2;
    }
}

$object = new BytecodeMethodTarget();
$sum = 0;
for ($i = 0; $i < 4; $i++) {
    $sum = $sum + $object->value($i);
}
echo $sum, "\n";
