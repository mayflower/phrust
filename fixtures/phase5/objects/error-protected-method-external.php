<?php
// phase5-runtime: expect=fail
class HiddenProtectedMethod {
    protected function value() { return 1; }
}

(new HiddenProtectedMethod())->value();
