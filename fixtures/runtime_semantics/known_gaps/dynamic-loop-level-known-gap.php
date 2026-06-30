<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_DYNAMIC_LOOP_CONTROL_LEVEL
// PHP reference: static loop-control levels are validated against active loop depth.
for ($i = 0; $i < 1; $i++) {
    continue 3;
}
