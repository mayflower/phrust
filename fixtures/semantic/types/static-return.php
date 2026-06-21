<?php

class StaticReturn
{
    public function make(): static
    {
        return $this;
    }
}
