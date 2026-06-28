<?php
class BytecodePropertyReadDto {
    public $value = 3;
}

$object = new BytecodePropertyReadDto();
$sum = 0;
for ($i = 0; $i < 4; $i++) {
    $sum = $sum + $object->value;
}
echo $sum, "\n";
