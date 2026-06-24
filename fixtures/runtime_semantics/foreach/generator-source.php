<?php
// runtime-semantics: category=foreach expect=pass
function gen() {
    yield "a" => 1;
    yield "b" => 2;
}

foreach (gen() as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
