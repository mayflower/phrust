<?php
class BytecodePropertyWriteDto {
    public $value = 1;
}

$object = new BytecodePropertyWriteDto();
for ($i = 0; $i < 4; $i++) {
    $object->value = $object->value + 1;
}
echo $object->value, "\n";
