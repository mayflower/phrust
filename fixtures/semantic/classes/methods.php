<?php

abstract class MethodShapes
{
    public function run(int $count): void
    {
    }

    abstract protected function required(): string;

    private static function helper(): int
    {
        return 1;
    }
}
