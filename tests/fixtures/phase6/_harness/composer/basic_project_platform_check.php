<?php
// phase6-diff: id=PHASE6_COMPOSER_PLATFORM_CHECK area=composer expect=pass
ini_set('include_path', 'tests/fixtures/phase6/composer/basic_project/vendor/composer');

$result = require 'platform_check.php';
echo $result ? "return-ok\n" : "return-fail\n";

$extensions = get_loaded_extensions();
echo in_array('json', $extensions, true) ? "json-loaded\n" : "json-missing\n";
echo in_array('mbstring', $extensions, true) ? "bad-mbstring\n" : "mbstring-missing\n";
echo function_exists('version_compare') ? "version-compare-exists\n" : "version-compare-missing\n";
