<?php
include __DIR__ . "/_data/runtime-default-source.php";

class IncludeRuntimeConstantDefaultsFixture {
    public const VALUE = INCLUDED_RUNTIME_DEFAULT_CONST;
    public static $items = [INCLUDED_RUNTIME_DEFAULT_CONST => self::VALUE];
}

echo IncludeRuntimeConstantDefaultsFixture::VALUE, "|", IncludeRuntimeConstantDefaultsFixture::$items["included"], "\n";
