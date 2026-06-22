<?php
// phase5-runtime: category=generators expect=pass
function gen() {
    $value = yield 1;
    echo $value, "\n";
}

$g = gen();
$g->rewind();
$g->send(7);
