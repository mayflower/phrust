<?php
// runtime-semantics: category=generators expect=fail
function inner() {
    yield 1;
    throw new Exception("boom");
}

function outer() {
    yield from inner();
}

$g = outer();
$g->rewind();
$g->next();
