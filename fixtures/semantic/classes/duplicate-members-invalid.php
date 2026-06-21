<?php

class DuplicateMembers
{
    public function run(): void
    {
    }

    public function RUN(): void
    {
    }

    public string $name;
    private int $name;

    public const VALUE = 1;
    private const VALUE = 2;
}
