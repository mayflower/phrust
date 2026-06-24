<?php
class PerfPropertyMetadataBase {
    public string $value = "base";
}

class PerfPropertyMetadataChild extends PerfPropertyMetadataBase {
    public string $other = "child";
}

function perf_property_metadata_read(PerfPropertyMetadataBase $object): string {
    return $object->value;
}

echo perf_property_metadata_read(new PerfPropertyMetadataBase()), "\n";
echo perf_property_metadata_read(new PerfPropertyMetadataChild()), "\n";
