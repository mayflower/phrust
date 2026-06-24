<?php
// runtime-semantics: expect=fail
echo "before|";
require (__DIR__ . "/_data/lib/missing.php");
echo "after\n";
