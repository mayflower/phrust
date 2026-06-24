<?php

class StaticThisFixture
{
    public static function deferred(): mixed
    {
        return $this;
    }
}
