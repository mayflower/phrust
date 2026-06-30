<?php
// runtime-fixture: expect=known_gap known_gap=E_PHP_RUNTIME_VAR_DUMP_FORMAT_MATRIX category=StdoutMismatch
var_dump(["alpha" => 1, "beta" => [2, 3]]);
