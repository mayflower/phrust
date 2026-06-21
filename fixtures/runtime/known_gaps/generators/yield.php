<?php
// phase4: kind=known_gap id=E_PHP_IR_UNSUPPORTED_GENERATOR
function phase4_generator_gap() {
    yield 1;
}

foreach (phase4_generator_gap() as $value) {
    echo $value;
}
