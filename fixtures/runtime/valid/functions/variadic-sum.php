<?php
function sum(...$xs)
{
    return $xs[0] + $xs[1];
}

echo sum(2, 3), "\n";
