<?php

namespace App\Types;

interface Reader {}
interface Writer {}
interface Seekable {}
interface Buffer {}

function dnf((Reader&Writer)|Seekable $stream): (Reader&Writer)|Buffer
{
    return $stream;
}
