<?php
// runtime-semantics: category=destructors expect=fail
class D {
    public function __destruct() {
        echo "destruct\n";
        throw new Exception("boom");
    }
}

new D();
echo "body\n";
