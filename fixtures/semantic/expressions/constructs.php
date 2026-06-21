<?php

namespace Exprs;

$path = __DIR__ . "/not-loaded.php";
$included = include $path;
$required = require_once $path;
$evaluated = eval("return 1;");
$printed = print "value";
exit($printed);
