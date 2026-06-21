<?php
function greet($name = "world", $punct = "!")
{
    echo "hi ", $name, $punct;
}

greet();
echo "|";
greet("php", "?");
echo "\n";
