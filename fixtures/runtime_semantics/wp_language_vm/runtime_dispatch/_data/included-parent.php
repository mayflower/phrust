<?php
class IncludedParent
{
    public static function label()
    {
        return static::class . "|" . self::class;
    }
}
