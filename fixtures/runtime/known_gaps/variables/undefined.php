<?php
// runtime-fixture: kind=known_gap id=E_PHP_RUNTIME_UNDEFINED_VARIABLE_WARNING expected_stdout="x\n" diagnostic_emitted=true
echo $missing, "x\n";
