<?php
class BaseBox {
    public $value;
    public function set($value) { $this->value = $value; }
    public function get() { return $this->value; }
}

class ChildBox extends BaseBox {
    public function label() { return 'child'; }
}

$box = new ChildBox();
$box->set(9);
echo $box->get(), '|', $box->label(), "\n";
