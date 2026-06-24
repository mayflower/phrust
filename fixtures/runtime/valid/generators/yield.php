<?php
// runtime-fixture: kind=pass id=generator-yield
function runtime_generator_gap() {
    yield 1;
}

foreach (runtime_generator_gap() as $value) {
    echo $value;
}
