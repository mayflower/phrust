<?php

trait InvalidAdaptationSource
{
    public function run(): void
    {
    }
}

class UsesInvalidAdaptation
{
    use InvalidAdaptationSource {
        run as;
    }
}
