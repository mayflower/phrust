<?php
namespace App\Scopes;

function make_arrow($seed)
{
    return fn($value) => $value + $seed;
}
