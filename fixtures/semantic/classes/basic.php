<?php

namespace App\Model;

#[Entity]
final class User
{
    public const KIND = 'user';

    public string $name;

    public function name(): string
    {
        return $this->name;
    }
}
