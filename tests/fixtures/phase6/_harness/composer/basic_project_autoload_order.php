<?php
// phase6-diff: id=PHASE6_COMPOSER_BASIC_PROJECT_AUTOLOAD_ORDER area=composer expect=pass
ini_set('include_path', __DIR__ . '/../../composer/basic_project/vendor');
include_once 'autoload.php';
include_once 'autoload.php';

echo function_exists('phase6_basic_file_helper') ? "files-first\n" : "files-missing\n";
echo phase6_basic_file_helper('order'), "\n";
$psr = new Phase6\Basic\PsrGreeter();
echo $psr->message(), "\n";
$mapped = new Phase6\BasicClassmap\MappedThing();
echo $mapped->label(), "\n";
echo count(spl_autoload_functions()), "\n";
echo class_exists('Phase6\\Basic\\Missing', true) ? "bad\n" : "safe-missing\n";
