<?php
class Cell {
    public $value;
}

$left = new Cell();
$right = new Cell();
$left->value = 1;
$right->value = 2;
echo $left->value, "|", $right->value, "\n";
