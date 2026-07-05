<?php

class BootstrapConfig
{
    public const MODE = "prod";

    public static array $flags = ["cache" => true];

    public static function flag(string $key): bool
    {
        return self::$flags[$key] ?? false;
    }
}
