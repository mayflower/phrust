<?php
class ClassNameBase {
    public static function names() { return self::class . '|' . static::class; }
}

class ClassNameChild extends ClassNameBase {
}

echo ClassNameBase::class, '|', ClassNameChild::names(), "\n";
