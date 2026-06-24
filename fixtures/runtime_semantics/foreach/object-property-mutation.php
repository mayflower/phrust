<?php
// runtime-semantics: category=foreach expect=pass
class MutablePropsFixture
{
    public $a = 1;
    public $b = 2;
}
$object = new MutablePropsFixture();
foreach ($object as $key => $value) {
    echo $key, ":", $value, ";";
    if ($key === "a") {
        $object->b = 9;
    }
}
echo "|", $object->b, "\n";
