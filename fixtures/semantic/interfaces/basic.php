<?php

namespace App\Contracts;

interface Runnable extends BaseRunnable, NamedRunnable
{
    public function run(): void;
}
