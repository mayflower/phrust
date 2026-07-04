<?php
class BytecodePropertyReadDto {
    public $value = 3;
}

// Dynamic instantiation keeps this a rich-planned auto-fallback probe.
$class = 'BytecodePropertyReadDto';
$object = new $class();
$sum = 0;
for ($i = 0; $i < 4; $i++) {
    $sum = $sum + $object->value;
}
echo $sum, "\n";
