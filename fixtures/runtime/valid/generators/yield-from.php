<?php
// runtime-fixture: kind=pass id=generator-yield-from
function runtime_inner_generator_gap() {
    yield 1;
}

function runtime_yield_from_gap() {
    yield from runtime_inner_generator_gap();
}

foreach (runtime_yield_from_gap() as $value) {
    echo $value;
}
