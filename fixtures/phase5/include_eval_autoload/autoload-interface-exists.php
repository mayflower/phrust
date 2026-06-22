<?php
spl_autoload_register(function ($class) {
    if ($class === "Prompt40Interface") {
        include (__DIR__ . "/_data/Prompt40Interface.php");
    }
});
echo interface_exists("Prompt40Interface") ? "yes\n" : "no\n";
