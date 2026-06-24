<?php
trait ClassOverrideTrait {
    public function run() { return 'trait'; }
}

class ClassOverrideTraitBox {
    use ClassOverrideTrait;
    public function run() { return 'class'; }
}

echo (new ClassOverrideTraitBox())->run(), "\n";
