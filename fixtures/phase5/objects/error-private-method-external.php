<?php
// phase5-runtime: expect=fail
class HiddenMethod {
    private function value() { return 1; }
}

(new HiddenMethod())->value();
