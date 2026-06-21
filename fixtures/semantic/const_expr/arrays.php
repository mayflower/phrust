<?php

const CONST_EXPR_ARRAY = [
    "name" => "phrust",
    "flags" => [true, false, null],
    "sum" => 1 + 2,
];

function const_expr_array_default($value = ["x" => CONST_EXPR_ARRAY]): void {}
