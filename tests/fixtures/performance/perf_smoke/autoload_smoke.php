<?php
spl_autoload_register(function ($class) {
    if (strtolower($class) === 'perfautoloadsmoke') {
        require 'tests/fixtures/performance/perf_smoke/_support/PerfAutoloadSmoke.php';
    }
});

for ($i = 0; $i < 3; $i++) {
    $object = new PerfAutoloadSmoke();
    echo $object->message(), "\n";
}

for ($i = 0; $i < 3; $i++) {
    if (!class_exists('PerfAutoloadSmoke', false)) {
        echo "autoload-cache-miss\n";
    }
}
