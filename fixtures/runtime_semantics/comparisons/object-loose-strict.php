<?php
class Box {
    public $value = 1;
}
$a = new Box();
$b = $a;
$c = clone $a;
echo ($a == $b) ? "1" : "0";
echo "|";
echo ($a === $b) ? "1" : "0";
echo "|";
echo ($a == $c) ? "1" : "0";
echo "|";
echo ($a === $c) ? "1" : "0";
echo "\n";
