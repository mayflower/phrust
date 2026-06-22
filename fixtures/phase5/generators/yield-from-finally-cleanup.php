<?php
// phase5-runtime: category=generators expect=pass
function gen() {
    try {
        yield from [1];
    } finally {
        echo "cleanup\n";
    }
}

foreach (gen() as $value) {
    echo $value, "\n";
}
