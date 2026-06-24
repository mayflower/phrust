<?php
// runtime-semantics: expect=fail
class LsbPrivateBase {
    public static function call() { return static::secret(); }
    private static function secret() { return 'base'; }
}

class LsbPrivateChild extends LsbPrivateBase {
    private static function secret() { return 'child'; }
}

echo LsbPrivateChild::call(), "\n";
