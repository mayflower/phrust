<?php
// runtime-semantics: category=magic expect=pass
class MagicStaticMissing {
    public static function __callStatic(string $name, array $args): string {
        return $name . ":" . $args[0];
    }
}

echo MagicStaticMissing::route("ok");
