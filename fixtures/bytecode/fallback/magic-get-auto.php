<?php
class BytecodeMagicGetTarget {
    public function __get($name) {
        return $name . "!";
    }
}

echo (new BytecodeMagicGetTarget())->missing, "\n";
