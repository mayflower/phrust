<?php
// phase5-runtime: expect=fail
interface Needed {
    public function run(): string;
}

class Missing implements Needed {}

echo "unreachable";
