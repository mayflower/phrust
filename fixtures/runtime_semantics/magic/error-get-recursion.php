<?php
// runtime-semantics: expect=fail
class MagicGetRecursion {
    public function __get(string $name): mixed {
        return $this->$name;
    }
}

echo (new MagicGetRecursion())->missing;
