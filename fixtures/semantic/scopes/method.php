<?php
namespace App\Scopes;

class Worker
{
    public function run(string $job): void
    {
        $callback = fn($value) => $value;
    }
}
