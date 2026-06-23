<?php
// phase6-diff: id=PHASE6_COMPOSER_BASIC_PROJECT_AUTOLOAD area=composer expect=pass
ini_set('include_path', __DIR__ . '/../../composer/basic_project/vendor');
require 'autoload.php';

echo function_exists('phase6_basic_file_helper') ? "files-loaded\n" : "files-missing\n";
$psr = new Phase6\Basic\PsrGreeter();
echo $psr->message(), "\n";
$mapped = new Phase6\BasicClassmap\MappedThing();
echo $mapped->label(), "\n";
echo class_exists('Phase6\\Basic\\Missing', true) ? "bad\n" : "safe-missing\n";
