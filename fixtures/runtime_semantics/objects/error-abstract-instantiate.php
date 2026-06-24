<?php
// runtime-semantics: expect=fail
abstract class Base {
    abstract public function run(): string;
}

new Base();
