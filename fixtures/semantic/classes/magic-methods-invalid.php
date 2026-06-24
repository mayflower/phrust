<?php

class MagicInvalidFixture
{
    public static function __toString(): string
    {
        return '';
    }

    public function __call(string $name): mixed
    {
        return null;
    }
}
