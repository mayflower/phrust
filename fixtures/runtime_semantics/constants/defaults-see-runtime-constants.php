<?php
class RuntimeConstantDefaultsFixture {
    public const VALUE = LATE_DEFINED_CONST;
    public static $items = [LATE_DEFINED_CONST => self::VALUE];
}

define("LATE_DEFINED_CONST", "ok");
echo RuntimeConstantDefaultsFixture::VALUE, "|", RuntimeConstantDefaultsFixture::$items["ok"], "\n";
