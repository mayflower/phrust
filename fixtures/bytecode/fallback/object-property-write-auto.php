<?php
class BytecodePropertyWriteDto {
    public $value = 1;
}

// Dynamic instantiation keeps this a rich-planned auto-fallback probe.
$class = 'BytecodePropertyWriteDto';
$object = new $class();
for ($i = 0; $i < 4; $i++) {
    $object->value = $object->value + 1;
}
echo $object->value, "\n";
