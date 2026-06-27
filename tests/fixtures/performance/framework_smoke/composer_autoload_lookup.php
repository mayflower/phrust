<?php
$classMap = array(
    'perfframeworkautoloadservice' => __DIR__ . '/_support/PerfFrameworkAutoloadService.php',
);

spl_autoload_register(function ($class) use ($classMap) {
    $key = strtolower($class);
    if (isset($classMap[$key])) {
        require $classMap[$key];
    }
});

class_exists('PerfFrameworkAutoloadService', true);

for ($i = 0; $i < 3; $i++) {
    $service = new PerfFrameworkAutoloadService('auto');
    echo $service->handle('LOOKUP'), "\n";
}

echo class_exists('PerfFrameworkMissingService', true) ? "missing-loaded\n" : "missing-safe\n";
