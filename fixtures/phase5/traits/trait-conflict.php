<?php
// phase5-runtime: expect=fail
trait ConflictFirstTrait {
    public function run() { return 'first'; }
}

trait ConflictSecondTrait {
    public function run() { return 'second'; }
}

class ConflictTraitBox {
    use ConflictFirstTrait, ConflictSecondTrait;
}

echo (new ConflictTraitBox())->run(), "\n";
