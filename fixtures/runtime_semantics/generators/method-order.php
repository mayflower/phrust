<?php
// runtime-semantics: category=generators expect=pass
function gen() {
    yield "a" => 7;
}

$g = gen();
echo $g->valid() ? "T" : "F";
echo "|", $g->current(), "|", $g->key();
$g->next();
echo "|", $g->valid() ? "T" : "F", "\n";
