<?php
class PrivateBase {
    private function value() { return 'base'; }
    public function callBase() { return $this->value(); }
}

class PrivateChild extends PrivateBase {
    private function value() { return 'child'; }
    public function callChild() { return $this->value(); }
}

$object = new PrivateChild();
echo $object->callBase(), '|', $object->callChild(), "\n";
