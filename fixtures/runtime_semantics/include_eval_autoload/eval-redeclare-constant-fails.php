<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_CONSTANT_REDECLARATION_WARNING_COMPAT
eval('const EVAL_REDECLARED_SYMBOL_CONST = "first";');
eval('const EVAL_REDECLARED_SYMBOL_CONST = "second";');
