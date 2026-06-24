<?php
// stdlib-diff: id=STDLIB_CORPUS_CONTAINER_AUTOLOAD area=corpus expect=pass
// purpose: Composer-like generated autoload include and service instantiation without vendoring framework code.
// reference-output:
// loaded
// service:CORE
require 'tests/fixtures/stdlib/corpus_support/ContainerService.php';

echo class_exists('Stdlib\\Corpus\\ContainerService', false) ? "loaded\n" : "missing\n";
$service = new Stdlib\Corpus\ContainerService();
echo $service->label('core'), "\n";
