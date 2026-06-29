<?php
// runtime-semantics: category=wordpress_blockers expect=pass
$count = 0;
include_once __DIR__ . "/_data/once.php";
include_once __DIR__ . "/_data/./once.php";
require_once __DIR__ . "/_data/nested/../once.php";
echo $count, "\n";
