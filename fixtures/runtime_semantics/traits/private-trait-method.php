<?php
trait PrivateTraitMethod {
    private function secret() { return 'secret'; }
    public function reveal() { return $this->secret(); }
}

class PrivateTraitMethodBox {
    use PrivateTraitMethod;
}

echo (new PrivateTraitMethodBox())->reveal(), "\n";
