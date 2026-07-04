<?php declare(strict_types=1);

class StrictTypedService {
    public function takesInt(int $n): int {
        return $n + 1;
    }

    public static function scaled(int $n): int {
        return $n * 10;
    }
}

function strict_takes_float(float $f): float {
    return $f / 2;
}
