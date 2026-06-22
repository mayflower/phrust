<?php
// phase5-runtime: category=generators expect=pass
function gen() {
    yield 1;
    return 9;
}

$g = gen();
foreach ($g as $value) {
    echo $value, "\n";
}
echo $g->getReturn(), "\n";
