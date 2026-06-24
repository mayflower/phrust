<?php
// runtime-semantics: category=generators expect=pass
function gen() {
    try {
        yield 1;
    } catch (Exception $exception) {
        echo $exception->getMessage(), "\n";
    }
}

$g = gen();
$g->rewind();
$g->throw(new Exception("boom"));
