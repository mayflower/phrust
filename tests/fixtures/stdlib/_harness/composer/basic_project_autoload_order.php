<?php
// stdlib-diff: id=STDLIB_COMPOSER_BASIC_PROJECT_AUTOLOAD_ORDER area=composer expect=pass
ini_set('include_path', __DIR__ . '/../../composer/basic_project/vendor');
include_once 'autoload.php';
include_once 'autoload.php';

echo function_exists('stdlib_basic_file_helper') ? "files-first\n" : "files-missing\n";
echo stdlib_basic_file_helper('order'), "\n";
$psr = new Stdlib\Basic\PsrGreeter();
echo $psr->message(), "\n";
$mapped = new Stdlib\BasicClassmap\MappedThing();
echo $mapped->label(), "\n";
echo count(spl_autoload_functions()), "\n";
echo class_exists('Stdlib\\Basic\\Missing', true) ? "bad\n" : "safe-missing\n";
