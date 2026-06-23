<?php
// phase6-diff: id=PHASE6_COMPOSER_PROJECT_AUTOLOAD area=composer expect=pass
ini_set(
    'include_path',
    __DIR__ . '/../../composer/project/vendor' . ':' . __DIR__ . '/../../composer/project/src'
);
require 'autoload.php';

echo class_exists('Phase6\\ComposerProject\\App\\Greeter', true) ? "loaded\n" : "missing\n";
echo class_exists('Phase6\\ComposerProject\\App\\Missing', true) ? "bad\n" : "safe-missing\n";

$greeter = new Phase6\ComposerProject\App\Greeter();
echo $greeter->message(), "\n";
