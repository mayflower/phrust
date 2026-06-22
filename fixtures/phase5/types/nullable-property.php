<?php
class Box {
    public string|null $name;
}

$box = new Box();
$box->name = null;
echo ($box->name === null) ? "null" : "bad";
$box->name = "ok";
echo "|", $box->name, "\n";
