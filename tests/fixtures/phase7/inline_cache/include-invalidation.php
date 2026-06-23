<?php
for ($i = 0; $i < 12; $i++) {
    echo strlen("x");
    if ($i === 5) {
        include_once "define-function.php";
    }
}

echo function_exists("phase7_ic_include_defined_fn") ? ":yes\n" : ":no\n";
