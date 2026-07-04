<?php
class BytecodeTypedPropertyTarget {
    public int $value = 7;
}

// Dynamic instantiation keeps this a rich-planned auto-fallback probe.
$class = 'BytecodeTypedPropertyTarget';
echo (new $class())->value, "\n";
