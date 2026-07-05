<?php

spl_autoload_register(function (string $class): void {
    $path = __DIR__ . '/_data/bootstrap_ic/' . $class . '.php';
    if (is_file($path)) {
        require $path;
    }
});

$hits = 0;
for ($i = 0; $i < 30; $i++) {
    require_once __DIR__ . '/_data/bootstrap_ic/BootstrapConfig.php';
    if (BootstrapConfig::MODE === 'prod' && BootstrapConfig::flag('cache')) {
        $hits++;
    }
    // The reference engine re-invokes autoloaders for every unknown-class
    // probe; repeated misses must not be negatively cached.
    $missing = class_exists('BootstrapNope');
}
echo $hits, ':', BootstrapConfig::MODE, ':', var_export($missing, true), "\n";
var_dump(BootstrapConfig::$flags);
