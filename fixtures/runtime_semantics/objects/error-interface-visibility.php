<?php
// runtime-semantics: expect=fail
interface PublicOnly {
    public function run(): string;
}

class Hidden implements PublicOnly {
    protected function run(): string { return "hidden"; }
}

echo "unreachable";
