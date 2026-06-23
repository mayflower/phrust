<?php
class Phase7CraneliftMethodMetadataMagic {
    public function __call($name, $args): int {
        return 42;
    }
}

$object = new Phase7CraneliftMethodMetadataMagic();
echo $object->missing(1, 2), "\n";
