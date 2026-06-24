<?php
// runtime-semantics: category=generators expect=pass
function gen() {
    echo "body\n";
    yield 1;
}

$g = gen();
echo "created\n";
foreach ($g as $value) {
    echo "v:", $value, "\n";
}
