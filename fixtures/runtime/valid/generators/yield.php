<?php
// phase5: kind=pass id=generator-yield
function phase4_generator_gap() {
    yield 1;
}

foreach (phase4_generator_gap() as $value) {
    echo $value;
}
