<?php

trait LogsActivity
{
    public function log(): void
    {
    }
}

class UsesTrait
{
    use LogsActivity;
}
