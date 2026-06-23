<?php
class Phase7CraneliftMethodMetadataByRef {
    public function bump(int &$value): int {
        $value += 1;
        return $value;
    }
}

$object = new Phase7CraneliftMethodMetadataByRef();
$value = 4;
echo $object->bump($value), "|", $value, "\n";
