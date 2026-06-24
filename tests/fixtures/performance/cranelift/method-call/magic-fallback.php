<?php
class PerfDirectMagic {
    public function __call(string $name, array $args): int {
        return strlen($name) + $args[0];
    }
}

$object = new PerfDirectMagic();
echo $object->missing(5), "\n";
