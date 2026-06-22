<?php
// phase5: kind=pass id=trait-use
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
