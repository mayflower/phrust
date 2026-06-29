<?php
// runtime-semantics: category=wordpress_blockers expect=pass
#[Attribute]
class Flag
{
    public function __construct(public int $value) {}
}

#[Flag(PHP_INT_MAX)]
class BootDefaults
{
    public const MASK = E_ALL & ~E_DEPRECATED;
    public const ROOT = DIRECTORY_SEPARATOR . "wp";
    public string $line = PHP_EOL;
}

function defaults($limit = PHP_INT_MAX, $path = DEFAULT_INCLUDE_PATH)
{
    echo $limit === PHP_INT_MAX ? "limit|" : "bad|";
    echo is_string($path) ? "path|" : "bad|";
}

defaults();
echo BootDefaults::MASK, "|", BootDefaults::ROOT, "|", PHP_EOL;
