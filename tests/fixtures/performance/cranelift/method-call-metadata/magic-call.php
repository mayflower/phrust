<?php
class PerfCraneliftMethodMetadataMagic {
    public function __call($name, $args): int {
        return 42;
    }
}

$object = new PerfCraneliftMethodMetadataMagic();
echo $object->missing(1, 2), "\n";
