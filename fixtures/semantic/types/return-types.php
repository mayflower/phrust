<?php

namespace App\Types;

use Vendor\Result;

function done(): void
{
}

function fail(): never
{
    throw new \RuntimeException('fail');
}

function maybe_result(): ?Result
{
    return null;
}
