<?php

class SelfParentBase
{
    public static function make(): static
    {
        return new static();
    }

    public function ping(): void
    {
    }
}

class SelfParentChild extends SelfParentBase
{
    public function ping(): void
    {
        self::make();
        static::make();
        parent::ping();
    }
}
