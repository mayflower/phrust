<?php
spl_autoload_register(function ($class) {
    include (__DIR__ . "/_data/Prompt40Included.php");
});
$object = new Prompt40Included();
echo gettype($object), "\n";
