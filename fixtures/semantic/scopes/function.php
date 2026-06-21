<?php
namespace App\Scopes;

function combine(string $left, string &$right, mixed ...$rest): string
{
    return $left . $right;
}
