<?php
$stdlibFilesPath = __DIR__ . '/composer/autoload_files.php';
$stdlibFiles = require $stdlibFilesPath;
foreach ($stdlibFiles as $stdlibFile) {
    include_once $stdlibFile;
}

spl_autoload_register('stdlib_basic_project_autoload');

function stdlib_basic_project_autoload($class)
{
    $normalized = strtolower($class);
    if ($normalized === 'stdlib\\basicclassmap\\mappedthing') {
        $classmapFile = __DIR__ . '/../lib/MappedThing.php';
        include $classmapFile;
        return;
    }

    if ($normalized === 'stdlib\\basic\\psrgreeter') {
        $psr4File = __DIR__ . '/../src/PsrGreeter.php';
        include $psr4File;
    }
}
