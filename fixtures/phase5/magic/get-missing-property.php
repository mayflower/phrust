<?php
class MagicGetMissing {
    public function __get(string $name): string {
        return "get:" . $name;
    }
}

echo (new MagicGetMissing())->missing;
