<?php
for ($i = 0; $i < 3; $i++) {
    include "include-path-cache-lib/value.php";
}

for ($i = 0; $i < 2; $i++) {
    include_once "include-path-cache-lib/once.php";
}

echo "\n";
