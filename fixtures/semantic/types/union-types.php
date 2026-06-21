<?php

namespace App\Types;

function union_parameter(int|string|null $value): int|string|null
{
    return $value;
}
