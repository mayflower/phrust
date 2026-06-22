<?php
// phase5-runtime: category=magic expect=pass
class MagicCallPrivate {
    private function secret(): string {
        return "hidden";
    }

    public function __call(string $name, array $args): string {
        return "fallback:" . $name;
    }
}

$proxy = new MagicCallPrivate();
echo $proxy->secret();
