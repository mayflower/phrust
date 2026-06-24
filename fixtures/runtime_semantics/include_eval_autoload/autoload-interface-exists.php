<?php
spl_autoload_register(function ($class) {
    if ($class === "AutoloadInterfaceFixture") {
        include (__DIR__ . "/_data/AutoloadInterfaceFixture.php");
    }
});
echo interface_exists("AutoloadInterfaceFixture") ? "yes\n" : "no\n";
