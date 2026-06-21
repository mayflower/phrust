<?php
namespace App\Scopes;

function counter(): int
{
    global $shared;
    static $count = 0;
    $count++;
    return $count + $shared;
}
