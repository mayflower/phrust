<?php
// runtime-fixture: kind=pass id=trait-use
trait RuntimeTraitGap
{
    public function label(): string
    {
        return "trait";
    }
}

class RuntimeUsesTraitGap
{
    use RuntimeTraitGap;
}

echo (new RuntimeUsesTraitGap())->label();
