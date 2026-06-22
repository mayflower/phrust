<?php
function prompt40_loader($class) {
    include (__DIR__ . "/_data/Prompt40Loaded.php");
}
spl_autoload_register("prompt40_loader");
$object = new Prompt40Loaded();
echo gettype($object), "\n";
