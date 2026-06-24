<?php
spl_autoload_register(function ($class) {
    include (__DIR__ . "/_data/AutoloadIncludedFixture.php");
});
$object = new AutoloadIncludedFixture();
echo gettype($object), "\n";
