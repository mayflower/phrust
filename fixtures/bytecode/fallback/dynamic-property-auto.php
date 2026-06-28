<?php
class BytecodeDynamicPropertyTarget {
}

$object = new BytecodeDynamicPropertyTarget();
$object->value = "dynamic";
echo $object->value, "\n";
