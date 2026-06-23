<?php
class Phase7PropertyMetadataBase {
    public string $value = "base";
}

class Phase7PropertyMetadataChild extends Phase7PropertyMetadataBase {
    public string $other = "child";
}

function phase7_property_metadata_read(Phase7PropertyMetadataBase $object): string {
    return $object->value;
}

echo phase7_property_metadata_read(new Phase7PropertyMetadataBase()), "\n";
echo phase7_property_metadata_read(new Phase7PropertyMetadataChild()), "\n";
