<?php
class PerfCraneliftMethodMetadataBase {
    public function value(): int {
        return 1;
    }
}

class PerfCraneliftMethodMetadataChild extends PerfCraneliftMethodMetadataBase {
    public function value(): int {
        return 2;
    }
}

function perf_cranelift_method_metadata_value(PerfCraneliftMethodMetadataBase $object): int {
    return $object->value();
}

echo perf_cranelift_method_metadata_value(new PerfCraneliftMethodMetadataBase()), "|";
echo perf_cranelift_method_metadata_value(new PerfCraneliftMethodMetadataChild()), "\n";
