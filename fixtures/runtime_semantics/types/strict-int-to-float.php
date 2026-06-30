<?php
declare(strict_types=1);

function takes_float(float $value): string {
    return gettype($value) . ":" . $value;
}

echo takes_float(1);
