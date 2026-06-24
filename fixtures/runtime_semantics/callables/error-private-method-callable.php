<?php
// runtime-semantics: expect=fail
class Hidden {
    private function secret() {
        return "bad";
    }
}

$callable = [new Hidden(), "secret"];
echo $callable();
