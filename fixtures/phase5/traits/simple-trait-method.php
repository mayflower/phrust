<?php
trait SimpleTrait {
    public function value() { return 'trait'; }
}

class SimpleTraitBox {
    use SimpleTrait;
}

echo (new SimpleTraitBox())->value(), "\n";
