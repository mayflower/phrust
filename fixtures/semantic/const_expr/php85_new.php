<?php

const CONST_EXPR_PHP85_NEW = new stdClass();

function const_expr_php85_new($value = new stdClass()): void {}

#[ConstExprPhp85New(new stdClass())]
class ConstExprPhp85NewTarget {}
