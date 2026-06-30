<?php
function binder_strict_callee_takes_int(int $value): void {
    echo "callee:", $value, "\n";
}
