<?php
include __DIR__ . "/_data/child.php";
include_once __DIR__ . "/_data/once.php";
require __DIR__ . "/_data/required.php";
require_once __DIR__ . "/_data/required-once.php";
$loaded = include_once (__DIR__ . "/_data/assigned-once.php");
$required = require (__DIR__ . "/_data/required-parenthesized.php");
print "done" . "\n";
