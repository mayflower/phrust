<?php
// runtime-semantics: category=pipe expect=pass
class WrapPipe {
    public function __invoke($value) {
        return $value . "!";
    }
}

$callable = new WrapPipe();
echo "x" |> $callable;
