<?php
class LsbBase {
    public static function who() { return static::class; }
}

class LsbChild extends LsbBase {
    public static function test() {
        return self::who() . '|' . parent::who() . '|' . LsbBase::who() . '|' . static::who();
    }
}

class LsbGrandChild extends LsbChild {
}

echo LsbGrandChild::test(), "\n";
