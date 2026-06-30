<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_VM_CONDITIONAL_FUNCTION_DECLARATION_GAP
eval('if (false) { function eval_conditional_declared_function() { return "no"; } }');
echo function_exists("eval_conditional_declared_function") ? "declared" : "missing";
eval('if (true) { function eval_conditional_declared_function() { return "conditional"; } }');
echo "|", eval_conditional_declared_function(), "\n";
