<?php

abstract class AbstractBase
{
}

final class FinalConcrete
{
}

readonly class ReadonlyValue
{
    public function __construct(public int $id)
    {
    }
}
