<?php
// runtime-semantics: category=magic expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_DEBUGINFO
class MagicDebugInfoGap {
    public function __debugInfo(): array {
        return ["visible" => 1];
    }
}

var_dump(new MagicDebugInfoGap());
