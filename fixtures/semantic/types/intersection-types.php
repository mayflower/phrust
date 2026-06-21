<?php

namespace App\Types;

interface Reader {}
interface Writer {}

function both(Reader&Writer $stream): Reader&Writer
{
    return $stream;
}
