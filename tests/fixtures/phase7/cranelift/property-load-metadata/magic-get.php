<?php
class Phase7PropertyMetadataMagic {
    public function __get(string $name): string {
        return $name . "-magic";
    }
}

$object = new Phase7PropertyMetadataMagic();
echo $object->missing, "\n";
