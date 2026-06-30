<?php
declare(strict_types=1);

function binder_weak_caller_takes_int(int $value): void {
    echo "callee:", $value + 1, "\n";
}
