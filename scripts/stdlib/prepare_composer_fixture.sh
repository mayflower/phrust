#!/usr/bin/env bash
set -euo pipefail

root="tests/fixtures/stdlib/composer/basic_project"
mkdir -p "$root/src" "$root/lib" "$root/files" "$root/vendor/composer"

cat > "$root/composer.json" <<'JSON'
{
  "name": "stdlib/basic-project",
  "autoload": {
    "psr-4": {
      "Stdlib\\Basic\\": "src/"
    },
    "classmap": [
      "lib/MappedThing.php"
    ],
    "files": [
      "files/helpers.php"
    ]
  }
}
JSON

cat > "$root/src/PsrGreeter.php" <<'PHP'
<?php
namespace Stdlib\Basic;

class PsrGreeter
{
    public function message()
    {
        return \stdlib_basic_file_helper('psr4');
    }
}
PHP

cat > "$root/lib/MappedThing.php" <<'PHP'
<?php
namespace Stdlib\BasicClassmap;

class MappedThing
{
    public function label()
    {
        return \stdlib_basic_file_helper('classmap');
    }
}
PHP

cat > "$root/files/helpers.php" <<'PHP'
<?php
function stdlib_basic_file_helper($value)
{
    return 'file-' . $value;
}
PHP

cat > "$root/vendor/composer/autoload_psr4.php" <<'PHP'
<?php
return [
    'Stdlib\\Basic\\' => [
        __DIR__ . '/../../src',
    ],
];
PHP

cat > "$root/vendor/composer/autoload_classmap.php" <<'PHP'
<?php
return [
    'Stdlib\\BasicClassmap\\MappedThing' => __DIR__ . '/../../lib/MappedThing.php',
];
PHP

cat > "$root/vendor/composer/autoload_files.php" <<'PHP'
<?php
return [
    'stdlib_basic_file_helper' => __DIR__ . '/../../files/helpers.php',
];
PHP

cat > "$root/vendor/composer/platform_check.php" <<'PHP'
<?php

$issues = array();

if (!defined('PHP_VERSION_ID')) {
    $issues[] = 'missing PHP_VERSION_ID';
}

if (PHP_VERSION_ID < 80500) {
    $issues[] = 'php version';
}

if (!version_compare(PHP_VERSION, '8.5.0', '>=')) {
    $issues[] = 'version compare';
}

if (!extension_loaded('json')) {
    $issues[] = 'json extension';
}

if (extension_loaded('mbstring')) {
    $issues[] = 'unexpected mbstring extension';
}

if (!function_exists('json_encode')) {
    $issues[] = 'json_encode';
}

if (!class_exists('JsonException', false)) {
    $issues[] = 'JsonException';
}

if (ini_get('default_charset') !== 'UTF-8') {
    $issues[] = 'default_charset';
}

if (constant('PHP_VERSION_ID') !== PHP_VERSION_ID) {
    $issues[] = 'constant PHP_VERSION_ID';
}

if (count($issues) > 0) {
    echo 'platform-fail:', implode(',', $issues), "\n";
    return false;
}

echo "platform-ok\n";
return true;
PHP

cat > "$root/vendor/autoload.php" <<'PHP'
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
PHP

printf '%s\n' "[ok] prepared $root"
