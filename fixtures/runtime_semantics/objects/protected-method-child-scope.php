<?php
class ProtectedBase {
    protected function value() { return 'base'; }
}

class ProtectedChild extends ProtectedBase {
    public function call() { return $this->value(); }
}

echo (new ProtectedChild())->call(), "\n";
