<?php
// runtime-fixture: expect=known_gap known_gap=E_PHP_VM_INCLUDE_MISSING
echo "before|";
include (__DIR__ . "/lib/missing.php");
echo "after\n";
