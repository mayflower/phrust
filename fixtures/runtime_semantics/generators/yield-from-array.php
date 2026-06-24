<?php
// runtime-semantics: category=generators expect=pass
function gen() {
    yield from ["a" => 1, "b" => 2];
}

foreach (gen() as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
