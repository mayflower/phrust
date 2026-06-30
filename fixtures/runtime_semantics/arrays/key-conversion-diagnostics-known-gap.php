<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_ARRAY_KEY_CONVERSION_EDGE_CASES
foreach ([null, 1.5, [], new stdClass()] as $key) {
    try {
        $array = [];
        $array[$key] = "value";
        foreach ($array as $actual => $value) {
            echo gettype($actual), ":", $actual, "=", $value, ";";
        }
        echo "\n";
    } catch (Throwable $error) {
        echo get_class($error), ":", $error->getMessage(), "\n";
    }
}
