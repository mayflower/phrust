<?php

class StaticClosureThis
{
    public function make(): Closure
    {
        return static function (): void {
            $this;
        };
    }
}
