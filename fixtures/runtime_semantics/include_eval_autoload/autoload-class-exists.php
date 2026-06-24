<?php
spl_autoload_register(function ($class) {
    if ($class === "AutoloadExistsFixture") {
        include (__DIR__ . "/_data/AutoloadExistsFixture.php");
    }
});
echo class_exists("AutoloadExistsFixture") ? "yes\n" : "no\n";
