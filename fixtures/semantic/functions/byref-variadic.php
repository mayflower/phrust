<?php

function &byref_variadic(array &$items, string ...$names): array
{
    return $items;
}
