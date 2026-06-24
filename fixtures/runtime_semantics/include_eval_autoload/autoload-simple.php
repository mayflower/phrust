<?php
function autoload_fixture_loader($class) {
    include (__DIR__ . "/_data/AutoloadLoadedFixture.php");
}
spl_autoload_register("autoload_fixture_loader");
$object = new AutoloadLoadedFixture();
echo gettype($object), "\n";
