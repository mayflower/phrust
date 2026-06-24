<?php
// runtime-semantics: category=generators expect=fail
function gen() {
    yield 1;
    return 9;
}

$g = gen();
$g->rewind();
echo $g->getReturn(), "\n";
