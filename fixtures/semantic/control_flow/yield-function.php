<?php
namespace App\ControlFlow;

function numbers(): iterable
{
    yield 1;
    yield from [2, 3];
}
