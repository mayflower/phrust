<?php

class ConstExprClassFetch
{
    public const BASE = 1;
    public const DERIVED = self::BASE;

    public int $value = self::DERIVED;

    public function __construct(public int $promoted = self::BASE) {}
}

function const_expr_class_fetch($value = ConstExprClassFetch::DERIVED): void {}
