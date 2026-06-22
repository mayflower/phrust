<?php
class LsbInstanceBase {
    public function who() { return static::class; }
}

class LsbInstanceChild extends LsbInstanceBase {
}

echo (new LsbInstanceChild())->who(), "\n";
