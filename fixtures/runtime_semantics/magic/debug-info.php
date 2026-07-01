<?php
// runtime-semantics: category=magic expect=pass
class MagicDebugInfoGap {
    public function __debugInfo(): array {
        return ["visible" => 1];
    }
}

var_dump(new MagicDebugInfoGap());
