<?php
class MagicGetPrivate {
    private string $secret = "hidden";

    public function __get(string $name): string {
        return "magic:" . $name;
    }
}

echo (new MagicGetPrivate())->secret;
