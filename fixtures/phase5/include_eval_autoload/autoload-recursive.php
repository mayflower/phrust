<?php
spl_autoload_register(function ($class) {
    echo "load:", $class, "\n";
    class_exists($class);
});
echo class_exists("Prompt40Missing") ? "yes\n" : "no\n";
