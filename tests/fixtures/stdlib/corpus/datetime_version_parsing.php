<?php
// stdlib-diff: id=STDLIB_CORPUS_DATETIME_VERSION area=corpus expect=pass
// purpose: Composer/framework version checks plus deterministic ISO-like date parsing.
// reference-output:
// php-ok
// day=2026-06-22
echo version_compare(PHP_VERSION, '8.5.0', '>=') ? "php-ok\n" : "php-old\n";
$timestamp = 1782086400;
echo 'day=', date('Y-m-d', $timestamp), "\n";
