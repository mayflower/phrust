<?php
$phase6FilesPath = __DIR__ . '/composer/autoload_files.php';
$phase6Files = require $phase6FilesPath;
foreach ($phase6Files as $phase6File) {
    include_once $phase6File;
}

spl_autoload_register('phase6_basic_project_autoload');

function phase6_basic_project_autoload($class)
{
    $normalized = strtolower($class);
    if ($normalized === 'phase6\\basicclassmap\\mappedthing') {
        $classmapFile = __DIR__ . '/../lib/MappedThing.php';
        include $classmapFile;
        return;
    }

    if ($normalized === 'phase6\\basic\\psrgreeter') {
        $psr4File = __DIR__ . '/../src/PsrGreeter.php';
        include $psr4File;
    }
}
