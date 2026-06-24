<?php
function perf_cranelift_strlen_guard_exit($value): int {
    return strlen($value);
}

try {
    echo perf_cranelift_strlen_guard_exit([]), "\n";
} catch (TypeError $e) {
    echo "type-error\n";
}
