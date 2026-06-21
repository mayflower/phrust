<?php

trait FirstRunTrait
{
    public function run(): void
    {
    }
}

trait SecondRunTrait
{
    public function run(): void
    {
    }
}

class UsesTraitPrecedence
{
    use FirstRunTrait, SecondRunTrait {
        FirstRunTrait::run insteadof SecondRunTrait;
        SecondRunTrait::run as runFromSecond;
    }
}
