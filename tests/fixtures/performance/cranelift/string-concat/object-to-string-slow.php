<?php
class PerfCraneliftConcatObject {
    public function __toString(): string {
        return "object";
    }
}

function perf_cranelift_concat_object($lhs, $rhs): string {
    return $lhs . $rhs;
}

echo perf_cranelift_concat_object("value:", new PerfCraneliftConcatObject()), "\n";
