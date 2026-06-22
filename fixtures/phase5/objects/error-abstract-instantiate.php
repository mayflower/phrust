<?php
// phase5-runtime: expect=fail
abstract class Base {
    abstract public function run(): string;
}

new Base();
