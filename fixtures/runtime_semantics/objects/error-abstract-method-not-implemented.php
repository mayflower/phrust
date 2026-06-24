<?php
// runtime-semantics: expect=fail
abstract class BaseMissing {
    abstract public function run(): string;
}

class ChildMissing extends BaseMissing {}

echo "unreachable";
