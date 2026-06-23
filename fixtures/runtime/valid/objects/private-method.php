<?php
class Prompt27Secret {
    private function hidden() {
        return 1;
    }

    function call_hidden() {
        return $this->hidden();
    }
}

$secret = new Prompt27Secret();
echo $secret->call_hidden(), "\n";
