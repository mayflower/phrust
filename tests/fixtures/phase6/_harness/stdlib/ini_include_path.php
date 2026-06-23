<?php
// phase6-diff: id=PHASE6_STDLIB_INI_INCLUDE_PATH area=stdlib expect=pass
ini_set('include_path', __DIR__ . '/ini_path_lib');
$value = include 'config.inc';
echo "return:", $value, "\n";
