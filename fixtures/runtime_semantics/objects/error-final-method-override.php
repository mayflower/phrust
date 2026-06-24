<?php
// runtime-semantics: expect=fail
class FinalMethodBase {
    final public function run(): string { return "base"; }
}

class FinalMethodChild extends FinalMethodBase {
    public function run(): string { return "child"; }
}

echo "unreachable";
