<?php
// phase5-runtime: category=generators expect=fail
function gen() {
    yield 1;
}

$g = gen();
$g->rewind();
$g->next();
$g->rewind();
