<?php
// runtime-semantics: expect=fail
eval('function eval_redeclared_symbol_function() { return "first"; }');
eval('function eval_redeclared_symbol_function() { return "second"; }');
