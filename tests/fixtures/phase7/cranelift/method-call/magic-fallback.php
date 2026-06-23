<?php
class Phase7DirectMagic {
    public function __call(string $name, array $args): int {
        return strlen($name) + $args[0];
    }
}

$object = new Phase7DirectMagic();
echo $object->missing(5), "\n";
