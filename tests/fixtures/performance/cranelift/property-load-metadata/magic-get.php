<?php
class PerfPropertyMetadataMagic {
    public function __get(string $name): string {
        return $name . "-magic";
    }
}

$object = new PerfPropertyMetadataMagic();
echo $object->missing, "\n";
