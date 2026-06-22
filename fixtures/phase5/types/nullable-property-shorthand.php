<?php
class Box {
    public ?string $name;
}

$box = new Box();
$box->name = null;
echo ($box->name === null) ? "null\n" : "bad\n";
