<?php

abstract class MethodInvalid
{
    abstract final public function both(): void;

    abstract private function hidden(): void;

    readonly public function wrong(): void
    {
    }
}
