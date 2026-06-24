<?php
$a = [0 => "1", "name" => 2];
$b = ["name" => 2, 0 => 1];
$c = [0 => "1", "name" => 2];
echo ($a == $b) ? "1" : "0";
echo "|";
echo ($a === $b) ? "1" : "0";
echo "|";
echo ($a === $c) ? "1" : "0";
echo "\n";
