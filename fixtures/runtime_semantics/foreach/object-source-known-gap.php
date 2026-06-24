<?php
// runtime-semantics: category=foreach expect=pass
class PublicPropsFixture
{
    public $a = 1;
    public $b = 2;
    private $hidden = 9;
}
foreach (new PublicPropsFixture() as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
