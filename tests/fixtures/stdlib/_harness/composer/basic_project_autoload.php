<?php
// stdlib-diff: id=STDLIB_COMPOSER_BASIC_PROJECT_AUTOLOAD area=composer expect=pass
ini_set('include_path', __DIR__ . '/../../composer/basic_project/vendor');
require 'autoload.php';

echo function_exists('stdlib_basic_file_helper') ? "files-loaded\n" : "files-missing\n";
$psr = new Stdlib\Basic\PsrGreeter();
echo $psr->message(), "\n";
$mapped = new Stdlib\BasicClassmap\MappedThing();
echo $mapped->label(), "\n";
echo class_exists('Stdlib\\Basic\\Missing', true) ? "bad\n" : "safe-missing\n";
