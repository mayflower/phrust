<?php
class StaticBase {
    public static function value() { return 'base'; }
}

class StaticChild extends StaticBase {
    public static function call() { return self::value() . '|' . parent::value(); }
}

echo StaticChild::call(), "\n";
