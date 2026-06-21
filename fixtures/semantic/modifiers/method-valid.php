<?php

abstract class MethodValid
{
    abstract protected function compute(): int;

    final public static function named(): string
    {
        return "ok";
    }

    public function &reference(): array
    {
        static $value = [];
        return $value;
    }
}
