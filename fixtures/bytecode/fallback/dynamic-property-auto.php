<?php
class BytecodeDynamicPropertyTarget {
}

// Dynamic instantiation keeps this a rich-planned auto-fallback probe.
$class = 'BytecodeDynamicPropertyTarget';
$object = new $class();
$object->value = "dynamic";
echo $object->value, "\n";
