<?php

class Prompt26Base
{
    public static function make(): static
    {
        return new static();
    }

    public function ping(): void
    {
    }
}

class Prompt26Child extends Prompt26Base
{
    public function ping(): void
    {
        self::make();
        static::make();
        parent::ping();
    }
}
