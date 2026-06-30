<?php
class RefBox {
    public $value = 1;
}

function bump_object_ref(&$value) {
    $value++;
}

$box = new RefBox();
bump_object_ref($box->value);
echo $box->value;
