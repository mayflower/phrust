<?php
class PropertyDefaults {
    public $name = 'box';
    public int $count;
}
$box = new PropertyDefaults();
$box->count = 3;
echo $box->name, '|', $box->count, "\n";
