<?php

namespace App\Types;

use Vendor\Model;

class TypedProperties
{
    public int $id;
    public ?string $name;
    protected Model $model;
    private array $items = [];
}
