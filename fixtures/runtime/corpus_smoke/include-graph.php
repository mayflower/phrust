<?php
// phase4-runtime: corpus=pass
$root = "root";
$settings = include (__DIR__ . "/lib/settings.php");
include_once (__DIR__ . "/lib/routes.php");
include_once (__DIR__ . "/lib/routes.php");

echo $root, "|", $settings["mode"], "|", $route, "\n";
