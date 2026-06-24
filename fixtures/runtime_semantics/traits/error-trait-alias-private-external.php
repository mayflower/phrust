<?php
// runtime-semantics: expect=fail
trait PrivateAliasTrait {
    public function run() { return 'run'; }
}

class PrivateAliasTraitBox {
    use PrivateAliasTrait {
        run as private hidden;
    }
}

echo (new PrivateAliasTraitBox())->hidden(), "\n";
