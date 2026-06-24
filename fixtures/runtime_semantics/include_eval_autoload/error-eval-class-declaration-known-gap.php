<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_VM_EVAL_DECLARATION_GAP
eval('class EvalDeclaredClassFixture { public function value() { return 1; } }');
