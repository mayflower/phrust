<?php
trait FirstTrait {
    public function first() { return 'first'; }
}

trait SecondTrait {
    public function second() { return 'second'; }
}

class MultipleTraitBox {
    use FirstTrait, SecondTrait;
}

$box = new MultipleTraitBox();
echo $box->first(), '|', $box->second(), "\n";
