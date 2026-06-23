<?php
// phase6-diff: id=PHASE6_CORPUS_CONTAINER_AUTOLOAD area=corpus expect=pass
// purpose: Composer-like generated autoload include and service instantiation without vendoring framework code.
// reference-output:
// loaded
// service:CORE
require 'tests/fixtures/phase6/corpus_support/ContainerService.php';

echo class_exists('Phase6\\Corpus\\ContainerService', false) ? "loaded\n" : "missing\n";
$service = new Phase6\Corpus\ContainerService();
echo $service->label('core'), "\n";
