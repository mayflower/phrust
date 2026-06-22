<?php
// phase5: kind=pass id=generator-yield-from
function phase4_inner_generator_gap() {
    yield 1;
}

function phase4_yield_from_gap() {
    yield from phase4_inner_generator_gap();
}

foreach (phase4_yield_from_gap() as $value) {
    echo $value;
}
