<?php
class BytecodeMethodTarget {
    public function value($input) {
        return $input + 2;
    }
}

// Dynamic instantiation keeps this a rich-planned auto-fallback probe.
$class = 'BytecodeMethodTarget';
$object = new $class();
$sum = 0;
for ($i = 0; $i < 4; $i++) {
    $sum = $sum + $object->value($i);
}
echo $sum, "\n";
