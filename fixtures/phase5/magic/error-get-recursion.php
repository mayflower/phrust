<?php
// phase5-runtime: expect=fail
class MagicGetRecursion {
    public function __get(string $name): mixed {
        return $this->$name;
    }
}

echo (new MagicGetRecursion())->missing;
