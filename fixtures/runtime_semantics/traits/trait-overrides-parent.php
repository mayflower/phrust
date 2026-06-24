<?php
class TraitParentBox {
    public function run() { return 'parent'; }
}

trait ParentOverrideTrait {
    public function run() { return 'trait'; }
}

class TraitChildBox extends TraitParentBox {
    use ParentOverrideTrait;
}

echo (new TraitChildBox())->run(), "\n";
