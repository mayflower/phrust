<?php
class ConstBase {
    public const LABEL = 'base';
    protected const HIDDEN = 'protected';
    public static function hidden() { return self::HIDDEN; }
}

class ConstChild extends ConstBase {
    public const LABEL = 'child';
}

echo ConstBase::LABEL, '|', ConstChild::LABEL, '|', ConstChild::hidden(), "\n";
