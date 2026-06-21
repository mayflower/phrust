<?php

trait RunnableAliasSource
{
    public function run(): void
    {
    }
}

class UsesTraitAlias
{
    use RunnableAliasSource {
        run as private runPrivately;
    }
}
