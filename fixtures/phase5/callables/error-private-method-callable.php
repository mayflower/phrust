<?php
// phase5-runtime: expect=fail
class Hidden {
    private function secret() {
        return "bad";
    }
}

$callable = [new Hidden(), "secret"];
echo $callable();
