<?php
class Phase7CraneliftMethodMetadataBase {
    public function value(): int {
        return 1;
    }
}

class Phase7CraneliftMethodMetadataChild extends Phase7CraneliftMethodMetadataBase {
    public function value(): int {
        return 2;
    }
}

function phase7_cranelift_method_metadata_value(Phase7CraneliftMethodMetadataBase $object): int {
    return $object->value();
}

echo phase7_cranelift_method_metadata_value(new Phase7CraneliftMethodMetadataBase()), "|";
echo phase7_cranelift_method_metadata_value(new Phase7CraneliftMethodMetadataChild()), "\n";
