<?php
echo 1;
declare(strict_types=1);

function strict_types_late_fixture(int $value): int
{
    return $value;
}
