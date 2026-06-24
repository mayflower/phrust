<?php
class PrivateMethodFixture {
    private function hidden() {
        return 1;
    }

    function call_hidden() {
        return $this->hidden();
    }
}

$secret = new PrivateMethodFixture();
echo $secret->call_hidden(), "\n";
