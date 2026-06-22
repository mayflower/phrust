<?php
// phase5-runtime: category=foreach expect=pass
class Prompt42MutableProps
{
    public $a = 1;
    public $b = 2;
}
$object = new Prompt42MutableProps();
foreach ($object as $key => $value) {
    echo $key, ":", $value, ";";
    if ($key === "a") {
        $object->b = 9;
    }
}
echo "|", $object->b, "\n";
