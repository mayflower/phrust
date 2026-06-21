<?php

class AsymmetricVisibility
{
    public private(set) string $writeInside;

    protected private(set) string $writePrivate;

    private public(set) string $invalidOrder;

    private(set) string $missingGetterVisibility;

    public function __construct(public private(set) string $promoted)
    {
    }
}
