<?php
namespace App\Scopes;

function make_closure($seed)
{
    $extra = 1;
    return function ($value) use ($seed, &$extra) {
        return $value + $seed + $extra;
    };
}
