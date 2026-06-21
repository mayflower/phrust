<?php
// phase4: kind=known_gap id=E_PHP_IR_UNSUPPORTED_TRAIT_RUNTIME
trait Phase4TraitGap
{
    public function label(): string
    {
        return "trait";
    }
}

class Phase4UsesTraitGap
{
    use Phase4TraitGap;
}

echo (new Phase4UsesTraitGap())->label();
