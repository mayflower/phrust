<?php
// runtime-semantics: category=magic expect=pass
class MagicCallMissing {
    public function __call(string $name, array $args): string {
        return $name . ":" . $args[0] . ":" . $args[1];
    }
}

$proxy = new MagicCallMissing();
echo $proxy->combine("A", "B");
