<?php
// runtime-semantics: category=generators expect=fail
function gen() {
    yield 1;
    throw new Exception("boom");
}

$g = gen();
$g->rewind();
$g->next();
