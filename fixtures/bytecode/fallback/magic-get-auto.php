<?php
class BytecodeMagicGetTarget {
    public function __get($name) {
        return $name . "!";
    }
}

// Dynamic instantiation keeps this a rich-planned auto-fallback probe.
$class = 'BytecodeMagicGetTarget';
echo (new $class())->missing, "\n";
