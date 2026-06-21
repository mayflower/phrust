<?php
$x = 10;

function bump($x)
{
    $x = $x + 1;
    return $x;
}

echo bump(1), "|", $x, "\n";
