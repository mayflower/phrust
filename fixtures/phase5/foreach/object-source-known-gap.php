<?php
// phase5-runtime: category=foreach expect=pass
class Prompt42PublicProps
{
    public $a = 1;
    public $b = 2;
    private $hidden = 9;
}
foreach (new Prompt42PublicProps() as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
