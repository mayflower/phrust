<?php
class Box {
    public string $value;
}

function make(): Box {
    $box = new Box();
    $box->value = "ok";
    return $box;
}

$box = make();
echo $box->value, "\n";
