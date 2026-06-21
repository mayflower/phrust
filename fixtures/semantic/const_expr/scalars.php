<?php

const CONST_EXPR_SCALAR_INT = 42;
const CONST_EXPR_SCALAR_STRING = "phrust";
const CONST_EXPR_SCALAR_BOOL = true;

function const_expr_scalar_default($value = CONST_EXPR_SCALAR_INT): void {}

function const_expr_static_local(): void
{
    static $cached = 42;
}
