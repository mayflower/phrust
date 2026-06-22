<?php
// phase5-runtime: category=generators expect=pass
function gen() {
    yield "a" => 7;
}

foreach (gen() as $key => $value) {
    echo $key, ":", $value, "\n";
}
