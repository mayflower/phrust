<?php
spl_autoload_register(function ($class) {
    if (strtolower($class) === 'phase7autoloadsmoke') {
        require 'tests/fixtures/phase7/perf_smoke/_support/Phase7AutoloadSmoke.php';
    }
});

for ($i = 0; $i < 3; $i++) {
    $object = new Phase7AutoloadSmoke();
    echo $object->message(), "\n";
}

for ($i = 0; $i < 3; $i++) {
    if (!class_exists('Phase7AutoloadSmoke', false)) {
        echo "autoload-cache-miss\n";
    }
}
