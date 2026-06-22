<?php
// phase5-runtime: category=destructors expect=pass
class D {
    public function __destruct() {
        echo "destruct\n";
    }
}

$d = new D();
echo "body\n";
