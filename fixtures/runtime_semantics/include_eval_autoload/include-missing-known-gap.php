<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT
echo "before|";
include (__DIR__ . "/_data/lib/missing.php");
echo "after\n";
