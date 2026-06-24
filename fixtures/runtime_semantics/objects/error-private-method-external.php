<?php
// runtime-semantics: expect=fail
class HiddenMethod {
    private function value() { return 1; }
}

(new HiddenMethod())->value();
