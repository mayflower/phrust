<?php
class PerfCraneliftMethodMetadataByRef {
    public function bump(int &$value): int {
        $value += 1;
        return $value;
    }
}

$object = new PerfCraneliftMethodMetadataByRef();
$value = 4;
echo $object->bump($value), "|", $value, "\n";
