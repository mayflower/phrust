<?php
trait PreferredTrait {
    public function run() { return 'preferred'; }
}

trait ReplacedTrait {
    public function run() { return 'replaced'; }
}

class InsteadOfTraitBox {
    use PreferredTrait, ReplacedTrait {
        PreferredTrait::run insteadof ReplacedTrait;
    }
}

echo (new InsteadOfTraitBox())->run(), "\n";
