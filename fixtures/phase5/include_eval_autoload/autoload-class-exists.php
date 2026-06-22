<?php
spl_autoload_register(function ($class) {
    if ($class === "Prompt40Exists") {
        include (__DIR__ . "/_data/Prompt40Exists.php");
    }
});
echo class_exists("Prompt40Exists") ? "yes\n" : "no\n";
