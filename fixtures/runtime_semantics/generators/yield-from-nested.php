<?php
// runtime-semantics: category=generators expect=pass
function leaf() {
    yield 1;
    yield 2;
}

function middle() {
    yield from leaf();
    yield 3;
}

function root() {
    yield from middle();
    yield from [4];
}

foreach (root() as $value) {
    echo $value;
}
echo "\n";
