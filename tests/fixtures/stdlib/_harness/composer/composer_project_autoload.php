<?php
// stdlib-diff: id=STDLIB_COMPOSER_PROJECT_AUTOLOAD area=composer expect=pass
ini_set(
    'include_path',
    __DIR__ . '/../../composer/project/vendor' . ':' . __DIR__ . '/../../composer/project/src'
);
require 'autoload.php';

echo class_exists('Stdlib\\ComposerProject\\App\\Greeter', true) ? "loaded\n" : "missing\n";
echo class_exists('Stdlib\\ComposerProject\\App\\Missing', true) ? "bad\n" : "safe-missing\n";

$greeter = new Stdlib\ComposerProject\App\Greeter();
echo $greeter->message(), "\n";
