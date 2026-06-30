<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT
$s = "abc";
$s[-5] = "Q";
echo $s, "\n";

try {
    $s["name"] = "Q";
} catch (Throwable $error) {
    echo get_class($error), ":", $error->getMessage(), "\n";
}
